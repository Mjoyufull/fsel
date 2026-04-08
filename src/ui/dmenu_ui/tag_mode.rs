use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::DmenuUI;

/// Tag mode state for cclip.
#[derive(Debug, Clone, PartialEq)]
pub enum TagMode {
    /// Normal mode (not tagging).
    Normal,
    /// Prompting for tag name.
    PromptingTagName {
        input: String,
        selected_item: Option<String>,
        available_tags: Vec<String>,
        selected_tag: Option<usize>,
    },
    /// Prompting for tag emoji.
    PromptingTagEmoji {
        tag_name: String,
        input: String,
        selected_item: Option<String>,
    },
    /// Prompting for tag color.
    PromptingTagColor {
        tag_name: String,
        emoji: Option<String>,
        input: String,
        selected_item: Option<String>,
    },
    /// Prompting for tag removal (blank removes all).
    RemovingTag {
        input: String,
        tags: Vec<String>,
        selected: Option<usize>,
        selected_item: Option<String>,
    },
}

impl<'a> DmenuUI<'a> {
    pub fn cycle_removal_selection(&mut self, direction: i32) {
        if let TagMode::RemovingTag {
            tags,
            selected,
            input,
            ..
        } = &mut self.tag_mode
        {
            if tags.is_empty() {
                *selected = None;
                return;
            }

            let len = tags.len() as i32;
            let current = selected.map(|idx| idx as i32).unwrap_or(0);
            let next = (current + direction).rem_euclid(len);
            *selected = Some(next as usize);

            if let Some(idx) = *selected
                && idx < tags.len()
            {
                *input = tags[idx].clone();
            }
        }
    }

    pub fn cycle_tag_creation_selection(&mut self, direction: i32) {
        if let TagMode::PromptingTagName {
            available_tags,
            selected_tag,
            input,
            ..
        } = &mut self.tag_mode
        {
            if available_tags.is_empty() {
                *selected_tag = None;
                return;
            }

            let len = available_tags.len() as i32;
            let current = selected_tag.map(|idx| idx as i32).unwrap_or(-1);
            let next = (current + direction).rem_euclid(len);
            *selected_tag = Some(next as usize);

            if let Some(idx) = *selected_tag
                && idx < available_tags.len()
            {
                let tag = &available_tags[idx];
                let clean_tag = tag.split('(').next().unwrap_or(tag).trim();
                let clean_tag = clean_tag
                    .trim_start_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '-');
                *input = clean_tag.to_string();
            }
        }
    }
}

