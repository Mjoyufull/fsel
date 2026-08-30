use crate::cli::Opts;
use crate::core::state::State;
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

pub(super) fn render(frame: &mut Frame, state: &State, cli: &Opts, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    match cli.input_panel_style {
        super::InputPanelStyle::Classic => render_classic(frame, state, cli, area),
        super::InputPanelStyle::Command => render_command(frame, state, cli, area),
    }
}

fn render_classic(frame: &mut Frame, state: &State, cli: &Opts, area: Rect) {
    let block = input_block(cli);
    let available_width = usize::from(block.inner(area).width);
    let line = query_line(state, cli, cli.show_input_count);
    let scroll_x = horizontal_scroll(&line, available_width);
    let input = Paragraph::new(line)
        .block(block)
        .style(
            Style::default()
                .fg(cli.input_text_color)
                .bg(cli.input_background_color),
        )
        .scroll((0, scroll_x));
    frame.render_widget(input, area);
}

fn render_command(frame: &mut Frame, state: &State, cli: &Opts, area: Rect) {
    let block = input_block(cli);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let (header, footer) = command_areas(inner);
    render_command_query(frame, state, cli, header);
    if let Some(footer) = footer {
        render_command_footer(frame, state, cli, footer);
    }
}

fn render_command_query(frame: &mut Frame, state: &State, cli: &Opts, header: Rect) {
    let rail_width = u16::from(header.width > 2);
    if rail_width > 0 {
        for y in header.y..header.y + header.height {
            frame.render_widget(
                Paragraph::new("▎").style(
                    Style::default()
                        .fg(cli.highlight_color)
                        .bg(cli.input_background_color),
                ),
                Rect::new(header.x, y, 1, 1),
            );
        }
    }

    let query_area = Rect::new(
        header.x.saturating_add(rail_width.saturating_mul(2)),
        header.y + header.height.saturating_sub(1) / 2,
        header.width.saturating_sub(rail_width.saturating_mul(2)),
        u16::from(header.height > 0),
    );
    let line = query_line(state, cli, false);
    let scroll_x = horizontal_scroll(&line, usize::from(query_area.width));
    frame.render_widget(
        Paragraph::new(line)
            .style(
                Style::default()
                    .fg(cli.input_text_color)
                    .bg(cli.input_background_color),
            )
            .scroll((0, scroll_x)),
        query_area,
    );
}

fn render_command_footer(frame: &mut Frame, state: &State, cli: &Opts, footer: Rect) {
    let selected_name = state
        .selected
        .and_then(|selected| state.shown.get(selected))
        .map_or("fsel", |app| app.name.as_str());
    let count = state.selected.map_or(0, |selected| selected + 1);
    let right = if cli.show_input_count {
        format!("{count}/{}  enter launch  esc close", state.shown.len())
    } else {
        "enter launch  esc close".to_string()
    };
    let right_width = right.chars().count();
    if usize::from(footer.width) > right_width.saturating_add(2) {
        frame.render_widget(
            Paragraph::new(selected_name).style(
                Style::default()
                    .fg(cli.input_text_color)
                    .bg(cli.input_background_color),
            ),
            footer,
        );
    }
    frame.render_widget(
        Paragraph::new(right).alignment(Alignment::Right).style(
            Style::default()
                .fg(cli.input_text_color)
                .bg(cli.input_background_color),
        ),
        footer,
    );
}

fn input_block(cli: &Opts) -> Block<'static> {
    let mut block = Block::default()
        .borders(if cli.show_input_border {
            Borders::ALL
        } else {
            Borders::NONE
        })
        .style(Style::default().bg(cli.input_background_color))
        .border_style(Style::default().fg(cli.input_border_color))
        .border_type(if cli.rounded_borders {
            BorderType::Rounded
        } else {
            BorderType::Plain
        });
    if cli.show_panel_titles {
        block = block.title(Span::styled(
            " Input ",
            Style::default().fg(cli.header_title_color),
        ));
    }
    block
}

