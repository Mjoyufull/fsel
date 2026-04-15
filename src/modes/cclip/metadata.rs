use eyre::Result;
use ratatui::style::Color;
use std::collections::HashMap;

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
                    if include_color_names && let Some(color) = &meta.color {
                        display.push('(');
                        display.push_str(color);
                        display.push(')');
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

pub fn parse_color(input: &str) -> Result<Color> {
    crate::cli::string_to_color(input.to_string())
        .map_err(|e| eyre::eyre!("Invalid color '{}': {}", input, e))
}

/// Load tag metadata from fsel's database
pub(super) const TAG_METADATA_TABLE: redb::TableDefinition<&str, &[u8]> =
    redb::TableDefinition::new("tag_metadata");

pub fn load_tag_metadata(db: &std::sync::Arc<redb::Database>) -> HashMap<String, TagMetadata> {
    use redb::ReadableDatabase;

    let mut tags = HashMap::new();

    if let Ok(read_txn) = db.begin_read()
        && let Ok(table) = read_txn.open_table(TAG_METADATA_TABLE)
        && let Ok(Some(data)) = table.get("tag_metadata")
        && let Ok(metadata) = postcard::from_bytes::<Vec<TagMetadata>>(data.value())
    {
        for tag in metadata {
            tags.insert(tag.name.clone(), tag);
        }
    }

    tags
}

/// Save tag metadata to fsel's database
pub fn save_tag_metadata(
    db: &std::sync::Arc<redb::Database>,
    tags: &HashMap<String, TagMetadata>,
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
