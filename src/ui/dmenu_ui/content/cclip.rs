use super::super::DmenuUI;

impl<'a> DmenuUI<'a> {
    /// Check if an Item is a cclip item (has tab-separated format with rowid).
    pub(super) fn is_cclip_item(&self, item: &crate::common::Item) -> bool {
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
    pub(super) fn get_cclip_content_for_display(&mut self, item: &crate::common::Item) -> String {
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
                && !content.trim().is_empty()
            {
                self.content_cache
                    .insert(rowid.to_string(), content.clone());
                return content;
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
