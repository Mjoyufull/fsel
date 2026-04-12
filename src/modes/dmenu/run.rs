//! Dmenu compatibility mode

use crate::cli::Opts;
use crate::ui::{DmenuUI, InputEvent as Event};
use eyre::{Result, WrapErr};

use ratatui::backend::CrosstermBackend;
use ratatui::widgets::ListState;
use std::io;

use super::events::{LoopOutcome, handle_key_event, handle_mouse_event};
use super::options::DmenuOptions;
use super::render::draw_frame;

/// Run dmenu mode
pub fn run(cli: &Opts) -> Result<()> {
    use ratatui::Terminal;
    use ratatui::backend::CrosstermBackend;

    // Check if stdin is piped (unless prompt-only mode)
    if !cli.dmenu_prompt_only && !super::parse::is_stdin_piped() {
        return Err(eyre::eyre!("dmenu mode requires input from stdin"));
    }

    // Read stdin lines
    let lines = if cli.dmenu_prompt_only {
        vec![] // No input in prompt-only mode
    } else if cli.dmenu_null_separated {
        super::parse::read_stdin_null_separated().wrap_err("Failed to read from stdin")?
    } else {
        super::parse::read_stdin_lines().wrap_err("Failed to read from stdin")?
    };

    // Exit immediately if no input and exit_if_empty is set
    if cli.dmenu_exit_if_empty && lines.is_empty() {
        return Ok(());
    }

    // Also check if lines only contain empty strings
    if cli.dmenu_exit_if_empty && lines.iter().all(|l| l.trim().is_empty()) {
        return Ok(());
    }

    // Parse items
    let items = super::parse::parse_stdin_to_items(
        lines,
        &cli.dmenu_delimiter,
        cli.dmenu_with_nth.as_ref(),
    );

    let options = DmenuOptions::from_cli(cli);
    crate::ui::terminal::setup_terminal(options.disable_mouse)?;

    let run_result = (|| -> Result<LoopOutcome> {
        let backend = CrosstermBackend::new(io::stderr());
        let mut terminal = Terminal::new(backend).wrap_err("Failed to start crossterm terminal")?;
        terminal.hide_cursor().wrap_err("Failed to hide cursor")?;
        terminal.clear().wrap_err("Failed to clear terminal")?;

        let input = options.input_config().init();

        let mut ui = build_ui(cli, items, options.highlight_color);
        let mut list_state = ListState::default();

        loop {
            sync_update_mode(options.term_is_foot, true);
            terminal.draw(|frame| draw_frame(frame, &mut ui, &mut list_state, &options))?;
            sync_update_mode(options.term_is_foot, false);

            match input.next()? {
                Event::Input(key) => {
                    match handle_key_event(&mut ui, key, &options, terminal.size()?.height) {
                        LoopOutcome::Continue => {}
                        LoopOutcome::Exit => return Ok(LoopOutcome::Exit),
                        LoopOutcome::Print(output) => {
                            prepare_terminal_for_output(&mut terminal)?;
                            return Ok(LoopOutcome::Print(output));
                        }
                    }
                }
                Event::Mouse(mouse_event) => {
                    match handle_mouse_event(
                        &mut ui,
                        mouse_event,
                        &options,
                        terminal.size()?.height,
                    ) {
                        LoopOutcome::Continue => {}
                        LoopOutcome::Exit => return Ok(LoopOutcome::Exit),
                        LoopOutcome::Print(output) => {
                            prepare_terminal_for_output(&mut terminal)?;
                            return Ok(LoopOutcome::Print(output));
                        }
                    }
                }
                Event::Tick => {}
                Event::Render => {}
            }
        }
    })();

    let shutdown_result = crate::ui::terminal::shutdown_terminal(options.disable_mouse);
    match (run_result, shutdown_result) {
        (Ok(LoopOutcome::Exit), Ok(())) => Ok(()),
        (Ok(LoopOutcome::Print(output)), Ok(())) => {
            println!("{}", output);
            Ok(())
        }
        (Ok(LoopOutcome::Continue), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(error.wrap_err("Failed to restore dmenu terminal state")),
        (Err(error), Err(shutdown_error)) => Err(error.wrap_err(format!(
            "Failed to restore dmenu terminal state: {shutdown_error}"
        ))),
    }
}

fn build_ui<'a>(
    cli: &Opts,
    items: Vec<crate::common::Item>,
    highlight_color: ratatui::style::Color,
) -> DmenuUI<'a> {
    let mut ui = DmenuUI::new(
        items,
        cli.dmenu_wrap_long_lines,
        cli.dmenu_show_line_numbers,
    );
    ui.set_match_mode(cli.match_mode);
    ui.set_match_nth(cli.dmenu_match_nth.clone());

    if let Some(ref search) = cli.search_string {
        ui.query = search.clone();
    }

    ui.filter();

    if let Some(ref select_str) = cli.dmenu_select {
        let select_lower = select_str.to_lowercase();
        for (idx, item) in ui.shown.iter().enumerate() {
            if item.display_text.to_lowercase().contains(&select_lower) {
                ui.selected = Some(idx);
                break;
            }
        }
    } else if let Some(select_idx) = cli.dmenu_select_index
        && select_idx < ui.shown.len()
    {
        ui.selected = Some(select_idx);
    }

    if !ui.shown.is_empty() && ui.selected.is_none() {
        ui.selected = Some(0);
    }

    ui.info(highlight_color);
    ui
}

fn prepare_terminal_for_output(
    terminal: &mut ratatui::Terminal<CrosstermBackend<io::Stderr>>,
) -> Result<()> {
    terminal.show_cursor().wrap_err("Failed to show cursor")?;
    Ok(())
}

fn sync_update_mode(term_is_foot: bool, enable: bool) {
    if !term_is_foot {
        return;
    }

    let sequence = if enable {
        b"\x1b[?2026h"
    } else {
        b"\x1b[?2026l"
    };
    let mut stderr = std::io::stderr();
    let _ = std::io::Write::write_all(&mut stderr, sequence);
    let _ = std::io::Write::flush(&mut stderr);
}
