// Preview functionality for clipboard items

use super::CclipItem;
use eyre::{eyre, Result};
use std::process::{Command, Stdio};

impl CclipItem {
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
            .args(["get", &self.rowid])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?
            .wait_with_output()?;

        if !output.status.success() {
            return Err(eyre!(
                "Failed to get clipboard content for rowid {}",
                self.rowid
            ));
        }

        Ok(output.stdout)
    }
}
