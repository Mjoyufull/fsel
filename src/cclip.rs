use std::process::{Command, Stdio};
use eyre::{Result, eyre};
use redb::ReadableDatabase;

use crate::dmenu::DmenuItem;

/// Represents a clipboard entry from cclip with MIME type information
#[derive(Debug, Clone)]
pub struct CclipItem {
    pub rowid: String,
    pub mime_type: String,
    pub preview: String,
    pub original_line: String,
    pub tag: Option<String>,
}

impl CclipItem {
    /// Create a new CclipItem from a tab-separated line from cclip list
    /// Format: rowid\tmime_type\tpreview[\ttag]
    pub fn from_line(line: String) -> Result<Self> {
        let parts: Vec<&str> = line.splitn(4, '\t').collect();
        
        if parts.len() < 3 {
            return Err(eyre!("Invalid cclip list format: expected at least 3 tab-separated fields"));
        }
        
        let tag = if parts.len() >= 4 && !parts[3].is_empty() {
            Some(parts[3].to_string())
        } else {
            None
        };
        
        Ok(CclipItem {
            rowid: parts[0].to_string(),
            mime_type: parts[1].to_string(),
            preview: parts[2].to_string(),
            original_line: line,
            tag,
        })
    }
    
    /// Get a human-readable display name for this item
    pub fn get_display_name(&self) -> String {
        let base_name = match self.mime_type.as_str() {
            mime if mime.starts_with("image/") => {
                format!("{} ({})", 
                    self.preview.chars().take(50).collect::<String>(),
                    mime)
            },
            mime if mime.starts_with("text/") => {
                self.preview.chars().take(80).collect::<String>()
            },
            _ => {
                format!("{} ({})", 
                    self.preview.chars().take(50).collect::<String>(),
                    self.mime_type)
            }
        };
        
        // Add tag prefix if present
        if let Some(ref tag) = self.tag {
            format!("[{}] {}", tag, base_name)
        } else {
            base_name
        }
    }
    
    /// Get display name with rowid number prefix (for show_line_numbers)
    pub fn get_display_name_with_number(&self) -> String {
        let base_name = self.get_display_name();
        format!("{:<3} {}", self.rowid, base_name)
    }
    
    /// Check if this item is an image
    pub fn is_image(&self) -> bool {
        self.mime_type.starts_with("image/")
    }
    
    /// Check if this item is text
    pub fn is_text(&self) -> bool {
        self.mime_type.starts_with("text/")
    }
    
    /// Get the content for preview in the content panel
    pub fn get_content_for_preview(&self) -> Result<Vec<u8>> {
        let output = Command::new("cclip")
            .args(&["get", &self.rowid])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?
            .wait_with_output()?;
        
        if !output.status.success() {
            return Err(eyre!("Failed to get clipboard content for rowid {}", self.rowid));
        }
        
        Ok(output.stdout)
    }
    
    /// Copy this item back to the clipboard (Wayland)
    fn copy_to_clipboard_wayland(&self) -> Result<()> {
        let mut cclip_child = Command::new("cclip")
            .args(&["get", &self.rowid])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        
        let mut wl_copy_child = Command::new("wl-copy")
            .args(&["-t", &self.mime_type])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        
        // pipe cclip output to wl-copy
        if let (Some(cclip_stdout), Some(wl_copy_stdin)) = 
            (cclip_child.stdout.take(), wl_copy_child.stdin.take()) 
        {
            std::thread::spawn(move || {
                let mut cclip_stdout = cclip_stdout;
                let mut wl_copy_stdin = wl_copy_stdin;
                std::io::copy(&mut cclip_stdout, &mut wl_copy_stdin).ok();
            });
        }
        
        let cclip_status = cclip_child.wait()?;
        let wl_copy_status = wl_copy_child.wait()?;
        
        if !cclip_status.success() {
            return Err(eyre!("cclip get failed"));
        }
        
        if !wl_copy_status.success() {
            return Err(eyre!("wl-copy failed"));
        }
        
        Ok(())
    }
    
    /// Copy this item back to the clipboard (X11)
    fn copy_to_clipboard_x11(&self) -> Result<()> {
        // try xclip first, then xsel as fallback
        let x11_tools = ["xclip", "xsel"];
        
        for tool in &x11_tools {
            if !Command::new(tool)
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            {
                continue;
            }
            
            let mut cclip_child = Command::new("cclip")
                .args(&["get", &self.rowid])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()?;
            
            let args = match *tool {
                "xclip" => vec!["-selection", "clipboard", "-t", &self.mime_type],
                "xsel" => vec!["--clipboard", "--input"],
                _ => unreachable!(),
            };
            
            let mut x11_child = Command::new(tool)
                .args(&args)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            
            // pipe cclip output to x11 tool
            if let (Some(cclip_stdout), Some(x11_stdin)) = 
                (cclip_child.stdout.take(), x11_child.stdin.take()) 
            {
                std::thread::spawn(move || {
                    let mut cclip_stdout = cclip_stdout;
                    let mut x11_stdin = x11_stdin;
                    std::io::copy(&mut cclip_stdout, &mut x11_stdin).ok();
                });
            }
            
            let cclip_status = cclip_child.wait()?;
            let x11_status = x11_child.wait()?;
            
            if !cclip_status.success() {
                return Err(eyre!("cclip get failed"));
            }
            
            if !x11_status.success() {
                continue; // try next tool
            }
            
            return Ok(());
        }
        
        Err(eyre!("no X11 clipboard tool found (tried xclip, xsel)"))
    }
    
