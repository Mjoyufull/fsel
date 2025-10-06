use std::process::{Command, Stdio};
use eyre::{Result, eyre};

use crate::dmenu::DmenuItem;

/// Represents a clipboard entry from cclip with MIME type information
#[derive(Debug, Clone)]
pub struct CclipItem {
    pub rowid: String,
    pub mime_type: String,
    pub preview: String,
    pub original_line: String,
}

impl CclipItem {
    /// Create a new CclipItem from a tab-separated line from cclip list
    pub fn from_line(line: String) -> Result<Self> {
        let parts: Vec<&str> = line.splitn(3, '\t').collect();
        
        if parts.len() != 3 {
            return Err(eyre!("Invalid cclip list format: expected 3 tab-separated fields"));
        }
        
        Ok(CclipItem {
            rowid: parts[0].to_string(),
            mime_type: parts[1].to_string(),
            preview: parts[2].to_string(),
            original_line: line,
        })
    }
    
    /// Get a human-readable display name for this item
    pub fn get_display_name(&self) -> String {
        match self.mime_type.as_str() {
            mime if mime.starts_with("image/") => {
                format!("  {} ({})", 
                    self.preview.chars().take(50).collect::<String>(),
                    mime)
            },
            mime if mime.starts_with("text/") => {
                format!("  {}", 
                    self.preview.chars().take(80).collect::<String>())
            },
            _ => {
                format!("  {} ({})", 
                    self.preview.chars().take(50).collect::<String>(),
                    self.mime_type)
            }
        }
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
    
    /// Copy this item back to the clipboard
    pub fn copy_to_clipboard(&self) -> Result<()> {
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
        
        // Pipe cclip output to wl-copy
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
    let output = Command::new("cclip")
        .args(&["list", "rowid,mime_type,preview"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .wait_with_output()?;
    
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

/// Generate image preview using chafa for the content panel
pub fn generate_image_preview(content: &[u8], width: u16, height: u16) -> Result<String> {
    let mut child = Command::new("chafa")
        .args(&[
            "-f", "sixels",
            "--align", "center", 
            "--scale", "max",
            "--passthrough", "none",
            "--view-size", &format!("{}x{}", width, height),
            "-"
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let content_copy = content.to_vec(); // Copy the content to avoid lifetime issues
        std::thread::spawn(move || {
            stdin.write_all(&content_copy).ok();
        });
    }
    
    let output = child.wait_with_output()?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(eyre!("chafa failed to generate image preview"))
    }
}
