use crate::common::Item;
use eyre::{Result, eyre};
use time::macros::format_description;

use super::TagMetadataFormatter;

const IMAGE_TIMESTAMP_FORMAT: &[time::format_description::FormatItem<'static>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

/// Represents a clipboard entry from cclip with MIME type information
#[derive(Debug, Clone)]
pub struct CclipItem {
    pub rowid: String,
    pub mime_type: String,
    pub preview: String,
    data_size: Option<u64>,
    timestamp: Option<i64>,
    pub original_line: String,
    pub tags: Vec<String>,
}

impl CclipItem {
    /// Create a new CclipItem from a tab-separated line from cclip list
    /// Current format:
    /// rowid\tmime_type\tpreview\tdata_size\ttimestamp[\ttags]
    ///
    /// The legacy rowid\tmime_type\tpreview[\ttags] format remains supported.
    pub fn from_line(line: String) -> Result<Self> {
        let parts: Vec<&str> = line.split('\t').collect();

        if parts.len() < 3 {
            return Err(eyre!(
                "Invalid cclip list format: expected at least 3 tab-separated fields"
            ));
        }

        let metadata = parts
            .get(3)
            .zip(parts.get(4))
            .and_then(|(raw_data_size, raw_timestamp)| {
                Some((raw_data_size.parse().ok()?, raw_timestamp.parse().ok()?))
            });
        let (data_size, timestamp) = metadata
            .map(|(data_size, timestamp)| (Some(data_size), Some(timestamp)))
            .unwrap_or((None, None));
        let uses_metadata_format = metadata.is_some();
        let tags_index = if uses_metadata_format { 5 } else { 3 };
        let tags = if let Some(raw_tags) = parts.get(tags_index) {
            raw_tags
                .split(',')
                .filter_map(|tag| {
                    let trimmed = tag.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        Ok(CclipItem {
            rowid: parts[0].to_string(),
            mime_type: parts[1].to_string(),
            preview: parts[2].to_string(),
            data_size,
            timestamp,
            original_line: line,
            tags,
        })
    }

    /// Get a human-readable display name for this item using optional tag metadata formatting
    pub fn get_display_name_with_formatter(
        &self,
        formatter: Option<&TagMetadataFormatter>,
    ) -> String {
        self.get_display_name_with_formatter_options(formatter, true)
    }

    pub fn get_display_name_with_formatter_options(
        &self,
        formatter: Option<&TagMetadataFormatter>,
        include_color_names: bool,
    ) -> String {
        let base_name = match self.mime_type.as_str() {
            mime if mime.starts_with("image/") => self.image_display_name(mime),
            mime if mime.starts_with("text/") => self.preview.chars().take(80).collect::<String>(),
            _ => {
                format!(
                    "{} ({})",
                    self.preview.chars().take(50).collect::<String>(),
                    self.mime_type
                )
            }
        };

        format_tags_for_display(&self.tags, base_name, formatter, include_color_names)
    }

    fn image_display_name(&self, mime: &str) -> String {
        let Some(timestamp) = self.timestamp.and_then(format_timestamp) else {
            return format!(
                "{} ({})",
                self.preview.chars().take(50).collect::<String>(),
                mime
            );
        };
        let Some(data_size) = self.data_size else {
            return format!("{timestamp} ({mime})");
        };

        format!("{timestamp} ({}) ({mime})", format_data_size(data_size))
    }

    /// Get a human-readable display name without metadata formatting
    pub fn get_display_name(&self) -> String {
        self.get_display_name_with_formatter(None)
    }

    /// Get display name with rowid number prefix (for show_line_numbers)
    pub fn get_display_name_with_number(&self) -> String {
        self.get_display_name_with_number_formatter(None)
    }

    pub fn get_display_name_with_number_formatter(
        &self,
        formatter: Option<&TagMetadataFormatter>,
    ) -> String {
        self.get_display_name_with_number_formatter_options(formatter, true)
    }

    pub fn get_display_name_with_number_formatter_options(
        &self,
        formatter: Option<&TagMetadataFormatter>,
        include_color_names: bool,
    ) -> String {
        let base_name =
            self.get_display_name_with_formatter_options(formatter, include_color_names);
        let id_width = self.rowid.to_string().len().max(3);
        format!("{:<width$} {}", self.rowid, base_name, width = id_width)
    }
}

/// Convert CclipItem to Item for use with existing dmenu infrastructure
impl From<CclipItem> for Item {
    fn from(item: CclipItem) -> Self {
        let mut item_struct = Item::new_simple(
            item.original_line.clone(),
            item.get_display_name(),
            1, // line number, not really applicable for cclip
        );
        item_struct.tags = Some(item.tags);
        item_struct
    }
}

fn format_tags_for_display(
    tags: &[String],
    base: String,
    formatter: Option<&TagMetadataFormatter>,
    include_color_names: bool,
) -> String {
    if tags.is_empty() {
        return base;
    }

    let display_tags: Vec<String> = if let Some(formatter) = formatter {
        formatter.format_tags_with_options(tags, include_color_names)
    } else {
        tags.to_vec()
    };

    format!("[{}] {}", display_tags.join(", "), base)
}

fn format_timestamp(unix_seconds: i64) -> Option<String> {
    let timestamp = time::OffsetDateTime::from_unix_timestamp(unix_seconds).ok()?;
    let local_offset = time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC);
    timestamp
        .to_offset(local_offset)
        .format(IMAGE_TIMESTAMP_FORMAT)
        .ok()
}

fn format_data_size(data_size: u64) -> String {
    const UNITS: [&str; 3] = ["B", "KiB", "MiB"];

    let mut size = data_size as f64;
    let mut unit_index = 0;
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{data_size} B")
    } else {
        format!("{size:.2} {}", UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::{CclipItem, format_data_size};

    #[test]
    fn parses_current_cclip_metadata_format() {
        let item = CclipItem::from_line(
            "42\timage/png\timage/png | 44969 B (43.92 KiB)\t44969\t0\treference".into(),
        )
        .expect("current cclip line should parse");

        assert_eq!(item.rowid, "42");
        assert_eq!(item.data_size, Some(44_969));
        assert_eq!(item.timestamp, Some(0));
        assert_eq!(item.tags, vec!["reference"]);
    }

    #[test]
    fn legacy_cclip_format_remains_supported() {
        let item = CclipItem::from_line(
            "42\timage/png\timage/png | 44969 B (43.92 KiB)\treference".into(),
        )
        .expect("legacy cclip line should parse");

        assert_eq!(item.data_size, None);
        assert_eq!(item.timestamp, None);
        assert_eq!(item.tags, vec!["reference"]);
    }

    #[test]
    fn legacy_line_with_extra_tabs_is_not_rejected_as_metadata() {
        let item = CclipItem::from_line("42\ttext/plain\tpreview\twith\ttabs".into())
            .expect("legacy cclip line should not require numeric metadata");

        assert_eq!(item.data_size, None);
        assert_eq!(item.timestamp, None);
        assert_eq!(item.preview, "preview");
    }

    #[test]
    fn image_display_replaces_redundant_preview_with_timestamp() {
        let item = CclipItem::from_line(
            "42\timage/png\timage/png | 44969 B (43.92 KiB)\t44969\t0\t".into(),
        )
        .expect("current cclip line should parse");

        let display = item.get_display_name();
        assert!(!display.contains("image/png | 44969 B"));
        assert!(display.contains("(43.92 KiB) (image/png)"));
    }

    #[test]
    fn formats_binary_sizes_like_cclip() {
        assert_eq!(format_data_size(500), "500 B");
        assert_eq!(format_data_size(44_969), "43.92 KiB");
        assert_eq!(format_data_size(1_048_576), "1.00 MiB");
    }
}
