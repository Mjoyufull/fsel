use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::symbols::merge::MergeStrategy;
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

/// App filtering and sorting UI (Stateless Renderer)
pub struct UI;

impl UI {
    /// Create new stateless UI renderer
    pub fn new() -> Self {
        Self
    }

    /// Render the UI using the centralized State
    pub fn render(&self, f: &mut Frame, state: &crate::core::state::State, cli: &crate::cli::Opts) {
        let size = f.area();

        let should_render_border = cli.title_panel_height_percent > 0;
        let merge_strategy = if cli.unify_borders {
            MergeStrategy::Fuzzy
        } else {
            MergeStrategy::Replace
        };

        // Layout calculations
        let chunks = if should_render_border {
            match cli.title_panel_position {
                Some(crate::ui::PanelPosition::Bottom) => Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(0),
                        Constraint::Length(cli.input_panel_height),
                        Constraint::Percentage(cli.title_panel_height_percent),
                    ])
                    .split(size),
                Some(crate::ui::PanelPosition::Middle) => Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(0),
                        Constraint::Percentage(cli.title_panel_height_percent),
                        Constraint::Length(cli.input_panel_height),
                        Constraint::Min(0),
                    ])
                    .split(size),
                _ => Layout::default() // Top default
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(cli.title_panel_height_percent),
                        Constraint::Min(0), // Apps panel
                        Constraint::Length(cli.input_panel_height),
                    ])
                    .split(size),
            }
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(cli.input_panel_height),
                ])
                .split(size)
        };

        let (title_area, input_area, apps_area) = if cli.unify_borders {
            // Overlap adjacent rects by 1 row so merge_borders can collapse shared walls.
            // chunks[0], chunks[1], chunks[2] are top-to-bottom; grow the top two
            // chunks downward by 1 so their bottom border overlaps the next chunk's top.
            let c0 = Rect::new(chunks[0].x, chunks[0].y, chunks[0].width,
                (chunks[0].height + 1).min(size.height.saturating_sub(chunks[0].y)));
            let c1 = Rect::new(chunks[1].x, chunks[1].y, chunks[1].width,
                (chunks[1].height + 1).min(size.height.saturating_sub(chunks[1].y)));
            let c2 = chunks[2];
            match cli.title_panel_position {
                Some(crate::ui::PanelPosition::Bottom) => (c2, c1, c0),
                Some(crate::ui::PanelPosition::Middle) => (c1, c2, c0),
                _ => (c0, c2, c1),
            }
        } else {
            match cli.title_panel_position {
                Some(crate::ui::PanelPosition::Bottom) => (chunks[2], chunks[1], chunks[0]),
                Some(crate::ui::PanelPosition::Middle) => (chunks[1], chunks[2], chunks[0]),
                _ => (chunks[0], chunks[2], chunks[1]),
            }
        };

        // Render Title/Info Panel
        if should_render_border {
            // Determine dynamic title
            let title = if cli.fancy_mode {
                if let Some(selected) = state.selected {
                    state
                        .shown
                        .get(selected)
                        .map(|a| a.name.clone())
                        .unwrap_or_else(|| "Fsel".to_string())
                } else {
                    "Fsel".to_string()
                }
            } else {
                "Fsel".to_string()
            };

            let info_block = Block::default()
                .borders(Borders::ALL)

                .border_style(Style::default().fg(cli.main_border_color))
                .title(Span::styled(
                    format!(" {} ", title),
                    Style::default().fg(cli.header_title_color),
                ))
                .border_type(if cli.rounded_borders {
                    BorderType::Rounded
                } else {
                    BorderType::Plain
                })
                .merge_borders(merge_strategy);

            // Text rendering from state.text which should be populated by state.update_info
            let info_text: Vec<Line> = state.text.lines().map(Line::from).collect();
            let paragraph = Paragraph::new(info_text)
                .block(info_block)
                .style(Style::default().fg(cli.main_text_color));
            f.render_widget(paragraph, title_area);
        }

        // Render Input
        let input_block = Block::default()
            .borders(Borders::ALL)

            .border_style(Style::default().fg(cli.input_border_color))
            .title(Span::styled(
                " Input ",
                Style::default().fg(cli.header_title_color),
            ))
            .border_type(if cli.rounded_borders {
                BorderType::Rounded
            } else {
                BorderType::Plain
            })
            .merge_borders(merge_strategy);

        // Legacy Formatting: (Selected/Total) >> Query
        // Colors:
        // - Brackets/Slash/Text: Input Text Color
        // - Selected Number: Highlight Color
        // - > Cursor: Highlight Color
        // - Cursor Block: Highlight Color

        let spans = vec![
            Span::styled("(", Style::default().fg(cli.input_text_color)),
            Span::styled(
                (state.selected.map_or(0, |v| v + 1)).to_string(),
                Style::default().fg(cli.highlight_color),
            ),
            Span::styled("/", Style::default().fg(cli.input_text_color)),
            Span::styled(
                state.shown.len().to_string(),
                Style::default().fg(cli.input_text_color),
            ),
            Span::styled(") ", Style::default().fg(cli.input_text_color)),
            Span::styled(">", Style::default().fg(cli.highlight_color)),
            Span::styled("> ", Style::default().fg(cli.input_text_color)),
            Span::styled(&state.query, Style::default().fg(cli.input_text_color)),
            Span::styled(&cli.cursor, Style::default().fg(cli.highlight_color)),
        ];

        let line = Line::from(spans);
        let text_len = line.width();

        let available_width = input_area.width.saturating_sub(2) as usize; // Account for borders

        let scroll_x = if text_len > available_width {
            (text_len - available_width) as u16
        } else {
            0
        };

        let input = Paragraph::new(line)
            .block(input_block)
            .style(Style::default().fg(cli.input_text_color))
            .scroll((0, scroll_x));
        f.render_widget(input, input_area);

        // Calculate max visible rows (subtract borders)
        let max_visible = apps_area.height.saturating_sub(2) as usize;

        // Apps block with border
        let apps_block = Block::default()
            .borders(Borders::ALL)

            .border_style(Style::default().fg(cli.apps_border_color))
            .title(Span::styled(
                " Apps ",
                Style::default().fg(cli.header_title_color),
            ))
            .border_type(if cli.rounded_borders {
                BorderType::Rounded
            } else {
                BorderType::Plain
            })
            .merge_borders(merge_strategy);

        // only render whats on screen, not the whole dang list
        let items: Vec<ListItem> = state
            .shown
            .iter()
            .skip(state.scroll_offset)
            .take(max_visible)
            .map(|app| {
                let mut spans = Vec::new();

                // Pin support
                if app.pinned {
                    spans.push(Span::styled(
                        &cli.pin_icon,
                        Style::default().fg(cli.pin_color),
                    ));
                    spans.push(Span::raw(" "));
                }

                spans.push(Span::styled(
                    &app.name,
                    Style::default().fg(cli.apps_text_color),
                ));

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items)
            .block(apps_block)
            .highlight_style(
                Style::default()
                    .fg(cli.highlight_color)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        // gotta adjust for the scroll offset innit
        let mut list_state = ratatui::widgets::ListState::default();
        if let Some(sel) = state.selected {
            // Only highlight if selection is within visible range
            if sel >= state.scroll_offset && sel < state.scroll_offset + max_visible {
                list_state.select(Some(sel - state.scroll_offset));
            }
        }

        f.render_stateful_widget(list, apps_area, &mut list_state);
    }
}
