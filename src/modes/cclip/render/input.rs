use super::super::state::CclipOptions;
use crate::ui::{DmenuUI, TagMode};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

pub(super) fn input_line_and_title(
    ui: &DmenuUI<'_>,
    options: &CclipOptions,
) -> (Line<'static>, &'static str) {
    match &ui.tag_mode {
        TagMode::PromptingTagName { input, .. } => {
            (prompt_line("Tag: ", input, options, None), " Tag Name ")
        }
        TagMode::PromptingTagEmoji { input, .. } => (
            prompt_line("Emoji: ", input, options, Some(" (or blank)")),
            " Tag Emoji ",
        ),
        TagMode::PromptingTagColor { input, .. } => (
            prompt_line("Color: ", input, options, Some(" (hex/name or blank)")),
            " Tag Color ",
        ),
        TagMode::RemovingTag { input, .. } => (
            prompt_line("Remove: ", input, options, Some(" (blank = all)")),
            " Remove Tag ",
        ),
        TagMode::Normal => (filter_line(ui, options), " Filter "),
    }
}

fn prompt_line(
    label: &'static str,
    input: &str,
    options: &CclipOptions,
    hint: Option<&'static str>,
) -> Line<'static> {
    let mut spans = vec![
        Span::styled(label, Style::default().fg(options.highlight_color)),
        Span::styled(
            input.to_string(),
            Style::default().fg(options.input_text_color),
        ),
        Span::styled(
            options.cursor.clone(),
            Style::default().fg(options.highlight_color),
        ),
    ];

    if let Some(hint) = hint {
        spans.push(Span::styled(
            hint,
            Style::default()
                .fg(options.input_text_color)
                .add_modifier(Modifier::DIM),
        ));
    }

    Line::from(spans)
}

fn filter_line(ui: &DmenuUI<'_>, options: &CclipOptions) -> Line<'static> {
    Line::from(vec![
        Span::styled("(", Style::default().fg(options.input_text_color)),
        Span::styled(
            ui.selected.map_or(0, |selected| selected + 1).to_string(),
            Style::default().fg(options.highlight_color),
        ),
        Span::styled("/", Style::default().fg(options.input_text_color)),
        Span::styled(
            ui.shown.len().to_string(),
            Style::default().fg(options.input_text_color),
        ),
        Span::styled(") ", Style::default().fg(options.input_text_color)),
        Span::styled(">", Style::default().fg(options.highlight_color)),
        Span::styled("> ", Style::default().fg(options.input_text_color)),
        Span::styled(
            ui.query.clone(),
            Style::default().fg(options.input_text_color),
        ),
        Span::styled(
            options.cursor.clone(),
            Style::default().fg(options.highlight_color),
        ),
    ])
}
