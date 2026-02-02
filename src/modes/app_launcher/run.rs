//! Application launcher mode

use crate::cli::Opts;
use eyre::{eyre, Result, WrapErr};

use crate::ui::{InputConfig, InputEvent as Event, UI};

use crate::core::state::{sort_by_frecency, Message, State};
use directories::ProjectDirs;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use redb::ReadableTable;
use scopeguard::defer;
use std::collections::BTreeSet;
use std::time::Duration;
use std::{env, fs, io, path};

use crossterm::event::{MouseButton, MouseEventKind};

/// Run application launcher mode
pub async fn run(cli: Opts) -> Result<()> {
    use crossterm::event::KeyCode;

    // Handle direct launch mode (bypass TUI)
    // Require at least 2 characters, otherwise just launch TUI
    if let Some(ref program_name) = cli.program {
        if program_name.len() >= 2 {
            return super::search::launch_program_directly(&cli, program_name);
        }
        // Less than 2 characters, ignore and continue to TUI
    }

    crate::setup_terminal(cli.disable_mouse)?;
    defer! {
        let _ = crate::shutdown_terminal(cli.disable_mouse);
    }
    let db: std::sync::Arc<redb::Database>;
    let lock_path: path::PathBuf;

    // Open redb database
    if let Some(project_dirs) = ProjectDirs::from("ch", "forkbomb9", env!("CARGO_PKG_NAME")) {
        let data_dir = project_dirs.data_local_dir().to_path_buf();

        if !data_dir.exists() {
            // Create dir if it doesn't exist
            if let Err(error) = fs::create_dir_all(&data_dir) {
                return Err(eyre!("Failed to create data directory: {}", error));
            }
        }

        let hist_db_file = data_dir.join("hist_db.redb");

        // Check if Fsel is already running (mode-specific lock file) - BEFORE opening database
        {
            let mut lock = data_dir.clone();
            lock.push("fsel-fsel.lock");
            lock_path = lock;
            let contents = match fs::read_to_string(&lock_path) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
                Ok(c) => c,
                Err(e) => return Err(e).wrap_err("Failed to read lockfile"),
            };

            if !contents.is_empty() {
                if cli.replace {
                    let mut target_pids: BTreeSet<i32> = BTreeSet::new();

                    if let Ok(pid) = contents.parse::<i32>() {
                        target_pids.insert(pid);
                    }

                    if let Ok(holders) = crate::find_processes_holding_file(&hist_db_file) {
                        target_pids.extend(holders);
                    }

                    for pid in target_pids.clone() {
                        if let Err(e) = crate::process::kill_process_sigterm_result(pid) {
                            if e.raw_os_error() != Some(libc::ESRCH) {
                                return Err(eyre!("Failed to kill process {}: {}", pid, e));
                                // Log or handle error, but don't necessarily exit
                            }
                        }

                        const CHECK_INTERVAL_MS: u64 = 5;
                        const TOTAL_WAIT_MS: u64 = 30;
                        let mut waited_ms = 0u64;
                        let mut escalated = false;

                        loop {
                            #[allow(unsafe_code)]
                            let still_running = unsafe { libc::kill(pid, 0) == 0 };

                            if !still_running {
                                break;
                            }

                            if !escalated {
                                #[allow(unsafe_code)]
                                unsafe {
                                    let _ = libc::kill(pid, libc::SIGKILL);
                                }
                                escalated = true;
                            }

                            if waited_ms >= TOTAL_WAIT_MS {
                                return Err(eyre::eyre!(
                                    "Existing fsel instance (pid {pid}) refused to exit"
                                ));
                            }

                            std::thread::sleep(std::time::Duration::from_millis(CHECK_INTERVAL_MS));
                            waited_ms += CHECK_INTERVAL_MS;
                        }
                    }

                    if let Ok(mut remaining) = crate::find_processes_holding_file(&hist_db_file) {
                        remaining.retain(|pid| !target_pids.contains(pid));

                        if !remaining.is_empty() {
                            return Err(eyre::eyre!(
                                "Existing fsel instance (pid(s) {:?}) refused to exit",
                                remaining
                            ));
                        }
                    }
                } else {
                    return Err(eyre!("Fsel is already running"));
                }
            } else if cli.replace {
                if let Ok(holders) = crate::find_processes_holding_file(&hist_db_file) {
                    if !holders.is_empty() {
                        for pid in holders.clone() {
                            if let Err(e) = crate::process::kill_process_sigterm_result(pid) {
                                if e.raw_os_error() != Some(libc::ESRCH) {
                                    return Err(eyre!("Failed to kill process {}: {}", pid, e));
                                    // Log or handle error, but don't necessarily exit
                                }
                            }

                            const CHECK_INTERVAL_MS: u64 = 5;
                            const TOTAL_WAIT_MS: u64 = 30;
                            let mut waited_ms = 0u64;
                            let mut escalated = false;

                            loop {
                                #[allow(unsafe_code)]
                                let still_running = unsafe { libc::kill(pid, 0) == 0 };

                                if !still_running {
                                    break;
                                }

                                if !escalated {
                                    #[allow(unsafe_code)]
                                    unsafe {
                                        let _ = libc::kill(pid, libc::SIGKILL);
                                    }
                                    escalated = true;
                                }

                                if waited_ms >= TOTAL_WAIT_MS {
                                    return Err(eyre::eyre!(
                                        "Existing fsel instance (pid {pid}) refused to exit"
                                    ));
                                }

                                std::thread::sleep(std::time::Duration::from_millis(
                                    CHECK_INTERVAL_MS,
                                ));
                                waited_ms += CHECK_INTERVAL_MS;
                            }
                        }

                        if let Ok(final_holders) = crate::find_processes_holding_file(&hist_db_file)
                        {
                            if !final_holders.is_empty() {
                                return Err(eyre::eyre!(
                                    "Existing fsel instance (pid(s) {:?}) refused to exit",
                                    final_holders
                                ));
                            }
                        }
                    }
                }
            }

            if let Err(err) = fs::remove_file(&lock_path) {
                if err.kind() != io::ErrorKind::NotFound {
                    return Err(err).wrap_err("Failed to remove existing lockfile");
                }
            }

            let mut lock_file = fs::File::create(&lock_path)?;
            let pid;
            #[allow(unsafe_code)]
            unsafe {
                pid = libc::getpid();
            }
            use std::io::Write;
            lock_file.write_all(pid.to_string().as_bytes())?;
        }

        // Lock file cleanup guard
        struct LockGuard(path::PathBuf);
        impl Drop for LockGuard {
            fn drop(&mut self) {
                let _ = fs::remove_file(&self.0);
            }
        }
        let _lock_guard = LockGuard(lock_path.clone());

        let mut db_instance = redb::Database::create(&hist_db_file);
        if let Err(err) = &db_instance {
            if cli.replace && err.to_string().contains("Cannot acquire lock") {
                std::thread::sleep(std::time::Duration::from_millis(15));
                db_instance = redb::Database::create(&hist_db_file);
            }
        }

        let db_instance = db_instance
            .wrap_err_with(|| format!("Failed to open database at {:?}", hist_db_file))?;

        db = std::sync::Arc::new(db_instance);

        if cli.clear_history {
            // Clear all tables in redb
            const HISTORY_TABLE: redb::TableDefinition<&str, u64> =
                redb::TableDefinition::new("history");
            const PINNED_TABLE: redb::TableDefinition<&str, &[u8]> =
                redb::TableDefinition::new("pinned_apps");

            let write_txn = db.begin_write().wrap_err("Error starting transaction")?;
            {
                let mut history_table = write_txn.open_table(HISTORY_TABLE)?;
                let mut pinned_table = write_txn.open_table(PINNED_TABLE)?;

                // Collect keys first, then delete
                let history_keys: Vec<String> = history_table
                    .iter()?
                    .filter_map(|r| r.ok().map(|(k, _)| k.value().to_string()))
                    .collect();
                let pinned_keys: Vec<String> = pinned_table
                    .iter()?
                    .filter_map(|r| r.ok().map(|(k, _)| k.value().to_string()))
                    .collect();

                for key in history_keys {
                    history_table.remove(key.as_str())?;
                }
                for key in pinned_keys {
                    pinned_table.remove(key.as_str())?;
                }
            }
            write_txn.commit().wrap_err("Error clearing database")?;

            println!("Database cleared succesfully!");
            println!(
                "To fully remove the database, delete {}",
                project_dirs.data_local_dir().display()
            );
            // Lock file cleanup is handled by LockGuard when it goes out of scope
            return Ok(());
        }

        if cli.clear_cache {
            let cache = crate::core::cache::DesktopCache::new(db.clone())?;
            cache.clear().wrap_err("Error clearing cache")?;
            println!("Desktop file cache cleared successfully!");
            return Ok(());
        }

        if cli.refresh_cache {
            let cache = crate::core::cache::DesktopCache::new(db.clone())?;
            // Just clear the file list, parsed apps stay cached
            cache.clear_file_list().wrap_err("Error refreshing cache")?;
            println!("Desktop file list refreshed - will rescan on next launch!");
            return Ok(());
        }
    } else {
        return Err(eyre!(
            "can't find data dir for {}, is your system broken?",
            env!("CARGO_PKG_NAME")
        ));
    };

    // Directories to look for applications (XDG Base Directory Specification)
    let mut dirs: Vec<path::PathBuf> = vec![];

    // User data directory (XDG_DATA_HOME or ~/.local/share)
    if let Some(xdg_data_home) = env::var("XDG_DATA_HOME").ok().filter(|s| !s.is_empty()) {
        let mut dir = path::PathBuf::from(xdg_data_home);
        dir.push("applications");
        if dir.exists() {
            dirs.push(dir);
        }
    } else if let Some(home_dir) = directories::UserDirs::new().map(|d| d.home_dir().to_path_buf()) {
        let mut dir = home_dir;
        dir.push(".local/share/applications");
        if dir.exists() {
            dirs.push(dir);
        }
    }

    // System data directories (XDG_DATA_DIRS)
    if let Ok(res) = env::var("XDG_DATA_DIRS") {
        for data_dir in res.split(':').filter(|s| !s.is_empty()) {
            let mut dir = path::PathBuf::from(data_dir);
            dir.push("applications");
            if dir.exists() {
                dirs.push(dir);
            }
        }
    } else {
        // XDG specification fallback directories for Linux and BSD
        let mut default_paths = vec![
            path::PathBuf::from("/usr/local/share"),
            path::PathBuf::from("/usr/share"),
        ];

        // add BSD-specific paths
        #[cfg(target_os = "openbsd")]
        {
            default_paths.push(path::PathBuf::from("/usr/X11R6/share"));
        }

        for data_dir in &mut default_paths {
            data_dir.push("applications");
            if data_dir.exists() {
                dirs.push(data_dir.clone());
            }
        }
    }

    // Initialize debug mode if requested
    if cli.test_mode {
        crate::cli::DEBUG_ENABLED.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Err(e) = crate::core::debug_logger::init_test_log() {
            eprintln!("Warning: Failed to initialize debug logging: {}", e);
        } else {
            crate::core::debug_logger::log_event("App launcher started in test mode");
        }
    }

    // Load database and cache (Blocking I/O - Keep for now)
    // gotta load everything up front or it looks janky af
    // no one wants to see apps popping in one by one like its 1999
    let filter_desktop = cli.filter_desktop;
    let list_executables = cli.list_executables_in_path;

    let apps_rx =
        crate::desktop::read_with_options(dirs.clone(), &db, filter_desktop, list_executables);

    let mut all_apps = Vec::with_capacity(500);
    while let Ok(app) = apps_rx.recv() {
        all_apps.push(app);
    }

    // Sort by frecency ONCE
    let frecency_data = crate::core::database::load_frecency(&db);
    sort_by_frecency(&mut all_apps, &frecency_data);

    // Log startup info if in test mode
    if crate::cli::DEBUG_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        crate::core::debug_logger::log_startup_info(&cli, all_apps.len(), frecency_data.len());
    }

    // Initialize the terminal with crossterm backend using stderr
    let backend = CrosstermBackend::new(io::stderr());
    let mut terminal = Terminal::new(backend).wrap_err("Failed to start crossterm terminal")?;
    terminal.hide_cursor().wrap_err("Failed to hide cursor")?;
    terminal.clear().wrap_err("Failed to clear terminal")?;

    // Initialize State with ALL apps loaded
    let mut state = State::new(all_apps, cli.match_mode, frecency_data, cli.prefix_depth);

    // Pre-fill search
    if let Some(ref s) = cli.search_string {
        state.query = s.clone();
    }

    // Filter ONCE with all apps loaded - INSTANT display
    state.filter();
    state.update_info(
        cli.highlight_color,
        cli.fancy_mode,
        cli.verbose.unwrap_or(0),
    );

    // Initialize Async Input
    let mut input = InputConfig {
        disable_mouse: cli.disable_mouse,
        tick_rate: Duration::from_millis(16),
        exit_key: KeyCode::Null, // Handle exit manually
        ..InputConfig::default()
    }
    .init_async();

    // App Loop
    loop {
        // Render
        terminal.draw(|f| {
            let ui = UI::new();
            ui.render(f, &state, &cli);
        })?;

        // Handle Events
        tokio::select! {
            // Input Event
            Some(event) = input.next() => {
                match event {
                    Event::Input(key) => {
                         // figure out how many items actually fit on screen
                         let total_height = terminal.size()?.height;
                         let title_height = (total_height as f32 * cli.title_panel_height_percent as f32 / 100.0).round() as u16;
                         let input_height = cli.input_panel_height;
                         let apps_panel_height = total_height.saturating_sub(title_height + input_height);
                         let max_visible = apps_panel_height.saturating_sub(2) as usize; // -2 for borders

                         // Map cursor/keys to Message using configured keybinds
                         let msg = if cli.keybinds.matches_exit(key.code, key.modifiers) {
                             Message::Exit
                         } else if cli.keybinds.matches_select(key.code, key.modifiers) {
                             Message::Select
                         } else if cli.keybinds.matches_up(key.code, key.modifiers) {
                             Message::MoveUp
                         } else if cli.keybinds.matches_down(key.code, key.modifiers) {
                             Message::MoveDown
                         } else if cli.keybinds.matches_left(key.code, key.modifiers) {
                             Message::MoveUp // Left mapped to Up for list navigation consistency if desired, or change logic
                         } else if cli.keybinds.matches_right(key.code, key.modifiers) {
                             Message::MoveDown // Right mapped to Down
                         } else if cli.keybinds.matches_backspace(key.code, key.modifiers) {
                             Message::Backspace
                         } else if cli.keybinds.matches_pin(key.code, key.modifiers) {
                             // Handle Pin toggling directly here as it requires DB access,
                             // or emit Message::TogglePin which we intercept below.
                             // Let's emit Message::TogglePin to keep it clean, and handle it in the State update post-check?
                             // Actually, State update doesn't have DB access.
                             // So we handle it here and return Tick.
                             // Check if we need to manually toggle logic here.
                             if let Some(idx) = state.selected {
                                 if let Some(app) = state.shown.get(idx).cloned() {
                                     if let Ok(is_pinned) = crate::core::database::toggle_pin(&db, &app.name) {
                                        // Update app in lists
                                        for a in &mut state.apps {
                                            if a.name == app.name {
                                                a.pinned = is_pinned;
                                            }
                                        }
                                        // Re-sort so pinned apps move to top
                                        let frecency_data = crate::core::database::load_frecency(&db);
                                        crate::core::state::sort_by_frecency(&mut state.apps, &frecency_data);
                                        state.filter();
                                     }
                                 }
                             }
                             Message::Tick
                         } else {
                             match key.code {
                                 KeyCode::Char(c) if !key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) && !key.modifiers.contains(crossterm::event::KeyModifiers::ALT) => Message::CharInput(c),
                                 KeyCode::Home => Message::MoveFirst,
                                 KeyCode::End => Message::MoveLast,
                                 KeyCode::Tab => Message::MoveDown,
                                 KeyCode::BackTab => Message::MoveUp,
                                 _ => Message::Tick,
                             }
                         };

                         // Special case for Ctrl+C if not handled by keybinds (though it's usually in exit)
                         if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') && !cli.keybinds.matches_exit(key.code, key.modifiers) {
                             state.should_exit = true;
                         }

                         crate::core::state::update(&mut state, msg, cli.hard_stop, max_visible);

                         // Post-update: Check text (update info)
                         let fancy = cli.fancy_mode;
                         state.update_info(cli.highlight_color, fancy, cli.verbose.unwrap_or(0));
                    }
                    Event::Tick => {
                        // Animation frame
                    }
                    Event::Render => {
                        // Trigger redraw
                        // Handled by loop start
                    }
                    Event::Mouse(mouse_event) => {
                        let mouse_row = mouse_event.row;

                        // Calculate panel positions based on title_panel_position
                        let total_height = terminal.size()?.height;
                        let title_height = (total_height as f32 * cli.title_panel_height_percent as f32
                            / 100.0)
                            .round() as u16;
                        let input_height = cli.input_panel_height;
                        let title_panel_position = cli
                            .title_panel_position
                            .unwrap_or(crate::ui::PanelPosition::Top);

                        // Calculate apps panel coordinates based on layout
                        let (apps_panel_start, apps_panel_height) = match title_panel_position {
                            crate::ui::PanelPosition::Top => {
                                // Top: title, apps, input - apps start after title
                                (title_height, total_height.saturating_sub(title_height + input_height))
                            }
                            crate::ui::PanelPosition::Middle => {
                                // Middle: apps, title, input - apps start at top
                                (0, total_height.saturating_sub(title_height + input_height))
                            }
                            crate::ui::PanelPosition::Bottom => {
                                // Bottom: apps, input, title - apps start at top
                                (0, total_height.saturating_sub(title_height + input_height))
                            }
                        };

                        // List content area (inside the borders) - first item starts 1 row down from panel start
                        let list_content_start = apps_panel_start + 1;
                        let max_visible_rows = apps_panel_height.saturating_sub(2); // -2 for top/bottom borders
                        let list_content_end = list_content_start + max_visible_rows;

                        // Helper to calculate index from row
                        let get_app_index = |row: u16| -> Option<usize> {
                            if row >= list_content_start && row < list_content_end {
                                let row_in_content = row - list_content_start;
                                let index = state.scroll_offset + row_in_content as usize;
                                if index < state.shown.len() {
                                    return Some(index);
                                }
                            }
                            None
                        };

                        let msg = match mouse_event.kind {
                            MouseEventKind::Down(MouseButton::Left) => {
                                if let Some(idx) = get_app_index(mouse_row) {
                                    crate::core::state::update(&mut state, Message::SelectIndex(idx), cli.hard_stop, max_visible_rows as usize);
                                    Message::Select
                                } else {
                                    Message::Tick
                                }
                            },
                            MouseEventKind::Moved => {
                                if let Some(idx) = get_app_index(mouse_row) {
                                    Message::SelectIndex(idx)
                                } else {
                                    Message::Tick
                                }
                            },
                            // scrollin scrollin scrollin, keep that cursor rollin
                            MouseEventKind::ScrollDown => {
                                if mouse_row >= list_content_start && mouse_row < list_content_end && !state.shown.is_empty() {
                                    let max_visible = max_visible_rows as usize;
                                    if state.scroll_offset + max_visible < state.shown.len() {
                                        state.scroll_offset += 1;
                                        // snap cursor to wherever the mouse is chillin
                                        let row_in_content = mouse_row - list_content_start;
                                        let new_idx = state.scroll_offset + row_in_content as usize;
                                        if new_idx < state.shown.len() {
                                            state.selected = Some(new_idx);
                                            state.update_info(cli.highlight_color, cli.fancy_mode, cli.verbose.unwrap_or(0));
                                        }
                                    }
                                }
                                Message::Tick
                            },
                            MouseEventKind::ScrollUp => {
                                if mouse_row >= list_content_start && mouse_row < list_content_end && !state.shown.is_empty() && state.scroll_offset > 0 {
                                    state.scroll_offset -= 1;
                                    // same deal, keep cursor under mouse
                                    let row_in_content = mouse_row - list_content_start;
                                    let new_idx = state.scroll_offset + row_in_content as usize;
                                    if new_idx < state.shown.len() {
                                        state.selected = Some(new_idx);
                                        state.update_info(cli.highlight_color, cli.fancy_mode, cli.verbose.unwrap_or(0));
                                    }
                                }
                                Message::Tick
                            },
                            _ => Message::Tick,
                        };

                        if let Message::Tick = msg {
                            // Don't update for ignored mouse events
                        } else {
                            if crate::cli::DEBUG_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                                crate::core::debug_logger::log_event(&format!("State update via Mouse: {:?}", msg));
                            }

                            crate::core::state::update(&mut state, msg, cli.hard_stop, max_visible_rows as usize);

                            // Post-update: Check text (update info)
                            let fancy = cli.fancy_mode;
                            state.update_info(cli.highlight_color, fancy, cli.verbose.unwrap_or(0));
                        }
                    }
                }
            }
        }

        if state.should_exit {
            if crate::cli::DEBUG_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                crate::core::debug_logger::log_session_end();
            }
            break;
        }

        if state.should_launch {
            if let Some(selected_idx) = state.selected {
                if let Some(app) = state.shown.get(selected_idx) {
                    // Record access in frecency
                    if let Err(e) = crate::core::database::record_access(&db, &app.name) {
                        eprintln!("Failed to record access: {}", e);
                    }

                    crate::shutdown_terminal(cli.disable_mouse)?;

                    // Launch
                    // Handle --no-exec
                    if cli.no_exec {
                        println!("{}", app.command);
                        return Ok(());
                    }

                    super::launch::launch_app(app, &cli, &db)?;
                }
            }
            break;
        }
    }

    if !state.should_launch {
        crate::shutdown_terminal(cli.disable_mouse)?;
    }

    // Log session end if in test mode
    if crate::cli::DEBUG_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        crate::core::debug_logger::log_session_end();
    }

    // Lock file cleanup handled by Guard
    Ok(())
}
