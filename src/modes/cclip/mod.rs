// Cclip mode - clipboard history browser

pub mod preview;
pub mod run;
pub mod scan;
pub mod select;

use crate::common::Item;
use eyre::{eyre, Result};
use ratatui::style::Color;
use std::collections::HashMap;

// Re-export main entry point
pub use run::run;

// Re-export commonly used scan functions
pub use scan::{check_cclip_available, check_cclip_database};

/// Represents a clipboard entry from cclip with MIME type information
#[derive(Debug, Clone)]
pub struct CclipItem {
    pub rowid: String,
    pub mime_type: String,
    pub preview: String,
    pub original_line: String,
    pub tags: Vec<String>,
}

impl CclipItem {
    /// Create a new CclipItem from a tab-separated line from cclip list
    /// Format: rowid\tmime_type\tpreview[\ttags]
    /// The optional `tags` field is a comma-separated list of tag names.
    pub fn from_line(line: String) -> Result<Self> {
        let parts: Vec<&str> = line.splitn(4, '\t').collect();

        if parts.len() < 3 {
            return Err(eyre!(
                "Invalid cclip list format: expected at least 3 tab-separated fields"
            ));
        }

        let tags = if parts.len() >= 4 {
            parts[3]
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
            mime if mime.starts_with("image/") => {
                format!(
                    "{} ({})",
                    self.preview.chars().take(50).collect::<String>(),
                    mime
                )
            }
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
        format!("{:<3} {}", self.rowid, base_name)
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

pub fn parse_color(input: &str) -> Result<Color> {
    // Use the existing comprehensive color parser from cli.rs
    crate::cli::string_to_color(input.to_string())
        .map_err(|e| eyre!("Invalid color '{}': {}", input, e))
}

/// Display metadata for tags (colors, emojis).
pub struct TagMetadataFormatter {
    pub metadata: HashMap<String, TagMetadata>,
}

impl TagMetadataFormatter {
    pub fn new(metadata: HashMap<String, TagMetadata>) -> Self {
        Self { metadata }
    }

    pub fn get_color_string(&self, tag: &str) -> Option<&str> {
        self.metadata
            .get(tag)
            .and_then(|meta| meta.color.as_deref())
    }

    pub fn get_color(&self, tag: &str) -> Option<Color> {
        self.get_color_string(tag)
            .and_then(|value| parse_color(value).ok())
    }

    pub fn get_emoji(&self, tag: &str) -> Option<&str> {
        self.metadata
            .get(tag)
            .and_then(|meta| meta.emoji.as_deref())
    }

    pub fn format_tags_with_options(
        &self,
        tags: &[String],
        include_color_names: bool,
    ) -> Vec<String> {
        tags.iter()
            .map(|tag| {
                if let Some(meta) = self.metadata.get(tag) {
                    let mut display = String::new();
                    if let Some(emoji) = &meta.emoji {
                        display.push_str(emoji);
                        display.push(' ');
                    }
                    display.push_str(tag);
                    if include_color_names {
                        if let Some(color) = &meta.color {
                            display.push('(');
                            display.push_str(color);
                            display.push(')');
                        }
                    }
                    display
                } else {
                    tag.clone()
                }
            })
            .collect()
    }

    pub fn format_tags(&self, tags: &[String]) -> Vec<String> {
        self.format_tags_with_options(tags, true)
    }
}

/// Tag metadata stored in fsel's database
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TagMetadata {
    pub name: String,
    pub color: Option<String>, // Hex color or named color
    pub emoji: Option<String>, // Optional emoji prefix
}

impl TagMetadata {
    pub fn new(name: String) -> Self {
        Self {
            name,
            color: None,
            emoji: None,
        }
    }

    pub fn with_color(mut self, color: String) -> Self {
        self.color = Some(color);
        self
    }

    pub fn with_emoji(mut self, emoji: String) -> Self {
        self.emoji = Some(emoji);
        self
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

/// Load tag metadata from fsel's database
const TAG_METADATA_TABLE: redb::TableDefinition<&str, &[u8]> =
    redb::TableDefinition::new("tag_metadata");

pub fn load_tag_metadata(
    db: &std::sync::Arc<redb::Database>,
) -> std::collections::HashMap<String, TagMetadata> {
    use redb::ReadableDatabase;
    let mut tags = std::collections::HashMap::new();

    if let Ok(read_txn) = db.begin_read() {
        if let Ok(table) = read_txn.open_table(TAG_METADATA_TABLE) {
            if let Ok(Some(data)) = table.get("tag_metadata") {
                if let Ok(metadata) = postcard::from_bytes::<Vec<TagMetadata>>(data.value()) {
                    for tag in metadata {
                        tags.insert(tag.name.clone(), tag);
                    }
                }
            }
        }
    }

    tags
}

/// Save tag metadata to fsel's database
pub fn save_tag_metadata(
    db: &std::sync::Arc<redb::Database>,
    tags: &std::collections::HashMap<String, TagMetadata>,
) -> Result<()> {
    let metadata: Vec<TagMetadata> = tags.values().cloned().collect();
    let data = postcard::to_allocvec(&metadata)?;

    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(TAG_METADATA_TABLE)?;
        table.insert("tag_metadata", data.as_slice())?;
    }
    write_txn.commit()?;
    Ok(())
}
