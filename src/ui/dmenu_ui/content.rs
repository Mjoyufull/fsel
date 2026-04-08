use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::{DmenuUI, tag_mode::tag_mode_lines};

impl<'a> DmenuUI<'a> {
    /// Update `self.text` to show content for current selection.
    pub fn info(&mut self, color: Color) {
        self.info_with_image_support(color, false, false, 0, 0);
    }

    /// Update `self.text` to show content with optional image preview support.
    pub fn info_with_image_support(
        &mut self,
        highlight_color: Color,
        enable_images: bool,
        hide_image_message: bool,
        panel_width: u16,
        panel_height: u16,
    ) {
        if let Some(text) =
            tag_mode_lines(&self.tag_mode, self.temp_message_text(), highlight_color)
        {
            self.text = text;
            return;
        }

        let Some(selected) = self.selected else {
            self.text.clear();
            return;
        };

        if selected >= self.shown.len() {
            self.text.clear();
            return;
        }

        let item = self.shown[selected].clone();

        if enable_images && self.is_cclip_image_item(&item) {
            self.text = if hide_image_message {
                vec![Line::from(Span::raw(String::new()))]
            } else {
                vec![
                    Line::from(vec![
                        Span::styled(
                            "󰋩 IMAGE PREVIEW ",
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        image_status_span(),
                    ]),
                    Line::from(""),
                    Line::from("  󱇛 Press 'Alt+i' for Fullscreen View"),
                    Line::from("  󰆏 Press 'Enter' to Copy to Clipboard"),
                    Line::from(""),
                    Line::from(self.get_image_info(&item)),
                ]
            };
            return;
        }

        let content = if self.is_cclip_item(&item) {
            self.get_cclip_content_for_display(&item)
        } else {
            item.get_content_display()
        };

        let safe_content = sanitize_selected_content(content);
        let mut display_content = if self.show_line_numbers {
            format!("{}  {}", item.line_number, safe_content)
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

        let mut lines = build_content_lines(
            &display_content,
            self.wrap_long_lines,
            panel_width,
            panel_height,
        );
        if lines.is_empty() {
            lines.push(Line::from(Span::raw("[No content]")));
        }

        let blank_width = panel_width.saturating_sub(2) as usize;
        let blank_line = " ".repeat(blank_width);
        while lines.len() < panel_height as usize {
            lines.push(Line::from(Span::raw(blank_line.clone())));
        }

        self.text = lines;
    }

    /// Check if an Item is a cclip item (has tab-separated format with rowid).
    fn is_cclip_item(&self, item: &crate::common::Item) -> bool {
        if item.original_line.trim().is_empty() {
            return false;
        }

        let parts: Vec<&str> = item.original_line.splitn(3, '\t').collect();
        if parts.len() >= 2 {
            return parts[0].trim().parse::<u64>().is_ok();
        }

        false
    }

    /// Check if an Item is a cclip image item by parsing its original line.
    pub fn is_cclip_image_item(&self, item: &crate::common::Item) -> bool {
        if item.original_line.trim().is_empty() {
            return false;
        }

        let parts: Vec<&str> = item.original_line.splitn(4, '\t').collect();
        if parts.len() >= 2 {
            let mime_type = parts[1].trim();
            return !mime_type.is_empty() && mime_type.starts_with("image/");
        }

        false
    }

    /// Get actual clipboard content for display.
    fn get_cclip_content_for_display(&mut self, item: &crate::common::Item) -> String {
        let parts: Vec<&str> = item.original_line.splitn(4, '\t').collect();

        if parts.len() >= 3 {
            let rowid = parts[0].trim();
            let preview = parts[2].trim();

            if let Some(cached_content) = self.content_cache.get(rowid) {
                return cached_content.clone();
            }

            if let Ok(output) = std::process::Command::new("cclip")
                .args(["get", rowid])
                .output()
                && output.status.success()
                && let Ok(content) = String::from_utf8(output.stdout)
            {
                if !content.trim().is_empty() {
                    self.content_cache
                        .insert(rowid.to_string(), content.clone());
                    return content;
                }
            }

            if !preview.is_empty() {
                preview.to_string()
            } else {
                format!("[Failed to get content for rowid {}]", rowid)
            }
        } else if parts.len() >= 2 {
            format!("[{} content]", parts[1].trim())
        } else {
            item.original_line.clone()
        }
    }

    /// Get image info for display in the preview panel.
    pub fn get_image_info(&self, item: &crate::common::Item) -> String {
        if !self.is_cclip_image_item(item) {
            return String::new();
        }

        let parts: Vec<&str> = item.original_line.splitn(4, '\t').collect();
        if parts.len() >= 3 {
            let preview = parts[2].trim();
            if !preview.is_empty() {
                preview.to_string()
            } else {
                "Unknown Image".to_string()
            }
        } else {
            "Unknown Image".to_string()
        }
    }

    /// Get the rowid for any cclip item (not just images).
    pub fn get_cclip_rowid(&self, item: &crate::common::Item) -> Option<String> {
        let trimmed = item.original_line.trim();
        if trimmed.is_empty() {
            return None;
        }

        let parts: Vec<&str> = trimmed.splitn(2, '\t').collect();
        let rowid = parts[0].trim();
        if !rowid.is_empty() && rowid.chars().all(|c| c.is_ascii_digit()) {
            return Some(rowid.to_string());
        }

        None
    }
}

fn image_status_span() -> Span<'static> {
    let mut status_span = Span::styled("- Loading...", Style::default().fg(Color::Yellow));
    if let Ok(state) = crate::ui::DISPLAY_STATE.try_lock() {
        match &*state {
            crate::ui::DisplayState::Failed(message) => {
                status_span = Span::styled(
                    format!("- Failed: {}", message),
                    Style::default().fg(Color::Red),
                );
            }
            crate::ui::DisplayState::Image(_) => {
                status_span = Span::styled("- Ready", Style::default().fg(Color::Green));
            }
            _ => {}
        }
    }
    status_span
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

fn build_content_lines<'a>(
    display_content: &str,
    wrap_long_lines: bool,
    panel_width: u16,
    _panel_height: u16,
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
        if chunk.width() >= max_width {
            if let Some(safe_split) = find_safe_split(current_pos, remaining, max_width) {
                split_pos = safe_split;
            }
        }

        lines.push(Line::from(Span::raw(
            display_content[current_pos..split_pos].to_string(),
        )));
        current_pos = split_pos;
    }

    lines
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