pub(super) fn tag_mode_lines<'a>(
    tag_mode: &TagMode,
    temp_message: Option<&str>,
    highlight_color: Color,
) -> Option<Vec<Line<'a>>> {
    match tag_mode {
        TagMode::PromptingTagName {
            input,
            available_tags,
            selected_tag,
            ..
        } => {
            let mut text = vec![
                Line::from(vec![Span::styled(
                    "Tagging Mode",
                    Style::default().add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
                Line::from("Enter a tag name for this clipboard item."),
                Line::from("Use Up/Down to browse existing tags."),
                Line::from(""),
            ];

            if !available_tags.is_empty() {
                text.push(Line::from("Existing tags:"));
                for (idx, tag) in available_tags.iter().enumerate() {
                    let marker = if Some(idx) == *selected_tag {
                        "▶"
                    } else {
                        " "
                    };
                    text.push(Line::from(vec![
                        Span::styled(marker, Style::default().fg(highlight_color)),
                        Span::raw(" "),
                        Span::raw(tag.clone()),
                    ]));
                }
                text.push(Line::from(""));
            } else {
                text.push(Line::from("Examples: prompt, code, important, todo"));
                text.push(Line::from(""));
            }

            text.extend(prompt_input_lines(
                "Tag: ",
                input,
                highlight_color,
                temp_message,
                "Press Enter to continue, Esc to cancel.",
            ));
            Some(text)
        }
        TagMode::PromptingTagEmoji {
            tag_name, input, ..
        } => {
            let mut text = vec![
                Line::from(vec![Span::styled(
                    "Tag Emoji",
                    Style::default().add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
            ];

            extend_temp_message(&mut text, temp_message);
            text.extend_from_slice(&[
                Line::from(format!("Tag: {}", tag_name)),
                Line::from(""),
                Line::from("Enter an emoji to prefix the tag (optional):"),
                Line::from("  Examples: 📌 🔥 ⭐ 💡 📝"),
                Line::from("  Leave blank to keep existing emoji"),
            ]);
            text.extend(prompt_input_lines(
                "Emoji: ",
                input,
                highlight_color,
                None,
                "Press Enter to continue, Esc to cancel.",
            ));
            Some(text)
        }
        TagMode::PromptingTagColor {
            tag_name,
            emoji,
            input,
            ..
        } => {
            let emoji_display = emoji.as_deref().unwrap_or("(none)");
            let mut text = vec![
                Line::from(vec![Span::styled(
                    "Tag Color",
                    Style::default().add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
            ];

            extend_temp_message(&mut text, temp_message);
            text.extend_from_slice(&[
                Line::from(format!("Tag: {}", tag_name)),
                Line::from(format!("Emoji: {}", emoji_display)),
                Line::from(""),
                Line::from("Enter a color (optional):"),
                Line::from("  - Hex: #ff0000 or #f00"),
                Line::from("  - RGB: rgb(255,0,0)"),
                Line::from("  - Named: red, blue, green, etc."),
                Line::from("  - Leave blank to keep existing color"),
                Line::from(""),
            ]);
            text.extend(prompt_input_lines(
                "Color: ",
                input,
                highlight_color,
                None,
                "Press Enter to finish, Esc to cancel.",
            ));
            Some(text)
        }
        TagMode::RemovingTag {
            input,
            tags,
            selected,
            ..
        } => {
            let mut text = vec![
                Line::from(vec![Span::styled(
                    "Remove Tag",
                    Style::default().add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
            ];

            if tags.is_empty() {
                text.push(Line::from("No tags assigned to this entry."));
                text.push(Line::from(""));
            } else {
                text.push(Line::from("Use Up/Down to choose a tag, Enter to confirm."));
                text.push(Line::from(
                    "Leave blank and press Enter to remove all tags.",
                ));
                text.push(Line::from(""));

                for (idx, tag) in tags.iter().enumerate() {
                    let marker = if Some(idx) == *selected { "▶" } else { " " };
                    text.push(Line::from(vec![
                        Span::styled(marker, Style::default().fg(highlight_color)),
                        Span::raw(" "),
                        Span::raw(tag.clone()),
                    ]));
                }

                text.push(Line::from(""));
            }

            text.extend_from_slice(&[
                Line::from("Type to filter or add a tag name manually."),
                Line::from(""),
            ]);
            text.extend(prompt_input_lines(
                "Tag: ",
                input,
                highlight_color,
                None,
                "Press Enter to confirm, Esc to cancel.",
            ));
            Some(text)
        }
        TagMode::Normal => None,
    }
}

fn prompt_input_lines<'a>(
    label: &'a str,
    input: &str,
    highlight_color: Color,
    temp_message: Option<&str>,
    footer: &'a str,
) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    extend_temp_message(&mut lines, temp_message);
    lines.extend_from_slice(&[
        Line::from(vec![
            Span::styled(label, Style::default().fg(highlight_color)),
            Span::styled(
                input.to_string(),
                Style::default().fg(ratatui::style::Color::White),
            ),
            Span::styled("▌", Style::default().fg(highlight_color)),
        ]),
        Line::from(""),
        Line::from(footer),
    ]);
    lines
}

fn extend_temp_message<'a>(lines: &mut Vec<Line<'a>>, temp_message: Option<&str>) {
    if let Some(message) = temp_message {
        lines.push(Line::from(vec![Span::styled(
            message.to_string(),
            Style::default().fg(ratatui::style::Color::Yellow),
        )]));
        lines.push(Line::from(""));
    }
}
