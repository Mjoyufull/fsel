use super::super::TagMetadataFormatter;
use crate::ui::DmenuUI;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders};

pub(super) fn border_type(rounded_borders: bool) -> BorderType {
    if rounded_borders {
        BorderType::Rounded
    } else {
        BorderType::Plain
    }
}

pub(super) fn panel_block(
    title: &'static str,
    border_type: BorderType,
    title_color: Color,
    border_color: Color,
) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            title,
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(title_color),
        ))
        .border_type(border_type)
        .border_style(Style::default().fg(border_color))
}

pub(super) fn highlight_style(
    ui: &DmenuUI<'_>,
    formatter: &TagMetadataFormatter,
    highlight_color: Color,
) -> Style {
    let tag_color = ui.selected.and_then(|selected| {
        if selected < ui.shown.len() {
            ui.shown[selected]
                .tags
                .as_ref()
                .and_then(|tags| tags.first())
                .and_then(|tag| formatter.get_color(tag))
        } else {
            None
        }
    });

    Style::default()
        .fg(tag_color.unwrap_or(highlight_color))
        .add_modifier(Modifier::BOLD)
}
