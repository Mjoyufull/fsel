#![deny(unsafe_code)]
#![deny(missing_docs)]

//! # Gyr
//!
//! > _Blazing fast_ TUI launcher for GNU/Linux and *BSD
//!
//! For more info, check the [README](https://sr.ht/~f9/gyr)

/// CLI parser
mod cli;
/// Terminal input helpers
mod input;
/// Ui helpers
mod ui;
/// XDG apps
mod xdg;

use input::{Event, Input};
use ui::UI;

use std::env;
use std::fs;
use std::io;
use std::os::unix::process::CommandExt;
use std::path;
use std::process;
use std::sync::mpsc;

use directories::ProjectDirs;
use eyre::eyre;
use eyre::WrapErr;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;
use crossterm::{
    event::{EnableMouseCapture, DisableMouseCapture, MouseButton, MouseEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use scopeguard::defer;

fn main() {
    if let Err(error) = real_main() {
        shutdown_terminal();
        eprintln!("{error:?}");
        process::exit(1);
    }
}

fn setup_terminal() -> eyre::Result<()> {
    enable_raw_mode().wrap_err("Failed to enable raw mode")?;
    io::stdout().execute(EnterAlternateScreen).wrap_err("Failed to enter alternate screen")?;
    io::stdout().execute(EnableMouseCapture).wrap_err("Failed to enable mouse capture")?;
    Ok(())
}

fn shutdown_terminal() {
    let _ = io::stdout().execute(DisableMouseCapture);
    let _ = io::stdout().execute(LeaveAlternateScreen);
    let _ = disable_raw_mode();
}

fn launch_program_directly(cli: &cli::Opts, program_name: &str) -> eyre::Result<()> {
    use directories::ProjectDirs;
    use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
    
    // Open database for history (same logic as in main)
    let db = if let Some(project_dirs) = ProjectDirs::from("ch", "forkbomb9", env!("CARGO_PKG_NAME")) {
        let mut hist_db = project_dirs.data_local_dir().to_path_buf();
        if !hist_db.exists() {
            fs::create_dir_all(&hist_db).wrap_err("Error creating data dir")?;
        }
        hist_db.push("hist_db");
        sled::open(hist_db).wrap_err("Failed to open database")?
    } else {
        return Err(eyre!("can't find data dir for {}", env!("CARGO_PKG_NAME")));
    };
    
    // Get application directories (same logic as in main)
    let mut dirs: Vec<path::PathBuf> = vec![];
    
    // Add user's data directory
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
    
    // Add system data directories
    if let Ok(res) = env::var("XDG_DATA_DIRS") {
        for data_dir in res.split(':').filter(|s| !s.is_empty()) {
            let mut dir = path::PathBuf::from(data_dir);
            dir.push("applications");
            if dir.exists() {
                dirs.push(dir);
            }
        }
    } else {
        for data_dir in &mut [
            path::PathBuf::from("/usr/local/share"),
            path::PathBuf::from("/usr/share"),
        ] {
            data_dir.push("applications");
            if data_dir.exists() {
                dirs.push(data_dir.clone());
            }
        }
    }
    
    // Read applications
    let apps_receiver = xdg::read(dirs, &db);
    
    // Collect all apps
    let mut all_apps = Vec::new();
    while let Ok(app) = apps_receiver.recv() {
        all_apps.push(app);
    }
    
    if all_apps.is_empty() {
        return Err(eyre!("No applications found"));
    }
    
    // Find the best match using fuzzy matching
    let matcher = SkimMatcherV2::default();
    let mut best_app: Option<(xdg::App, i64)> = None;
    
    for app in all_apps {
        // Try to match against name, generic name, keywords, and description
        let mut best_score = None;
        
        // Match against app name (highest priority)
        if let Some(score) = matcher.fuzzy_match(&app.name, program_name) {
            best_score = Some(score * 3);
        }
        
        // Match against generic name
        if let Some(ref generic_name) = app.generic_name {
            if let Some(score) = matcher.fuzzy_match(generic_name, program_name) {
                let boosted_score = score * 2;
                best_score = Some(best_score.map_or(boosted_score, |current| current.max(boosted_score)));
            }
        }
        
        // Match against keywords
        for keyword in &app.keywords {
            if let Some(score) = matcher.fuzzy_match(keyword, program_name) {
                let boosted_score = score * 2;
                best_score = Some(best_score.map_or(boosted_score, |current| current.max(boosted_score)));
            }
        }
        
        // Match against description
        if let Some(score) = matcher.fuzzy_match(&app.description, program_name) {
            best_score = Some(best_score.map_or(score, |current| current.max(score)));
        }
        
        if let Some(score) = best_score {
            // Include history in scoring
            let final_score = if app.history > 0 {
                score * app.history as i64
            } else {
                score
            };
            
            if let Some((_, current_best_score)) = &best_app {
                if final_score > *current_best_score {
                    best_app = Some((app, final_score));
                }
            } else {
                best_app = Some((app, final_score));
            }
        }
    }
    
    let app_to_run = match best_app {
        Some((app, _)) => app,
        None => {
            return Err(eyre!("No matching application found for '{}'", program_name));
        }
    };
    
    // Print what we're launching if verbose
    if cli.verbose.unwrap_or(0) > 0 {
        eprintln!("Launching: {} ({})", app_to_run.name, app_to_run.command);
    }
    
    // Handle --no-exec: print command and exit cleanly
    if cli.no_exec {
        println!("{}", app_to_run.command);
        return Ok(());
    }
    
    // Launch the app (same logic as in main)
    let commands = shell_words::split(&app_to_run.command)?;
    
    if let Some(path) = &app_to_run.path {
        env::set_current_dir(path::PathBuf::from(path)).wrap_err_with(|| {
            format!("Failed to switch to {path} when starting {app_to_run}")
        })?;
    }
    
    let mut runner: Vec<&str> = vec![];
    
    // Determine launch method based on flags
    if cli.uwsm {
        runner.extend_from_slice(&["uwsm", "app", "--"]);
    } else if cli.systemd_run {
        runner.extend_from_slice(&["systemd-run", "--user", "--scope", "--"]);
    } else if cli.sway {
        runner.extend_from_slice(&["swaymsg", "exec", "--"]);
    }
    
    if app_to_run.is_terminal {
        runner.extend_from_slice(&cli.terminal_launcher.split(' ').collect::<Vec<&str>>());
    }
    
    runner.extend_from_slice(&commands.iter().map(AsRef::as_ref).collect::<Vec<&str>>());
    
    let mut exec = process::Command::new(runner[0]);
    exec.args(&runner[1..]);
    
    #[allow(unsafe_code)]
    unsafe {
        exec.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    
    if cli.verbose.unwrap_or(0) > 0 {
        exec.stdin(process::Stdio::null())
            .stdout(process::Stdio::null())
            .stderr(process::Stdio::null())
            .spawn()
            .wrap_err_with(|| format!("Failed to run {exec:?}"))?;
    } else {
        exec.spawn()
            .wrap_err_with(|| format!("Failed to run {exec:?}"))?;
    }
    
    // Update history
    let value = app_to_run.history + 1;
    let packed = bytes::pack(value);
    db.insert(app_to_run.name.as_bytes(), &packed).unwrap();
    
    Ok(())
}

fn real_main() -> eyre::Result<()> {
    let cli = cli::parse()?;
    
    // Handle direct launch mode (bypass TUI)
    if let Some(ref program_name) = cli.program {
        return launch_program_directly(&cli, program_name);
    }
    
    setup_terminal()?;
    defer! {
        shutdown_terminal();
    }
    let db: sled::Db;
    let lock_path: path::PathBuf;

    // Open sled database
    if let Some(project_dirs) = ProjectDirs::from("ch", "forkbomb9", env!("CARGO_PKG_NAME")) {
        let mut hist_db = project_dirs.data_local_dir().to_path_buf();

        if !hist_db.exists() {
            // Create dir if it doesn't exist
            if let Err(error) = fs::create_dir_all(&hist_db) {
                return Err(eyre!(
                    "Error creating data dir {}: {}",
                    hist_db.display(),
                    error,
                ));
            }
        }

        // Check if Gyr is already running
        {
            let mut lock = hist_db.clone();
            lock.push("lock");
            lock_path = lock;
            let contents = match fs::read_to_string(&lock_path) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
                Ok(c) => c,
                Err(e) => {
                    return Err(e).wrap_err("Failed to read lockfile");
                }
            };

            if !contents.is_empty() {
                if cli.replace {
                    let pid: i32 = contents
                        .parse()
                        .wrap_err("Failed to parse lockfile contents")?;
                    #[allow(unsafe_code)]
                    unsafe {
                        libc::kill(pid, libc::SIGTERM);
                    }
                    fs::remove_file(&lock_path)?;
                    std::thread::sleep(std::time::Duration::from_millis(200));
                } else {
                    // gyr is already running
                    return Err(eyre!("Gyr is already running"));
                }
            }

            // Write current pid to lock file
            let mut lock_file = fs::File::create(&lock_path)?;
            let pid;
            // Safety: call to getpid is safe
            #[allow(unsafe_code)]
            unsafe {
                pid = libc::getpid();
            }
            use std::io::Write;
            lock_file.write_all(pid.to_string().as_bytes())?;
        }

        // Create a guard that will clean up the lock file when dropped
        struct LockGuard(path::PathBuf);
        impl Drop for LockGuard {
            fn drop(&mut self) {
                let _ = fs::remove_file(&self.0);
            }
        }
        let _lock_guard = LockGuard(lock_path.clone());

        hist_db.push("hist_db");

        db = sled::open(hist_db).wrap_err("Failed to open database")?;


        if cli.clear_history {
            db.clear().wrap_err("Error clearing database")?;
            println!("Database cleared succesfully!");
            println!(
                "Note: to completely remove all traces of the database,
                remove {}.",
                project_dirs.data_local_dir().display()
            );
            // Lock file cleanup is handled by LockGuard when it goes out of scope
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
    
    // FIRST: Add user's data directory (XDG_DATA_HOME or ~/.local/share)
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
    
    // SECOND: Add system data directories (XDG_DATA_DIRS)
    if let Ok(res) = env::var("XDG_DATA_DIRS") {
        for data_dir in res.split(':').filter(|s| !s.is_empty()) {
            let mut dir = path::PathBuf::from(data_dir);
            dir.push("applications");
            if dir.exists() {
                dirs.push(dir);
            }
        }
    } else {
        // XDG specification fallback directories
        for data_dir in &mut [
            path::PathBuf::from("/usr/local/share"),
            path::PathBuf::from("/usr/share"),
        ] {
            data_dir.push("applications");
            if data_dir.exists() {
                dirs.push(data_dir.clone());
            }
        }
    }


    // Read applications
    let apps = xdg::read(dirs, &db);

    // Initialize the terminal with crossterm backend
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).wrap_err("Failed to start crossterm terminal")?;
    terminal.hide_cursor().wrap_err("Failed to hide cursor")?;
    terminal.clear().wrap_err("Failed to clear terminal")?;

    // Input handler
    let input = Input::new();

    // App UI
    //
    // Get one app to initialize the UI
    let mut ui = UI::new(vec![apps.recv()?]);

    // Set user-defined verbosity level
    if let Some(level) = cli.verbose {
        ui.verbosity(level);
    }
    
    // Pre-fill search string if provided
    if let Some(ref search_str) = cli.search_string {
        ui.query = search_str.clone();
    }

    // App list
    let mut app_state = ListState::default();

    let mut app_loading_finished = false;

    loop {
        if !app_loading_finished {
            loop {
                match apps.try_recv() {
                    Ok(app) => {
                        ui.hidden.push(app);
                    }
                    Err(e) => {
                        match e {
                            mpsc::TryRecvError::Disconnected => {
                                // Done loading, add apps to the UI
                                app_loading_finished = true;
                                ui.filter();
                                ui.info(cli.highlight_color, cli.fancy_mode);
                                
                                // If we have a pre-filled search string, run filter again to apply it
                                if cli.search_string.is_some() {
                                    ui.filter();
                                    ui.info(cli.highlight_color, cli.fancy_mode);
                                }
                            }
                            mpsc::TryRecvError::Empty => (),
                        }
                        break;
                    }
                }
            }
        }

        // Draw UI
        terminal.draw(|f| {
            // Calculate layout based on configuration
            let total_height = f.size().height;
            let title_height = (total_height as f32 * cli.title_panel_height_percent as f32 / 100.0).round() as u16;
            let input_height = cli.input_panel_height;
            
            // Split the window into three parts
            let window = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(title_height.max(3)),  // Title panel (min 3 lines)
                    Constraint::Min(3),                       // Apps panel (remaining space, min 3)
                ].as_ref())
                .split(f.size());

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
                        Style::default().add_modifier(Modifier::BOLD).fg(cli.header_title_color),
                    ))
                    .border_type(border_type)
                    .border_style(Style::default().fg(cli.main_border_color))
            };
            
            let create_apps_block = |title: String| {
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        format!(" {} ", title), // Add spaces around title
                        Style::default().add_modifier(Modifier::BOLD).fg(cli.header_title_color),
                    ))
                    .border_type(border_type)
                    .border_style(Style::default().fg(cli.apps_border_color))
            };
            
            let create_input_block = |title: String| {
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        format!(" {} ", title), // Add spaces around title
                        Style::default().add_modifier(Modifier::BOLD).fg(cli.header_title_color),
                    ))
                    .border_type(border_type)
                    .border_style(Style::default().fg(cli.input_border_color))
            };

            // Split the bottom section (apps + input)
            let bottom_half = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(3),                      // Apps panel (remaining space)
                    Constraint::Length(input_height),        // Input panel (configurable height)
                ].as_ref())
                .split(window[1]);

            // Determine panel titles based on fancy mode
            let (main_title, apps_title) = if cli.fancy_mode 
                && ui.selected.is_some() 
                && !ui.shown.is_empty() 
                && ui.selected.unwrap() < ui.shown.len() {
                let selected_app = &ui.shown[ui.selected.unwrap()];
                // In fancy mode: main panel shows app name, apps panel shows description or "Apps"
                (selected_app.name.clone(), "Apps".to_string())
            } else {
                // Normal mode: static titles
                ("Gyr".to_string(), "Apps".to_string())
            };
            
            // Description of the current app
            let description = Paragraph::new(ui.text.clone())
                .block(create_main_block(main_title))
                .style(Style::default().fg(cli.main_text_color))
                // Don't trim leading spaces when wrapping
                .wrap(Wrap { trim: false })
                .alignment(Alignment::Left);

            // Calculate visible apps based on scroll offset
            let apps_panel_height = f.size().height - (f.size().height as f32 * cli.title_panel_height_percent as f32 / 100.0).round() as u16 - cli.input_panel_height - 2; // -2 for borders
            let max_visible = apps_panel_height.saturating_sub(2) as usize; // -2 for top/bottom borders inside the panel
            
            // Get the visible slice of apps based on scroll offset
            let visible_apps = ui.shown
                .iter()
                .skip(ui.scroll_offset)
                .take(max_visible)
                .map(ListItem::from)
                .collect::<Vec<ListItem>>();

            // App list (stateful widget)
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
                // The resulting style will be:
                // (10/51) >> filter
                // With `10` and the first `>` colorized with the highlight color
                Span::styled("(", Style::default().fg(cli.input_text_color)),
                Span::styled(
                    (ui.selected.map_or(0, |v| v + 1)).to_string(),
                    Style::default().fg(cli.highlight_color),
                ),
                Span::styled("/", Style::default().fg(cli.input_text_color)),
                Span::styled(ui.shown.len().to_string(), Style::default().fg(cli.input_text_color)),
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

            // Render description
            f.render_widget(description, window[0]);
            // Render app list
            f.render_stateful_widget(list, bottom_half[0], &mut app_state);
            // Render query
            f.render_widget(query, bottom_half[1]);
        })?;

        // Handle user input
        match input.next()? {
            Event::Input(key) => {
            use crossterm::event::{KeyCode, KeyModifiers};
            match (key.code, key.modifiers) {
                // Exit on escape
                (KeyCode::Esc, _) | (KeyCode::Char('q'), KeyModifiers::CONTROL) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                    ui.selected = None;
                    break;
                }
                // Run app on enter
                (KeyCode::Enter, _) | (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                    break;
                }
                // Add character to query
                (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                    ui.query.push(c);
                    ui.filter();
                }
                // Remove character from query
                (KeyCode::Backspace, _) => {
                    ui.query.pop();
                    ui.filter();
                }
                // Go to top of list
                (KeyCode::Left, _) => {
                    if !ui.shown.is_empty() {
                        ui.selected = Some(0);
                    }
                }
                // Go to end of list
                (KeyCode::Right, _) => {
                    if !ui.shown.is_empty() {
                        ui.selected = Some(ui.shown.len() - 1);
                    }
                }
                // Go down one item.
                // If we're at the bottom, back to the top.
                (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
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
                            let apps_panel_height = terminal.size()?.height - (terminal.size()?.height as f32 * cli.title_panel_height_percent as f32 / 100.0).round() as u16 - cli.input_panel_height - 2;
                            let max_visible = apps_panel_height.saturating_sub(2) as usize;
                            
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
                }
                // Go up one item.
                // If we're at the top, go to the end.
                (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
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
                            let apps_panel_height = terminal.size()?.height - (terminal.size()?.height as f32 * cli.title_panel_height_percent as f32 / 100.0).round() as u16 - cli.input_panel_height - 2;
                            let max_visible = apps_panel_height.saturating_sub(2) as usize;
                            
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
                }
                _ => {}
            }

            ui.info(cli.highlight_color, cli.fancy_mode);
            }
            Event::Mouse(mouse_event) => {
                let mouse_row = mouse_event.row;
                
                // Get the layout areas to determine the apps panel area
                let total_height = terminal.size()?.height;
                let title_height = (total_height as f32 * cli.title_panel_height_percent as f32 / 100.0).round() as u16;
                let input_height = cli.input_panel_height;
                
                // Calculate the apps panel area
                let apps_panel_start = title_height + 1; // +1 for border
                let apps_panel_height = total_height - title_height - input_height - 2; // -2 for borders
                let apps_panel_end = apps_panel_start + apps_panel_height;
                
                // Update selection based on current mouse position
                let update_selection_for_mouse_pos = |ui: &mut UI, mouse_row: u16| {
                    if !ui.shown.is_empty() && mouse_row >= apps_panel_start + 1 && mouse_row < apps_panel_end - 1 {
                        let row_in_panel = mouse_row - apps_panel_start - 1;
                        let max_visible = apps_panel_height.saturating_sub(2) as u16;
                        
                        if row_in_panel < max_visible {
                            let hovered_app_index = ui.scroll_offset + row_in_panel as usize;
                            if hovered_app_index < ui.shown.len() {
                                ui.selected = Some(hovered_app_index);
                                ui.info(cli.highlight_color, cli.fancy_mode);
                            }
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
                        // Check if click is within the apps panel content area
                        if mouse_row >= apps_panel_start + 1 && mouse_row < apps_panel_end - 1 && !ui.shown.is_empty() {
                            // Calculate which visible index was clicked (fixed offset calculation)
                            let row_in_panel = mouse_row - apps_panel_start - 1; // Subtract border
                            let max_visible = apps_panel_height.saturating_sub(2) as u16; // -2 for borders
                            
                            if row_in_panel < max_visible {
                                // Convert to absolute index by adding scroll offset
                                let clicked_app_index = ui.scroll_offset + row_in_panel as usize;
                                
                                // Check if the clicked index is valid
                                if clicked_app_index < ui.shown.len() {
                                    ui.selected = Some(clicked_app_index);
                                    ui.info(cli.highlight_color, cli.fancy_mode);
                                    break; // Launch the clicked app
                                }
                            }
                        }
                    }
                    // Handle scroll wheel for scrolling the list
                    MouseEventKind::ScrollUp => {
                        if !ui.shown.is_empty() && ui.scroll_offset > 0 {
                            ui.scroll_offset -= 1;
                            // Update selection to match current mouse position after scrolling
                            update_selection_for_mouse_pos(&mut ui, mouse_row);
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        if !ui.shown.is_empty() {
                            // Calculate maximum visible items in the apps panel
                            let apps_panel_height = total_height - title_height - input_height - 2; // -2 for borders
                            let max_visible = apps_panel_height.saturating_sub(2) as usize; // -2 for top/bottom borders
                            
                            // Only scroll down if there are more items to show
                            if ui.scroll_offset + max_visible < ui.shown.len() {
                                ui.scroll_offset += 1;
                                // Update selection to match current mouse position after scrolling
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

        // Split command in a shell-parseable format.
        let commands = shell_words::split(&app_to_run.command)?;

        // Switch to path specified by app to be run
        if let Some(path) = &app_to_run.path {
            env::set_current_dir(path::PathBuf::from(path)).wrap_err_with(|| {
                format!("Failed to switch to {path} when starting {app_to_run}")
            })?;
        }

        // Actual commands being run
        let mut runner: Vec<&str> = vec![];

        // Determine launch method based on flags (priority: uwsm > systemd-run > sway > default)
        if cli.uwsm {
            // Use uwsm to launch the app
            runner.extend_from_slice(&["uwsm", "app", "--"]);
        } else if cli.systemd_run {
            // Use systemd-run with user scope
            runner.extend_from_slice(&["systemd-run", "--user", "--scope", "--"]);
        } else if cli.sway {
            // Use swaymsg to run the command (allows Sway to move app to current workspace)
            runner.extend_from_slice(&["swaymsg", "exec", "--"]);
        }

        // Use terminal runner to run the app.
        if app_to_run.is_terminal {
            runner.extend_from_slice(&cli.terminal_launcher.split(' ').collect::<Vec<&str>>());
        }

        // Add app commands
        runner.extend_from_slice(&commands.iter().map(AsRef::as_ref).collect::<Vec<&str>>());

        let mut exec = process::Command::new(runner[0]);
        exec.args(&runner[1..]);

        // Set program as session leader.
        // Otherwise the OS may kill the app after the Gyr exits.
        //
        // # Safety: pre_exec() isn't modifyng the memory and setsid() fails if the calling
        // process is already a process group leader (which isn't)
        #[allow(unsafe_code)]
        unsafe {
            exec.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }

        if cli.verbose.unwrap_or(0) > 0 {
            exec.stdin(process::Stdio::null())
                .stdout(process::Stdio::null())
                .stderr(process::Stdio::null())
                .spawn()
                .wrap_err_with(|| format!("Failed to run {exec:?}"))?;
        } else {
            exec.spawn()
                .wrap_err_with(|| format!("Failed to run {exec:?}"))?;
        }

        {
            let value = app_to_run.history + 1;
            let packed = bytes::pack(value);
            db.insert(app_to_run.name.as_bytes(), &packed).unwrap();
        }
    }

    // Lock file cleanup is handled by LockGuard
    Ok(())
}

/// Byte packer and unpacker
mod bytes {
    /// Unacks an `[u8; 8]` array into a single `u64`, previously packed with [pack]
    ///
    /// [pack]: pack
    pub const fn unpack(buffer: [u8; 8]) -> u64 {
        let mut data = 0u64;
        data |= buffer[0] as u64;
        data |= (buffer[1] as u64) << 8;
        data |= (buffer[2] as u64) << 16;
        data |= (buffer[3] as u64) << 24;
        data |= (buffer[4] as u64) << 32;
        data |= (buffer[5] as u64) << 40;
        data |= (buffer[6] as u64) << 48;
        data |= (buffer[7] as u64) << 56;
        data
    }

    /// Packs an `u64` into a `[u8; 8]` array.
    ///
    /// Can be unpacked with [unpack].
    ///
    /// [unpack]: unpack
    pub const fn pack(data: u64) -> [u8; 8] {
        let mut buffer = [0u8; 8];
        buffer[0] = (data & 0xFF) as u8;
        buffer[1] = ((data >> 8) & 0xFF) as u8;
        buffer[2] = ((data >> 16) & 0xFF) as u8;
        buffer[3] = ((data >> 24) & 0xFF) as u8;
        buffer[4] = ((data >> 32) & 0xFF) as u8;
        buffer[5] = ((data >> 40) & 0xFF) as u8;
        buffer[6] = ((data >> 48) & 0xFF) as u8;
        buffer[7] = ((data >> 56) & 0xFF) as u8;
        buffer
    }
}
