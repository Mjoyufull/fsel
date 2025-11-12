//! Application launcher mode

use crate::cli::Opts;
use eyre::{eyre, Result, WrapErr};

use crate::core::database;
use crate::desktop;
use crate::ui::{InputConfig, InputEvent as Event, UI};

use crossterm::event::{MouseButton, MouseEventKind};
use directories::ProjectDirs;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;
use redb::ReadableTable;
use scopeguard::defer;
use std::collections::BTreeSet;
use std::{env, fs, io, path};

/// Run application launcher mode
pub fn run(cli: Opts) -> Result<()> {
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
    } else if let Some(home_dir) = dirs::home_dir() {
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

    // Read applications with filtering options
    let apps =
        desktop::read_with_options(dirs, &db, cli.filter_desktop, cli.list_executables_in_path);

    // Initialize the terminal with crossterm backend using stderr
    let backend = CrosstermBackend::new(io::stderr());
    let mut terminal = Terminal::new(backend).wrap_err("Failed to start crossterm terminal")?;
    terminal.hide_cursor().wrap_err("Failed to hide cursor")?;
    terminal.clear().wrap_err("Failed to clear terminal")?;

    // Input handler with fast tick rate for instant app loading
    let input = InputConfig {
        disable_mouse: cli.disable_mouse,
        tick_rate: std::time::Duration::from_millis(16), // ~60fps for instant updates
        exit_key: KeyCode::Null, // Use Null key to prevent accidental input thread termination
        ..InputConfig::default()
    }
    .init();

    // PERFORMANCE FIX: Load ALL apps FIRST, then filter ONCE (eliminates "populating" effect)
    let mut all_apps = Vec::with_capacity(500);
    while let Ok(app) = apps.recv() {
        all_apps.push(app);
    }

    // Create UI with ALL apps loaded
    let mut ui = UI::new(all_apps);

    // Set user-defined verbosity level
    if let Some(level) = cli.verbose {
        ui.verbosity(level);
    }

    // Pre-fill search string if provided
    if let Some(ref search_str) = cli.search_string {
        ui.query = search_str.clone();
    }

    // Filter ONCE with all apps loaded - INSTANT display
    ui.filter(cli.match_mode);
    ui.info(cli.highlight_color, cli.fancy_mode);

    // App list
    let mut app_state = ListState::default();

    loop {
        // Draw UI
        terminal.draw(|f| {
            // Calculate layout based on configuration
            let total_height = f.area().height;
            let title_height = (total_height as f32 * cli.title_panel_height_percent as f32 / 100.0)
                .round() as u16;
            let input_height = cli.input_panel_height;

            // Get title panel position (defaults to Top if not set)
            let title_panel_position = cli
                .title_panel_position
                .unwrap_or(crate::cli::PanelPosition::Top);

            // Split the window into three parts based on title panel position
            let (window, title_panel_index, apps_panel_index, input_panel_index) =
                match title_panel_position {
                    crate::cli::PanelPosition::Top => {
                        // Top: title, apps, input (original layout)
                        let layout = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints(
                                [
                                    Constraint::Length(title_height.max(3)), // Title panel (min 3 lines)
                                    Constraint::Min(1), // Apps panel (remaining space)
                                    Constraint::Length(input_height), // Input panel
                                ]
                                .as_ref(),
                            )
                            .split(f.area());
                        (layout, 0, 1, 2)
                    }
                    crate::cli::PanelPosition::Middle => {
                        // Middle: apps, title, input
                        let layout = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints(
                                [
                                    Constraint::Min(1),                      // Apps panel (remaining space)
                                    Constraint::Length(title_height.max(3)), // Title panel
                                    Constraint::Length(input_height),        // Input panel
                                ]
                                .as_ref(),
                            )
                            .split(f.area());
                        (layout, 1, 0, 2)
                    }
                    crate::cli::PanelPosition::Bottom => {
                        // Bottom: apps, input, title
                        let layout = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints(
                                [
                                    Constraint::Min(1),                      // Apps panel (remaining space)
                                    Constraint::Length(input_height),        // Input panel
                                    Constraint::Length(title_height.max(3)), // Title panel at bottom
                                ]
                                .as_ref(),
                            )
                            .split(f.area());
                        (layout, 2, 0, 1)
                    }
                };

            // Create blocks with configurable colors and borders
            let border_type = if cli.rounded_borders {
                BorderType::Rounded
            } else {
                BorderType::Plain
            };

            let create_main_block = |title: String| {
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        format!(" {} ", title), // Add spaces around title
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(cli.header_title_color),
                    ))
                    .border_type(border_type)
                    .border_style(Style::default().fg(cli.main_border_color))
            };

            let create_apps_block = |title: String| {
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        format!(" {} ", title), // Add spaces around title
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(cli.header_title_color),
                    ))
                    .border_type(border_type)
                    .border_style(Style::default().fg(cli.apps_border_color))
            };

            let create_input_block = |title: String| {
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        format!(" {} ", title), // Add spaces around title
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(cli.header_title_color),
                    ))
                    .border_type(border_type)
                    .border_style(Style::default().fg(cli.input_border_color))
            };

            // Determine panel titles based on fancy mode
            let (main_title, apps_title) = if cli.fancy_mode
                && ui.selected.is_some()
                && !ui.shown.is_empty()
                && ui.selected.unwrap() < ui.shown.len()
            {
                let selected_app = &ui.shown[ui.selected.unwrap()];
                // In fancy mode: main panel shows app name, apps panel shows "Apps"
                (selected_app.name.clone(), "Apps".to_string())
            } else {
                // Normal mode: static titles
                ("Fsel".to_string(), "Apps".to_string())
            };

            // Description of the current app
            let description = Paragraph::new(ui.text.clone())
                .block(create_main_block(main_title))
                .style(Style::default().fg(cli.main_text_color))
                // Don't trim leading spaces when wrapping
                .wrap(Wrap { trim: false })
                .alignment(Alignment::Left);

            // Calculate apps panel height - account for borders (2 rows: top + bottom)
            let apps_panel_height = window[apps_panel_index].height;
            let max_visible = apps_panel_height.saturating_sub(2) as usize; // -2 for top/bottom borders

            // get the visible slice of apps based on scroll offset
            let visible_apps = ui
                .shown
                .iter()
                .skip(ui.scroll_offset)
                .take(max_visible)
                .map(|app| {
                    if app.pinned {
                        // add pin icon with color
                        let pin_span = Span::styled(
                            format!("{} ", cli.pin_icon),
                            Style::default().fg(cli.pin_color),
                        );
                        let name_span = Span::raw(&app.name);
                        ListItem::new(Line::from(vec![pin_span, name_span]))
                    } else {
                        ListItem::new(app.name.clone())
                    }
                })
                .collect::<Vec<ListItem>>();

            // App list (stateful widget) with borders
            let list = List::new(visible_apps)
                .block(create_apps_block(apps_title))
                .style(Style::default().fg(cli.apps_text_color))
                // Bold & colorized selection
                .highlight_style(
                    Style::default()
                        .fg(cli.highlight_color)
                        .add_modifier(Modifier::BOLD),
                )
                // Prefixed before the list item
                .highlight_symbol("> ");

            // Ensure we always have a valid selection when rendering
            if !ui.shown.is_empty() {
                match ui.selected {
                    None => {
                        // No selection at all, default to first visible item
                        ui.selected = Some(ui.scroll_offset.min(ui.shown.len() - 1));
                    }
                    Some(sel) if sel >= ui.shown.len() => {
                        // Selection is out of bounds, clamp to valid range
                        ui.selected = Some((ui.shown.len() - 1).min(sel));
                    }
                    _ => {
                        // Selection is valid, keep it
                    }
                }
            }

            // Update selection - adjust for scroll offset
            let visible_selection = ui.selected.and_then(|sel| {
                if sel >= ui.scroll_offset && sel < ui.scroll_offset + max_visible {
                    Some(sel - ui.scroll_offset)
                } else {
                    None
                }
            });
            app_state.select(visible_selection);

            // Query
            let query = Paragraph::new(Line::from(vec![
                // Format: (10/51) >> query
                Span::styled("(", Style::default().fg(cli.input_text_color)),
                Span::styled(
                    (ui.selected.map_or(0, |v| v + 1)).to_string(),
                    Style::default().fg(cli.highlight_color),
                ),
                Span::styled("/", Style::default().fg(cli.input_text_color)),
                Span::styled(
                    ui.shown.len().to_string(),
                    Style::default().fg(cli.input_text_color),
                ),
                Span::styled(") ", Style::default().fg(cli.input_text_color)),
                Span::styled(">", Style::default().fg(cli.highlight_color)),
                Span::styled("> ", Style::default().fg(cli.input_text_color)),
                Span::styled(&ui.query, Style::default().fg(cli.input_text_color)),
                Span::styled(&cli.cursor, Style::default().fg(cli.highlight_color)),
            ]))
            .block(create_input_block("Input".to_string()))
            .style(Style::default().fg(cli.input_text_color))
            .alignment(Alignment::Left)
            .wrap(ratatui::widgets::Wrap { trim: false });

            // Render panels in their dynamic positions
            f.render_widget(description, window[title_panel_index]);
            // Only render app list if not hide_before_typing or query is not empty
            if !cli.hide_before_typing || !ui.query.is_empty() {
                f.render_stateful_widget(list, window[apps_panel_index], &mut app_state);
            }
            f.render_widget(query, window[input_panel_index]);
        })?;

        // Handle user input
        match input.next()? {
            Event::Input(key) => {
                use crossterm::event::KeyCode;

                // check keybinds
                if cli.keybinds.matches_exit(key.code, key.modifiers) {
                    ui.selected = None;
                    break;
                } else if cli.keybinds.matches_select(key.code, key.modifiers) {
                    break;
                } else if cli.keybinds.matches_pin(key.code, key.modifiers) {
                    if let Some(selected) = ui.selected {
                        if selected < ui.shown.len() {
                            let app_name = ui.shown[selected].name.clone();
                            if let Ok(is_pinned) = database::toggle_pin(&db, &app_name) {
                                // Update all apps with same name (handles duplicates like 2x Alacritty)
                                for app in &mut ui.shown {
                                    if app.name == app_name {
                                        app.pinned = is_pinned;
                                    }
                                }
                                ui.filter(cli.match_mode);
                                // Cursor stays put, app moves to top
                            }
                        }
                    }
                } else if cli.keybinds.matches_backspace(key.code, key.modifiers) {
                    ui.query.pop();
                    ui.filter(cli.match_mode);
                } else if cli.keybinds.matches_left(key.code, key.modifiers) {
                    if !ui.shown.is_empty() {
                        ui.selected = Some(0);
                    }
                } else if cli.keybinds.matches_right(key.code, key.modifiers) {
                    if !ui.shown.is_empty() {
                        ui.selected = Some(ui.shown.len() - 1);
                    }
                } else if cli.keybinds.matches_down(key.code, key.modifiers) {
                    if let Some(selected) = ui.selected {
                        ui.selected = if selected < ui.shown.len() - 1 {
                            Some(selected + 1)
                        } else if !cli.hard_stop {
                            Some(0)
                        } else {
                            Some(selected)
                        };

                        // Auto-scroll to keep selection visible
                        if let Some(new_selected) = ui.selected {
                            let total_height = terminal.size()?.height;
                            let title_height = (total_height as f32
                                * cli.title_panel_height_percent as f32
                                / 100.0)
                                .round() as u16;
                            let input_height = cli.input_panel_height;
                            let apps_panel_height = total_height - title_height - input_height;
                            let max_visible = apps_panel_height.saturating_sub(2) as usize; // -2 for borders

                            // Scroll down if selection is below visible area
                            if new_selected >= ui.scroll_offset + max_visible {
                                ui.scroll_offset = new_selected.saturating_sub(max_visible - 1);
                            }
                            // Scroll up if selection is above visible area (happens when wrapping to top)
                            else if new_selected < ui.scroll_offset {
                                ui.scroll_offset = new_selected;
                            }
                        }
                    }
                } else if cli.keybinds.matches_up(key.code, key.modifiers) {
                    if let Some(selected) = ui.selected {
                        ui.selected = if selected > 0 {
                            Some(selected - 1)
                        } else if !cli.hard_stop {
                            Some(ui.shown.len() - 1)
                        } else {
                            Some(selected)
                        };

                        // Auto-scroll to keep selection visible
                        if let Some(new_selected) = ui.selected {
                            let total_height = terminal.size()?.height;
                            let title_height = (total_height as f32
                                * cli.title_panel_height_percent as f32
                                / 100.0)
                                .round() as u16;
                            let input_height = cli.input_panel_height;
                            let apps_panel_height = total_height - title_height - input_height;
                            let max_visible = apps_panel_height.saturating_sub(2) as usize; // -2 for borders

                            // Scroll up if selection is above visible area
                            if new_selected < ui.scroll_offset {
                                ui.scroll_offset = new_selected;
                            }
                            // Scroll down if selection is below visible area (happens when wrapping to bottom)
                            else if new_selected >= ui.scroll_offset + max_visible {
                                ui.scroll_offset = new_selected.saturating_sub(max_visible - 1);
                            }
                        }
                    }
                } else {
                    // regular character input
                    match (key.code, key.modifiers) {
                        (KeyCode::Char(c), crossterm::event::KeyModifiers::NONE)
                        | (KeyCode::Char(c), crossterm::event::KeyModifiers::SHIFT) => {
                            ui.query.push(c);
                            ui.filter(cli.match_mode);
                        }
                        _ => {}
                    }
                }

                ui.info(cli.highlight_color, cli.fancy_mode);
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
                    .unwrap_or(crate::cli::PanelPosition::Top);

                // Calculate apps panel coordinates based on layout
                let (apps_panel_start, apps_panel_height) = match title_panel_position {
                    crate::cli::PanelPosition::Top => {
                        // Top: title, apps, input - apps start after title
                        (title_height, total_height - title_height - input_height)
                    }
                    crate::cli::PanelPosition::Middle => {
                        // Middle: apps, title, input - apps start at top
                        (0, total_height - title_height - input_height)
                    }
                    crate::cli::PanelPosition::Bottom => {
                        // Bottom: apps, input, title - apps start at top
                        (0, total_height - title_height - input_height)
                    }
                };

                // List content area (inside the borders) - first item starts 1 row down from panel start
                let list_content_start = apps_panel_start + 1; // +1 for top border
                let max_visible_rows = apps_panel_height.saturating_sub(2); // -2 for top/bottom borders
                let list_content_end = list_content_start + max_visible_rows;

                let update_selection_for_mouse_pos = |ui: &mut UI, mouse_row: u16| {
                    if !ui.shown.is_empty()
                        && mouse_row >= list_content_start
                        && mouse_row < list_content_end
                    {
                        let row_in_content = mouse_row - list_content_start;
                        let hovered_app_index = ui.scroll_offset + row_in_content as usize;
                        if hovered_app_index < ui.shown.len() {
                            ui.selected = Some(hovered_app_index);
                            ui.info(cli.highlight_color, cli.fancy_mode);
                        }
                    }
                };

                match mouse_event.kind {
                    // Handle mouse movement for hover highlighting
                    MouseEventKind::Moved => {
                        update_selection_for_mouse_pos(&mut ui, mouse_row);
                    }
                    // Handle left mouse button clicks to launch
                    MouseEventKind::Down(MouseButton::Left) => {
                        // Check if click is within the list content area
                        if mouse_row >= list_content_start
                            && mouse_row < list_content_end
                            && !ui.shown.is_empty()
                        {
                            let row_in_content = mouse_row - list_content_start;
                            let clicked_app_index = ui.scroll_offset + row_in_content as usize;

                            if clicked_app_index < ui.shown.len() {
                                ui.selected = Some(clicked_app_index);
                                ui.info(cli.highlight_color, cli.fancy_mode);
                                break; // Launch the clicked app
                            }
                        }
                    }
                    // Handle scroll wheel only when mouse is over the apps list
                    MouseEventKind::ScrollUp => {
                        if mouse_row >= list_content_start
                            && mouse_row < list_content_end
                            && !ui.shown.is_empty()
                            && ui.scroll_offset > 0
                        {
                            ui.scroll_offset -= 1;
                            update_selection_for_mouse_pos(&mut ui, mouse_row);
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        if mouse_row >= list_content_start
                            && mouse_row < list_content_end
                            && !ui.shown.is_empty()
                        {
                            let max_visible = max_visible_rows as usize;
                            if ui.scroll_offset + max_visible < ui.shown.len() {
                                ui.scroll_offset += 1;
                                update_selection_for_mouse_pos(&mut ui, mouse_row);
                            }
                        }
                    }
                    _ => {} // Ignore other mouse events
                }
            }
            Event::Tick => {}
        }
    }

    // Clean terminal exit (defer handles the rest)
    terminal.show_cursor().wrap_err("Failed to show cursor")?;

    if let Some(selected) = ui.selected {
        let app_to_run = &ui.shown[selected];

        // Handle --no-exec: print command and exit cleanly
        if cli.no_exec {
            println!("{}", app_to_run.command);
            return Ok(());
        }

        // launch the app
        super::launch::launch_app(app_to_run, &cli, &db)?;
    }

    // Lock file cleanup is handled by LockGuard
    Ok(())
}
