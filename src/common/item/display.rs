use super::Item;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::ListItem;

impl Item {
    /// Check if this item contains an image (basic heuristic).
    pub fn is_image(&self) -> bool {
        let text = self.original_line.to_lowercase();
        text.contains(".png")
            || text.contains(".jpg")
            || text.contains(".jpeg")
            || text.contains(".gif")
            || text.contains(".bmp")
            || text.contains(".webp")
            || text.contains(".svg")
            || text.contains("image/")
            || text.contains("img:")
    }

    /// Get content for display in the main panel (description area).
    pub fn get_content_display(&self) -> String {
        let content = if self.is_image() {
            format!("[IMAGE] {}", self.original_line)
        } else {
            self.original_line.clone()
        };

        if content.contains('\t') {
            let parts: Vec<&str> = content.split('\t').collect();
            if parts.len() >= 2 && parts[0].parse::<u64>().is_ok() {
                format!("{:<6} {}", parts[0], parts[1..].join("  "))
            } else {
                content.replace('\t', "  ")
            }
        } else {
            content
        }
    }

    /// Get the original line with any terminal escape sequences stripped.
    pub fn get_clean_original_line(&self) -> String {
        strip_ansi_escapes::strip_str(&self.original_line)
    }

    /// Create a `ListItem` with optional tag metadata formatting.
    pub fn to_list_item<'a>(
        &'a self,
        tag_metadata: Option<&'a crate::modes::cclip::TagMetadataFormatter>,
    ) -> ListItem<'a> {
        if let Some(actual_tags) = &self.tags
            && !actual_tags.is_empty()
            && let Some(formatter) = tag_metadata
            && let Some((tag_start, tag_end)) = tag_bounds(&self.display_text)
        {
            return build_tagged_list_item(self, actual_tags, formatter, tag_start, tag_end);
        }

        ListItem::new(self.display_text.clone())
    }
}

fn build_tagged_list_item<'a>(
    item: &'a Item,
    actual_tags: &'a [String],
    formatter: &'a crate::modes::cclip::TagMetadataFormatter,
    tag_start: usize,
    tag_end: usize,
) -> ListItem<'a> {
    let mut spans = Vec::new();

    if tag_start > 0 {
        spans.push(Span::raw(&item.display_text[..tag_start]));
    }

    let first_tag_color = actual_tags
        .first()
        .and_then(|tag| formatter.get_color(tag))
        .unwrap_or(Color::Green);
    spans.push(Span::styled("[", Style::default().fg(first_tag_color)));

    let formatted_tags = item.display_text[tag_start + 1..tag_end]
        .split(", ")
        .collect::<Vec<_>>();

    for (index, tag_name) in actual_tags.iter().enumerate() {
        let tag_color = formatter.get_color(tag_name).unwrap_or(Color::Green);
        let display = formatted_tags
            .get(index)
            .map(|tag| (*tag).to_string())
            .unwrap_or_else(|| fallback_tag_display(tag_name, formatter));
        spans.push(Span::styled(display, Style::default().fg(tag_color)));

        if index < actual_tags.len() - 1 {
            spans.push(Span::styled(", ", Style::default().fg(first_tag_color)));
        }
    }

    spans.push(Span::styled("]", Style::default().fg(first_tag_color)));
    if tag_end + 2 < item.display_text.len() {
        spans.push(Span::raw(&item.display_text[tag_end + 2..]));
    }

    ListItem::new(Line::from(spans))
}

fn tag_bounds(display_text: &str) -> Option<(usize, usize)> {
    let tag_start = display_text.find('[')?;
    let tag_end = display_text.find(']')?;
    Some((tag_start, tag_end))
}

fn fallback_tag_display(
    tag_name: &str,
    formatter: &crate::modes::cclip::TagMetadataFormatter,
) -> String {
    let mut display = String::new();
    if let Some(meta) = formatter.metadata.get(tag_name)
        && let Some(emoji) = &meta.emoji
    {
        display.push_str(emoji);
        display.push(' ');
    }
    display.push_str(tag_name);
    display
}
