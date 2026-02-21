//! Dmenu compatibility mode

use crate::cli::Opts;
use crate::ui::{DmenuUI, InputConfig, InputEvent as Event};
use eyre::{Result, WrapErr};

use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use scopeguard::defer;
use std::io;

/// Run dmenu mode
pub fn run(cli: &Opts) -> Result<()> {
    use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
    use crossterm::{
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        ExecutableCommand,
    };
    use ratatui::backend::CrosstermBackend;
    use ratatui::layout::{Alignment, Constraint, Direction, Layout};
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{
        Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap,
    };
    use ratatui::Terminal;

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

    // Setup terminal
    enable_raw_mode().wrap_err("Failed to enable raw mode")?;
    io::stderr()
        .execute(EnterAlternateScreen)
        .wrap_err("Failed to enter alternate screen")?;

    // Get effective disable_mouse setting with dmenu -> regular inheritance
    let disable_mouse = cli.dmenu_disable_mouse.unwrap_or(cli.disable_mouse);
    if !disable_mouse {
        io::stderr()
            .execute(EnableMouseCapture)
            .wrap_err("Failed to enable mouse capture")?;
    }

    // Ensure cleanup on exit
    defer! {
        if !disable_mouse {
            let _ = io::stderr().execute(DisableMouseCapture);
        }
        let _ = io::stderr().execute(LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }

    // Initialize terminal using stderr to keep stdout clean for dmenu output
    let backend = CrosstermBackend::new(io::stderr());
    let mut terminal = Terminal::new(backend).wrap_err("Failed to start crossterm terminal")?;
    terminal.hide_cursor().wrap_err("Failed to hide cursor")?;
    terminal.clear().wrap_err("Failed to clear terminal")?;

    // Input handler - use a key that won't interfere with our escape handling
    let input = InputConfig {
        disable_mouse,
        exit_key: KeyCode::Null, // Use Null key to prevent accidental input thread termination
        ..InputConfig::default()
    }
    .init();

    // Create dmenu UI
    let mut ui = DmenuUI::new(
        items,
        cli.dmenu_wrap_long_lines,
        cli.dmenu_show_line_numbers,
    );
    ui.set_match_mode(cli.match_mode);
    ui.set_match_nth(cli.dmenu_match_nth.clone());

    // Pre-fill search if -ss was provided
    if let Some(ref search) = cli.search_string {
        ui.query = search.clone();
    }

    ui.filter(); // Initial filter to show all items (or filtered by search_string)

    // Handle pre-selection
    if let Some(ref select_str) = cli.dmenu_select {
        // Find first matching item (case-insensitive)
        let select_lower = select_str.to_lowercase();
        for (idx, item) in ui.shown.iter().enumerate() {
            if item.display_text.to_lowercase().contains(&select_lower) {
                ui.selected = Some(idx);
                break;
            }
        }
    } else if let Some(select_idx) = cli.dmenu_select_index {
        if select_idx < ui.shown.len() {
            ui.selected = Some(select_idx);
        }
    }

    // Ensure we have a valid selection if there are items
    if !ui.shown.is_empty() && ui.selected.is_none() {
        ui.selected = Some(0);
    }

    ui.info(cli.dmenu_highlight_color.unwrap_or(cli.highlight_color));

    // List state for ratatui
    let mut list_state = ListState::default();

    // Get effective dmenu colors with fallback
    let get_dmenu_color = |dmenu_opt: Option<ratatui::style::Color>,
                           default: ratatui::style::Color| {
        dmenu_opt.unwrap_or(default)
    };
    let get_dmenu_bool = |dmenu_opt: Option<bool>, default: bool| dmenu_opt.unwrap_or(default);
    let get_dmenu_u16 = |dmenu_opt: Option<u16>, default: u16| dmenu_opt.unwrap_or(default);
    let get_dmenu_panel_position =
        |dmenu_opt: Option<crate::cli::PanelPosition>, default: crate::cli::PanelPosition| {
            dmenu_opt.unwrap_or(default)
        };
    // Get effective cursor string
    let cursor = cli.dmenu_cursor.as_ref().unwrap_or(&cli.cursor);

    // Main TUI loop
    loop {
        // For Foot: use synchronized updates (DECSET 2026) to avoid mid-frame tearing
        let term_is_foot = std::env::var("TERM")
            .unwrap_or_default()
            .starts_with("foot");
        if term_is_foot {
            let mut stderr = std::io::stderr();
            let _ = std::io::Write::write_all(&mut stderr, b"\x1b[?2026h");
            let _ = std::io::Write::flush(&mut stderr);
        }

        terminal.draw(|f| {
            // Get effective colors and settings for dmenu mode
            let highlight_color = get_dmenu_color(cli.dmenu_highlight_color, cli.highlight_color);
            let main_border_color =
                get_dmenu_color(cli.dmenu_main_border_color, cli.main_border_color);
            let items_border_color =
                get_dmenu_color(cli.dmenu_items_border_color, cli.apps_border_color);
            let input_border_color =
                get_dmenu_color(cli.dmenu_input_border_color, cli.input_border_color);
            let main_text_color = get_dmenu_color(cli.dmenu_main_text_color, cli.main_text_color);
            let items_text_color = get_dmenu_color(cli.dmenu_items_text_color, cli.apps_text_color);
            let input_text_color =
                get_dmenu_color(cli.dmenu_input_text_color, cli.input_text_color);
            let header_title_color =
                get_dmenu_color(cli.dmenu_header_title_color, cli.header_title_color);
            let rounded_borders = get_dmenu_bool(cli.dmenu_rounded_borders, cli.rounded_borders);
            let content_panel_height = get_dmenu_u16(
                cli.dmenu_title_panel_height_percent,
                cli.title_panel_height_percent,
            );
            let input_panel_height =
                get_dmenu_u16(cli.dmenu_input_panel_height, cli.input_panel_height);

            // Layout calculation
            let total_height = f.area().height;
            let content_height =
                (total_height as f32 * content_panel_height as f32 / 100.0).round() as u16;

            // Get content panel position (defaults to Top if not set)
            let content_panel_position = get_dmenu_panel_position(
                cli.dmenu_title_panel_position,
                cli.title_panel_position
                    .unwrap_or(crate::cli::PanelPosition::Top),
            );

            // Split the window into three parts based on content panel position
            let (chunks, content_panel_index, items_panel_index, input_panel_index) =
                match content_panel_position {
                    crate::cli::PanelPosition::Top => {
                        // Top: content, items, input (original layout)
                        let layout = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Length(content_height.max(3)),
                                Constraint::Min(1),
                                Constraint::Length(input_panel_height),
                            ])
                            .split(f.area());
                        (layout, 0, 1, 2)
                    }
                    crate::cli::PanelPosition::Middle => {
                        // Middle: items, content, input
                        let layout = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Min(1),
                                Constraint::Length(content_height.max(3)),
                                Constraint::Length(input_panel_height),
                            ])
                            .split(f.area());
                        (layout, 1, 0, 2)
                    }
                    crate::cli::PanelPosition::Bottom => {
                        // Bottom: items, input, content
                        let layout = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Min(1),                        // Items panel (remaining space)
                                Constraint::Length(input_panel_height),    // Input panel
                                Constraint::Length(content_height.max(3)), // Content panel at bottom
                            ])
                            .split(f.area());
                        (layout, 2, 0, 1)
                    }
                };

            // Border type
            let border_type = if rounded_borders {
                BorderType::Rounded
            } else {
                BorderType::Plain
            };

            // Content panel (shows selected item's full content)
            let content_block = Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    " Content ",
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(header_title_color),
                ))
                .border_type(border_type)
                .border_style(Style::default().fg(main_border_color));

            let content_paragraph = Paragraph::new(ui.text.clone())
                .block(content_block)
                .style(Style::default().fg(main_text_color))
                .wrap(Wrap { trim: false })
                .alignment(Alignment::Left);

            // Items panel
            let items_panel_height = chunks[items_panel_index].height;
            let max_visible = items_panel_height.saturating_sub(2) as usize;

            let visible_items = ui
                .shown
                .iter()
                .skip(ui.scroll_offset)
                .take(max_visible)
                .map(ListItem::from)
                .collect::<Vec<ListItem>>();

            let items_list = List::new(visible_items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(Span::styled(
                            " Items ",
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(header_title_color),
                        ))
                        .border_type(border_type)
                        .border_style(Style::default().fg(items_border_color)),
                )
                .style(Style::default().fg(items_text_color))
                .highlight_style(
                    Style::default()
                        .fg(highlight_color)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("> ");

            // Update list state selection
            let visible_selection = ui.selected.and_then(|sel| {
                if sel >= ui.scroll_offset && sel < ui.scroll_offset + max_visible {
                    Some(sel - ui.scroll_offset)
                } else {
                    None
                }
            });
            list_state.select(visible_selection);

            // Input panel
            let input_paragraph = Paragraph::new(Line::from(vec![
                Span::styled("(", Style::default().fg(input_text_color)),
                Span::styled(
                    (ui.selected.map_or(0, |v| v + 1)).to_string(),
                    Style::default().fg(highlight_color),
                ),
                Span::styled("/", Style::default().fg(input_text_color)),
                Span::styled(
                    ui.shown.len().to_string(),
                    Style::default().fg(input_text_color),
                ),
                Span::styled(") ", Style::default().fg(input_text_color)),
                Span::styled(">", Style::default().fg(highlight_color)),
                Span::styled("> ", Style::default().fg(input_text_color)),
                Span::styled(
                    if cli.dmenu_password_mode {
                        cli.dmenu_password_character.repeat(ui.query.len())
                    } else {
                        ui.query.clone()
                    },
                    Style::default().fg(input_text_color),
                ),
                Span::styled(cursor, Style::default().fg(highlight_color)),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        if cli.dmenu_prompt_only {
                            " Input "
                        } else {
                            " Filter "
                        },
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(header_title_color),
                    ))
                    .border_type(border_type)
                    .border_style(Style::default().fg(input_border_color)),
            )
            .style(Style::default().fg(input_text_color))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false });

            // Clear all widget areas FIRST to remove any old content
            // ONLY for Kitty - Foot/Sixel terminals auto-refresh and clearing causes flashing
            use ratatui::widgets::Clear;
            let graphics = crate::ui::GraphicsAdapter::detect();
            if matches!(graphics, crate::ui::GraphicsAdapter::Kitty) {
                f.render_widget(Clear, chunks[content_panel_index]);
                f.render_widget(Clear, chunks[items_panel_index]);
                f.render_widget(Clear, chunks[input_panel_index]);
            }

            // NOW render all components in their dynamic positions
            // Only render content panel if not hide_before_typing or query is not empty
            if !cli.dmenu_hide_before_typing || !ui.query.is_empty() {
                f.render_widget(content_paragraph, chunks[content_panel_index]);
            }
            // Only render items list if not in prompt-only mode and (not hide_before_typing or query is not empty)
            if !cli.dmenu_prompt_only && (!cli.dmenu_hide_before_typing || !ui.query.is_empty()) {
                f.render_stateful_widget(items_list, chunks[items_panel_index], &mut list_state);
            }
            f.render_widget(input_paragraph, chunks[input_panel_index]);
        })?;

        if term_is_foot {
            let mut stderr = std::io::stderr();
            let _ = std::io::Write::write_all(&mut stderr, b"\x1b[?2026l");
            let _ = std::io::Write::flush(&mut stderr);
        }

        // Handle input events
        match input.next()? {
            Event::Input(key) => {
                match (key.code, key.modifiers) {
                    // Exit on escape or Ctrl+C/Q
                    (KeyCode::Esc, _)
                    | (KeyCode::Char('q'), KeyModifiers::CONTROL)
                    | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        return Ok(()); // Exit without output
                    }
                    // Select item on Enter or Ctrl+Y
                    (KeyCode::Enter, _) | (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                        // Auto-select if only one match and auto_select is enabled
                        if cli.dmenu_auto_select && ui.shown.len() == 1 {
                            ui.selected = Some(0);
                        }

                        // Store selection and exit loop to handle output outside TUI context
                        if let Some(selected) = ui.selected {
                            if selected < ui.shown.len() {
                                let output = if cli.dmenu_index_mode {
                                    // Output index instead of text
                                    selected.to_string()
                                } else if let Some(ref accept_cols) = cli.dmenu_accept_nth {
                                    // Output specific columns
                                    ui.shown[selected].get_accept_nth_output(accept_cols)
                                } else {
                                    // Output original line
                                    ui.shown[selected].original_line.clone()
                                };

                                // Clean up terminal completely
                                terminal.show_cursor().wrap_err("Failed to show cursor")?;
                                drop(terminal);
                                if !disable_mouse {
                                    let _ = io::stderr().execute(DisableMouseCapture);
                                }
                                let _ = io::stderr().execute(LeaveAlternateScreen);
                                let _ = disable_raw_mode();

                                // Print to stdout
                                println!("{}", output);
                                return Ok(());
                            }
                        } else if !cli.dmenu_only_match && !ui.query.is_empty() {
                            // No selection but have query - output the query itself (unless only_match is set)
                            terminal.show_cursor().wrap_err("Failed to show cursor")?;
                            drop(terminal);
                            if !disable_mouse {
                                let _ = io::stderr().execute(DisableMouseCapture);
                            }
                            let _ = io::stderr().execute(LeaveAlternateScreen);
                            let _ = disable_raw_mode();

                            println!("{}", ui.query);
                            return Ok(());
                        }

                        // only_match is set and no selection - don't exit
                        if cli.dmenu_only_match {
                            continue;
                        }

                        return Ok(()); // Exit without selection
                    }
                    // Add character to query
                    (KeyCode::Char(c), KeyModifiers::NONE)
                    | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                        ui.query.push(c);
                        ui.filter();

                        // Auto-select if only one match
                        if cli.dmenu_auto_select && ui.shown.len() == 1 {
                            ui.selected = Some(0);
                        }
                    }
                    // Remove character from query
                    (KeyCode::Backspace, _) => {
                        ui.query.pop();
                        ui.filter();

                        // Auto-select if only one match
                        if cli.dmenu_auto_select && ui.shown.len() == 1 {
                            ui.selected = Some(0);
                        }
                    }
                    // Navigation
                    (KeyCode::Left, _) => {
                        if !ui.shown.is_empty() {
                            ui.selected = Some(0);
                            ui.scroll_offset = 0;
                        }
                    }
                    (KeyCode::Right, _) => {
                        if !ui.shown.is_empty() {
                            let last_index = ui.shown.len() - 1;
                            ui.selected = Some(last_index);

                            // Scroll to show last item
                            let total_height = terminal.size()?.height;
                            let content_panel_height = get_dmenu_u16(
                                cli.dmenu_title_panel_height_percent,
                                cli.title_panel_height_percent,
                            );
                            let input_panel_height =
                                get_dmenu_u16(cli.dmenu_input_panel_height, cli.input_panel_height);

                            // Use same calculation as rendering code
                            let content_height = (total_height as f32 * content_panel_height as f32
                                / 100.0)
                                .round() as u16;
                            let content_height = content_height.max(3);
                            let items_panel_height =
                                total_height - content_height - input_panel_height;
                            let max_visible = items_panel_height.saturating_sub(2) as usize;

                            if max_visible > 0 && ui.shown.len() > max_visible {
                                ui.scroll_offset = ui.shown.len().saturating_sub(max_visible);
                            } else {
                                ui.scroll_offset = 0;
                            }
                        }
                    }
                    (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                        if let Some(selected) = ui.selected {
                            let hard_stop = get_dmenu_bool(cli.dmenu_hard_stop, cli.hard_stop);
                            ui.selected = if selected < ui.shown.len() - 1 {
                                Some(selected + 1)
                            } else if !hard_stop {
                                Some(0)
                            } else {
                                Some(selected)
                            };

                            // Auto-scroll to keep selection visible
                            if let Some(new_selected) = ui.selected {
                                let total_height = terminal.size()?.height;
                                let content_panel_height = get_dmenu_u16(
                                    cli.dmenu_title_panel_height_percent,
                                    cli.title_panel_height_percent,
                                );
                                let input_panel_height = get_dmenu_u16(
                                    cli.dmenu_input_panel_height,
                                    cli.input_panel_height,
                                );

                                // Use same calculation as rendering code
                                let content_height =
                                    (total_height as f32 * content_panel_height as f32 / 100.0)
                                        .round() as u16;
                                let content_height = content_height.max(3);
                                let items_panel_height =
                                    total_height - content_height - input_panel_height;
                                let max_visible = items_panel_height.saturating_sub(2) as usize; // -2 for borders

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
                    (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                        if let Some(selected) = ui.selected {
                            let hard_stop = get_dmenu_bool(cli.dmenu_hard_stop, cli.hard_stop);
                            ui.selected = if selected > 0 {
                                Some(selected - 1)
                            } else if !hard_stop {
                                Some(ui.shown.len() - 1)
                            } else {
                                Some(selected)
                            };

                            // Auto-scroll to keep selection visible
                            if let Some(new_selected) = ui.selected {
                                let total_height = terminal.size()?.height;
                                let content_panel_height = get_dmenu_u16(
                                    cli.dmenu_title_panel_height_percent,
                                    cli.title_panel_height_percent,
                                );
                                let input_panel_height = get_dmenu_u16(
                                    cli.dmenu_input_panel_height,
                                    cli.input_panel_height,
                                );

                                // Use same calculation as rendering code
                                let content_height =
                                    (total_height as f32 * content_panel_height as f32 / 100.0)
                                        .round() as u16;
                                let content_height = content_height.max(3);
                                let items_panel_height =
                                    total_height - content_height - input_panel_height;
                                let max_visible = items_panel_height.saturating_sub(2) as usize; // -2 for borders

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

                // Update info display
                ui.info(get_dmenu_color(
                    cli.dmenu_highlight_color,
                    cli.highlight_color,
                ));
            }
            Event::Mouse(mouse_event) => {
                // Dmenu-specific mouse handling with proper layout calculations
                let mouse_row = mouse_event.row;
                let total_height = terminal.size()?.height;
                let content_panel_height = get_dmenu_u16(
                    cli.dmenu_title_panel_height_percent,
                    cli.title_panel_height_percent,
                );
                let input_panel_height =
                    get_dmenu_u16(cli.dmenu_input_panel_height, cli.input_panel_height);

                // Use same calculation as rendering code
                let content_height =
                    (total_height as f32 * content_panel_height as f32 / 100.0).round() as u16;
                let content_height = content_height.max(3);
                let items_panel_height = total_height - content_height - input_panel_height;

                // Get content panel position to calculate items panel position
                let content_panel_position = get_dmenu_panel_position(
                    cli.dmenu_title_panel_position,
                    cli.title_panel_position
                        .unwrap_or(crate::cli::PanelPosition::Top),
                );

                // Calculate items panel coordinates based on layout
                let (items_panel_start, items_panel_height) = match content_panel_position {
                    crate::cli::PanelPosition::Top => {
                        // Top: content, items, input - items start after content
                        (content_height, items_panel_height)
                    }
                    crate::cli::PanelPosition::Middle => {
                        // Middle: items, content, input - items start at top
                        (0, items_panel_height)
                    }
                    crate::cli::PanelPosition::Bottom => {
                        // Bottom: items, input, content - items start at top
                        (0, items_panel_height)
                    }
                };

                let items_content_start = items_panel_start + 1; // +1 for top border
                let max_visible_rows = items_panel_height.saturating_sub(2); // -2 for borders
                let items_content_end = items_content_start + max_visible_rows;

                let update_selection_for_mouse_pos = |ui: &mut DmenuUI, mouse_row: u16| {
                    if !ui.shown.is_empty()
                        && mouse_row >= items_content_start
                        && mouse_row < items_content_end
                    {
                        let row_in_content = mouse_row - items_content_start;
                        let hovered_item_index = ui.scroll_offset + row_in_content as usize;
                        if hovered_item_index < ui.shown.len() {
                            ui.selected = Some(hovered_item_index);
                            ui.info(get_dmenu_color(
                                cli.dmenu_highlight_color,
                                cli.highlight_color,
                            ));
                        }
                    }
                };

                match mouse_event.kind {
                    MouseEventKind::Moved => {
                        update_selection_for_mouse_pos(&mut ui, mouse_row);
                    }
                    MouseEventKind::Down(MouseButton::Left) => {
                        if mouse_row >= items_content_start
                            && mouse_row < items_content_end
                            && !ui.shown.is_empty()
                        {
                            let row_in_content = mouse_row - items_content_start;
                            let clicked_item_index = ui.scroll_offset + row_in_content as usize;

                            if clicked_item_index < ui.shown.len() {
                                // Store the original line as-is for dmenu output
                                let selected_line = &ui.shown[clicked_item_index].original_line;

                                // Clean up terminal completely
                                terminal.show_cursor().wrap_err("Failed to show cursor")?;
                                drop(terminal); // Ensure terminal is fully cleaned up
                                if !disable_mouse {
                                    let _ = io::stderr().execute(DisableMouseCapture);
                                }
                                let _ = io::stderr().execute(LeaveAlternateScreen);
                                let _ = disable_raw_mode();

                                // Output selection in clean context
                                println!("{}", selected_line);
                                return Ok(());
                            }
                        }
                    }
                    MouseEventKind::ScrollUp => {
                        if !ui.shown.is_empty() && ui.scroll_offset > 0 {
                            ui.scroll_offset -= 1;
                            // Update selection to match current mouse position after scrolling
                            update_selection_for_mouse_pos(&mut ui, mouse_row);
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        if !ui.shown.is_empty() {
                            // Calculate maximum visible items (account for borders)
                            let max_visible = max_visible_rows as usize;

                            // Only scroll down if there are more items to show
                            if ui.scroll_offset + max_visible < ui.shown.len() {
                                ui.scroll_offset += 1;
                                // Update selection to match current mouse position after scrolling
                                update_selection_for_mouse_pos(&mut ui, mouse_row);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::Tick => {}
            Event::Render => {} // Handled by draw loop
        }
    }
}
