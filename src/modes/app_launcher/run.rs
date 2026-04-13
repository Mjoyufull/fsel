//! Application launcher mode

use crate::cli::Opts;
use crate::core::ranking::{current_unix_seconds, sort_by_ranking};
use crate::core::state::State;
use crate::ui::{InputConfig, InputEvent as Event, UI};
use eyre::{Result, WrapErr};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use scopeguard::defer;
use std::cell::Cell;
use std::io;
use std::time::Duration;

/// Run application launcher mode.
pub async fn run(cli: Opts) -> Result<()> {
    use crossterm::event::KeyCode;

    if let Some(ref program_name) = cli.program
        && program_name.len() >= 2
    {
        return super::direct::launch_program_directly(&cli, program_name);
    }

    crate::ui::terminal::setup_terminal(cli.disable_mouse)?;
    let terminal_active = Cell::new(true);
    defer! {
        if terminal_active.get() {
            let _ = crate::ui::terminal::shutdown_terminal(cli.disable_mouse);
        }
    }

    let data_dir = crate::app::paths::runtime_data_dir()?;
    let history_db_path = crate::app::paths::history_db_path()?;
    let lock_path = crate::app::paths::launcher_lock_path()?;
    let session =
        super::session::LauncherSession::start(&history_db_path, &lock_path, cli.replace)?;
    let db = std::sync::Arc::clone(session.db());

    if super::admin::handle_maintenance_command(&cli, &db, data_dir.as_path())? {
        return Ok(());
    }

    super::admin::initialize_test_mode(&cli);

    let apps_rx = crate::desktop::read_with_options(
        crate::desktop::application_dirs(),
        &db,
        cli.filter_desktop,
        cli.filter_actions,
        cli.list_executables_in_path,
    );

    let mut all_apps = Vec::with_capacity(500);
    while let Ok(app) = apps_rx.recv() {
        all_apps.push(app);
    }

    let frecency_data = crate::core::database::load_frecency(&db);
    let mut pin_timestamps = crate::core::database::load_pin_timestamps(&db);
    sort_by_ranking(
        &mut all_apps,
        &frecency_data,
        cli.ranking_mode,
        cli.pinned_order_mode,
        &pin_timestamps,
        current_unix_seconds(),
    );

    super::admin::log_startup_if_enabled(&cli, all_apps.len(), frecency_data.len());

    let backend = CrosstermBackend::new(io::stderr());
    let mut terminal = Terminal::new(backend).wrap_err("Failed to start crossterm terminal")?;
    terminal.hide_cursor().wrap_err("Failed to hide cursor")?;
    terminal.clear().wrap_err("Failed to clear terminal")?;

    let mut state = State::new(
        all_apps,
        cli.match_mode,
        frecency_data,
        cli.prefix_depth,
        cli.ranking_mode,
        cli.pinned_order_mode,
        std::mem::take(&mut pin_timestamps),
    );

    if let Some(ref search) = cli.search_string {
        state.query = search.clone();
    }

    state.filter();
    state.update_info(
        cli.highlight_color,
        cli.fancy_mode,
        cli.verbose.unwrap_or(0),
    );

    let mut input = InputConfig {
        disable_mouse: cli.disable_mouse,
        tick_rate: Duration::from_millis(16),
        exit_key: KeyCode::Null,
        ..InputConfig::default()
    }
    .init_async();

    loop {
        terminal.draw(|frame| {
            UI::new().render(frame, &state, &cli);
        })?;

        let Some(event) = input.next().await else {
            break;
        };

        if matches!(event, Event::Input(_) | Event::Mouse(_)) {
            let total_height = terminal.size()?.height;
            super::events::handle_event(&mut state, event, &cli, &db, total_height);
        }

        if state.should_exit {
            if crate::cli::DEBUG_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                crate::core::debug_logger::log_session_end();
            }
            break;
        }

        if state.should_launch {
            if let Some(selected_idx) = state.selected
                && let Some(app) = state.shown.get(selected_idx)
            {
                if let Err(error) = crate::core::database::record_access(&db, &app.name) {
                    eprintln!("Failed to record access: {}", error);
                }

                crate::ui::terminal::shutdown_terminal(cli.disable_mouse)?;
                terminal_active.set(false);

                if cli.no_exec {
                    println!("{}", app.command);
                    return Ok(());
                }

                super::launch::launch_app(app, &cli, &db)?;
            }
            break;
        }
    }

    if !state.should_launch {
        crate::ui::terminal::shutdown_terminal(cli.disable_mouse)?;
        terminal_active.set(false);
    }

    if crate::cli::DEBUG_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        crate::core::debug_logger::log_session_end();
    }

    Ok(())
}