fn query_line<'a>(state: &'a State, cli: &'a Opts, inline_count: bool) -> Line<'a> {
    let mut spans = Vec::new();
    if inline_count {
        spans.extend([
            Span::styled("(", Style::default().fg(cli.input_text_color)),
            Span::styled(
                state
                    .selected
                    .map_or(0, |selected| selected + 1)
                    .to_string(),
                Style::default().fg(cli.highlight_color),
            ),
            Span::styled("/", Style::default().fg(cli.input_text_color)),
            Span::styled(
                state.shown.len().to_string(),
                Style::default().fg(cli.input_text_color),
            ),
            Span::styled(") ", Style::default().fg(cli.input_text_color)),
        ]);
    }
    if cli.show_input_prompt {
        spans.extend([
            Span::styled(">", Style::default().fg(cli.highlight_color)),
            Span::styled("> ", Style::default().fg(cli.input_text_color)),
        ]);
    }
    spans.extend([
        Span::styled(&state.query, Style::default().fg(cli.input_text_color)),
        Span::styled(&cli.cursor, Style::default().fg(cli.highlight_color)),
    ]);
    Line::from(spans)
}

fn horizontal_scroll(line: &Line<'_>, available_width: usize) -> u16 {
    line.width()
        .saturating_sub(available_width)
        .min(usize::from(u16::MAX)) as u16
}

fn command_areas(inner: Rect) -> (Rect, Option<Rect>) {
    if inner.height < 2 {
        return (inner, None);
    }
    (
        Rect::new(inner.x, inner.y, inner.width, inner.height - 1),
        Some(Rect::new(
            inner.x,
            inner.y + inner.height - 1,
            inner.width,
            1,
        )),
    )
}

#[cfg(test)]
mod tests {
    use super::{command_areas, horizontal_scroll, render};
    use crate::cli::{MatchMode, Opts, PinnedOrderMode, RankingMode};
    use crate::core::state::State;
    use crate::ui::InputPanelStyle;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;
    use ratatui::text::Line;

    #[test]
    fn command_style_reserves_the_last_inner_row_for_status() {
        assert_eq!(
            command_areas(Rect::new(2, 4, 20, 3)),
            (Rect::new(2, 4, 20, 2), Some(Rect::new(2, 6, 20, 1)))
        );
        assert_eq!(
            command_areas(Rect::new(2, 4, 20, 1)),
            (Rect::new(2, 4, 20, 1), None)
        );
    }

    #[test]
    fn input_scroll_is_saturating() {
        assert_eq!(horizontal_scroll(&Line::from("abcdef"), 4), 2);
        assert_eq!(horizontal_scroll(&Line::from("abc"), 4), 0);
    }

    #[test]
    fn command_style_renders_an_accent_query_and_footer() {
        let mut state = State::new(
            Vec::new(),
            MatchMode::Fuzzy,
            Default::default(),
            3,
            RankingMode::Frecency,
            PinnedOrderMode::Ranking,
            Default::default(),
        );
        state.query = "fire".to_string();
        let cli = Opts {
            input_panel_style: InputPanelStyle::Command,
            show_input_border: false,
            show_panel_titles: false,
            show_input_prompt: false,
            ..Opts::default()
        };
        let backend = TestBackend::new(40, 3);
        let mut terminal = Terminal::new(backend).expect("test terminal should initialize");

        terminal
            .draw(|frame| render(frame, &state, &cli, frame.area()))
            .expect("command input should render");
        let buffer = terminal.backend().buffer();

        assert_eq!(buffer[(0, 0)].symbol(), "▎");
        assert_eq!(buffer[(2, 0)].symbol(), "f");
        assert_eq!(buffer[(0, 2)].symbol(), "f");
        let footer =
            (0..40)
                .map(|x| buffer[(x, 2)].symbol())
                .fold(String::new(), |mut line, symbol| {
                    line.push_str(symbol);
                    line
                });
        assert!(footer.contains("enter launch"));
    }
}