    /// Copy this item back to the clipboard (auto-detect Wayland/X11)
    pub fn copy_to_clipboard(&self) -> Result<()> {
        // check if we're on wayland
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            self.copy_to_clipboard_wayland()
        } else {
            self.copy_to_clipboard_x11()
        }
    }
}

/// Convert CclipItem to DmenuItem for use with existing dmenu infrastructure
impl From<CclipItem> for DmenuItem {
    fn from(item: CclipItem) -> Self {
        DmenuItem::new_simple(
            item.original_line.clone(),
            item.get_display_name(),
            1 // line number, not really applicable for cclip
        )
    }
}

/// Get clipboard history from cclip
pub fn get_clipboard_history() -> Result<Vec<CclipItem>> {
    // Try with tag field first (newer cclip), fall back to without tag (older cclip)
    let output = Command::new("cclip")
        .args(&["list", "rowid,mime_type,preview,tag"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .wait_with_output()?;
    
    // If tag field not supported, try without it
    let output = if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("invalid field: tag") {
            // Older cclip version without tag support
            Command::new("cclip")
                .args(&["list", "rowid,mime_type,preview"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
                .wait_with_output()?
        } else {
            output
        }
    } else {
        output
    };
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(eyre!("cclip list failed: {}", stderr));
    }
    
    let stdout = String::from_utf8(output.stdout)?;
    let mut items = Vec::new();
    
    for line in stdout.lines() {
        if !line.trim().is_empty() {
            match CclipItem::from_line(line.to_string()) {
                Ok(item) => items.push(item),
                Err(e) => eprintln!("Warning: Failed to parse cclip line: {}", e),
            }
        }
    }
    
    Ok(items)
}

/// Check if cclip is available on the system
pub fn check_cclip_available() -> bool {
    Command::new("cclip")
        .arg("-h")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Check if cclip database exists and has entries
pub fn check_cclip_database() -> Result<()> {
    let output = Command::new("cclip")
        .args(&["list", "rowid"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .wait_with_output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("unable to open database file") {
            return Err(eyre!("cclip database not found. Make sure cclipd is running and has stored some clipboard history."));
        } else {
            return Err(eyre!("cclip error: {}", stderr));
        }
    }
    
    Ok(())
}

/// Check if chafa is available for image previews
pub fn check_chafa_available() -> bool {
    Command::new("chafa")
        .arg("--help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Tag metadata stored in fsel's database
/// DISABLED: Waiting for cclip maintainer to add tag support
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TagMetadata {
    pub name: String,
    pub color: Option<String>,  // Hex color or named color
    pub emoji: Option<String>,  // Optional emoji prefix
}

#[allow(dead_code)]
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

/// Load tag metadata from fsel's database
const TAG_METADATA_TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("tag_metadata");

/// DISABLED: Waiting for cclip maintainer to add tag support
#[allow(dead_code)]
pub fn load_tag_metadata(db: &std::sync::Arc<redb::Database>) -> std::collections::HashMap<String, TagMetadata> {
    let mut tags = std::collections::HashMap::new();
    
    if let Ok(read_txn) = db.begin_read() {
        if let Ok(table) = read_txn.open_table(TAG_METADATA_TABLE) {
            if let Ok(Some(data)) = table.get("tag_metadata") {
                if let Ok(metadata) = bincode::deserialize::<Vec<TagMetadata>>(data.value()) {
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
/// DISABLED: Waiting for cclip maintainer to add tag support
#[allow(dead_code)]
pub fn save_tag_metadata(db: &std::sync::Arc<redb::Database>, tags: &std::collections::HashMap<String, TagMetadata>) -> Result<()> {
    let metadata: Vec<TagMetadata> = tags.values().cloned().collect();
    let data = bincode::serialize(&metadata)?;
    
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(TAG_METADATA_TABLE)?;
        table.insert("tag_metadata", data.as_slice())?;
    }
    write_txn.commit()?;
    Ok(())
}

/// Tag a cclip item using cclip's tag command
/// DISABLED: Waiting for cclip maintainer to add tag support
#[allow(dead_code)]
pub fn tag_item(rowid: &str, tag: &str) -> Result<()> {
    let output = Command::new("cclip")
        .args(&["tag", rowid, tag])
        .output()?;
    
    if !output.status.success() {
        return Err(eyre!("Failed to tag item: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    Ok(())
}

/// Remove tag from a cclip item
#[allow(dead_code)]
pub fn untag_item(rowid: &str) -> Result<()> {
    let output = Command::new("cclip")
        .args(&["tag", "-d", rowid])
        .output()?;
    
    if !output.status.success() {
        return Err(eyre!("Failed to remove tag: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    Ok(())
}

/// Get all unique tags from cclip database
pub fn get_all_tags() -> Result<Vec<String>> {
    let output = Command::new("cclip")
        .args(&["list", "-t", "tag"])
        .output()?;
    
    if !output.status.success() {
        return Err(eyre!("Failed to list tags: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    let tags: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
        .collect();
    
    Ok(tags)
}

/// Get clipboard items filtered by tag
pub fn get_clipboard_history_by_tag(tag: &str) -> Result<Vec<CclipItem>> {
    // Query cclip for items with specific tag
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("cclip list -t rowid,mime_type,preview,tag | grep -F $'\\t{}$' || true", tag))
        .output()?;
    
    if !output.status.success() {
        return Err(eyre!("Failed to get clipboard history"));
    }
    
    let items: Result<Vec<CclipItem>> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| CclipItem::from_line(line.to_string()))
        .collect();
    
    items
}
