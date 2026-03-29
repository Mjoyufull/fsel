use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

/// App filtering and sorting UI (Stateless Renderer)
pub struct UI;

impl UI {
    /// Create new stateless UI renderer
    pub fn new() -> Self {
        Self
    }

    /// Render the UI using the centralized State
    pub fn render(
        &self,
        f: &mut Frame,
        state: &crate::core::state::State,
        cli: &crate::cli::Opts,
        image_manager: &mut crate::ui::graphics::ImageManager,
    ) {
        let size = f.area();

        let should_render_border = cli.title_panel_height_percent > 0;

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

        let (title_area, input_area, apps_area) = match cli.title_panel_position {
            Some(crate::ui::PanelPosition::Bottom) => (chunks[2], chunks[1], chunks[0]),
            Some(crate::ui::PanelPosition::Middle) => (chunks[1], chunks[2], chunks[0]),
            // Default: Title (0), Apps (1), Input (2)
            _ => (chunks[0], chunks[2], chunks[1]),
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
                });

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
            });

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

        // Calculate max visible rows (each app is 2 rows tall, subtract borders)
        let max_visible = (apps_area.height.saturating_sub(2) / 2) as usize;

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
            });

        // only render whats on screen, not the whole dang list
        //
        f.render_widget(apps_block, apps_area);
        let inner = apps_area.inner(ratatui::layout::Margin {
            vertical: 1,
            horizontal: 1,
        });

        let visible_apps = state
            .shown
            .iter()
            .skip(state.scroll_offset)
            .take(max_visible);

        for (i, app) in visible_apps.enumerate() {
            let row_rect = Rect::new(inner.x, inner.y + (i as u16 * 2), inner.width, 2);
            let is_selected = state.selected == Some(i + state.scroll_offset);

            // Split row into [Gutter (Selector + Icon), Content (Name)]
            let row_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(6), // 2 for selector, 4 for icon
                    Constraint::Min(0),
                ])
                .split(row_rect);

            let gutter_area = row_chunks[0];
            let name_area = row_chunks[1];

            // Render Selection Symbol
            let symbol = if is_selected { "> " } else { "  " };
            f.render_widget(
                Paragraph::new(symbol).style(Style::default().fg(cli.highlight_color)),
                Rect::new(gutter_area.x, gutter_area.y, 2, 1),
            );

            // Render the Icon
            let icon_rect = Rect::new(gutter_area.x + 2, gutter_area.y, 4, 1);
            if let Some(ref icon_name) = app.icon {
                image_manager.render_at(f, icon_name, icon_rect);
            }

            // Render App Name

            let style = if is_selected {
                Style::default()
                    .fg(cli.highlight_color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(cli.apps_text_color)
            };
            f.render_widget(Paragraph::new(app.name.as_str()).style(style), name_area);
        }
    }
}
