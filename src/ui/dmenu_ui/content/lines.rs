use ratatui::text::{Line, Span};

pub(super) fn normalize_display_content(
    content: String,
    show_line_numbers: bool,
    line_number: usize,
) -> String {
    let safe_content = sanitize_selected_content(content);
    let mut display_content = if show_line_numbers {
        format!("{}  {}", line_number, safe_content)
    } else {
        safe_content
    };

    display_content = display_content
        .replace('\n', " ")
        .replace('\t', "    ")
        .replace(['\r', '\0'], "");

    if display_content.contains('\x1b') {
        display_content = strip_ansi_escapes::strip_str(&display_content).to_string();
    }

    display_content
}

pub(super) fn build_content_lines<'a>(
    display_content: &str,
    wrap_long_lines: bool,
    panel_width: u16,
) -> Vec<Line<'a>> {
    if !wrap_long_lines {
        return vec![Line::from(Span::raw(display_content.to_string()))];
    }

    use unicode_width::UnicodeWidthStr;

    let max_width = panel_width.saturating_sub(2).max(20) as usize;
    let mut lines = Vec::new();
    let mut current_pos = 0;

    while current_pos < display_content.len() {
        let mut split_pos = current_pos;
        let remaining = &display_content[current_pos..];

        for (char_byte_pos, ch) in remaining.char_indices() {
            let candidate = &remaining[..char_byte_pos + ch.len_utf8()];
            if candidate.width() >= max_width {
                break;
            }
            split_pos = current_pos + char_byte_pos + ch.len_utf8();
        }

        if split_pos == current_pos {
            if let Some((next_byte_pos, ch)) = remaining.char_indices().next() {
                split_pos = current_pos + next_byte_pos + ch.len_utf8();
            } else {
                break;
            }
        }

        let chunk = &display_content[current_pos..split_pos];
        if chunk.width() >= max_width
            && let Some(safe_split) = find_safe_split(current_pos, remaining, max_width)
        {
            split_pos = safe_split;
        }

        lines.push(Line::from(Span::raw(
            display_content[current_pos..split_pos].to_string(),
        )));
        current_pos = split_pos;
    }

    lines
}

pub(super) fn pad_lines_to_height(
    lines: &mut Vec<Line<'static>>,
    panel_width: u16,
    panel_height: u16,
) {
    let blank_width = panel_width.saturating_sub(2) as usize;
    let blank_line = " ".repeat(blank_width);
    while lines.len() < panel_height as usize {
        lines.push(Line::from(Span::raw(blank_line.clone())));
    }
}

fn sanitize_selected_content(content: String) -> String {
    if content.is_empty() {
        "[Empty content]".to_string()
    } else if content.len() > 5000 {
        let mut truncate_at = 5000.min(content.len());
        while truncate_at > 0 && !content.is_char_boundary(truncate_at) {
            truncate_at -= 1;
        }
        format!("{}...", &content[..truncate_at])
    } else {
        content
    }
}

fn find_safe_split(current_pos: usize, remaining: &str, max_width: usize) -> Option<usize> {
    use unicode_width::UnicodeWidthStr;

    let mut safe_split = current_pos;
    for (char_byte_pos, ch) in remaining.char_indices() {
        let candidate = &remaining[..char_byte_pos + ch.len_utf8()];
        if candidate.width() < max_width {
            safe_split = current_pos + char_byte_pos + ch.len_utf8();
        } else {
            break;
        }
    }

    if safe_split > current_pos {
        Some(safe_split)
    } else {
        remaining
            .char_indices()
            .next()
            .map(|(next_byte_pos, ch)| current_pos + next_byte_pos + ch.len_utf8())
    }
}

#[cfg(test)]
mod tests {
    use super::{build_content_lines, normalize_display_content};

    #[test]
    fn normalize_display_content_strips_control_chars_and_ansi_sequences() {
        let normalized =
            normalize_display_content("hello\tworld\n\x1b[31mred\x1b[0m\r\0".to_string(), false, 1);

        assert_eq!(normalized, "hello    world red");
    }

    #[test]
    fn build_content_lines_wraps_long_text_without_returning_empty_lines() {
        let lines = build_content_lines("abcdefghijklmnopqrstuvwxyz", true, 22);

        assert!(lines.len() >= 2);
        assert!(lines.iter().all(|line| !line.spans.is_empty()));
    }
}
