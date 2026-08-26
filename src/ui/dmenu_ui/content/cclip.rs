use super::super::DmenuUI;
use std::process::Command;
use std::sync::mpsc::{self, TryRecvError};

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
            let mime_type = parts[1].trim();
            let preview = parts[2];

            let content = if let Some(cached_content) = self.content_cache.get(rowid) {
                cached_content.clone()
            } else if let Some(fetched_content) = self.poll_cclip_content_request(rowid) {
                fetched_content
            } else if !preview.is_empty() {
                let raw_content = self.cclip_verbosity > 0;
                self.start_cclip_content_request(rowid, mime_type, raw_content);
                let display_preview = content_for_view(mime_type, preview, raw_content);
                if display_preview.is_empty() {
                    "[Loading HTML content...]".to_string()
                } else {
                    display_preview
                }
            } else {
                format!("[Failed to get content for rowid {}]", rowid)
            };

            self.add_diagnostics(rowid, mime_type, content)
        } else if parts.len() >= 2 {
            format!("[{} content]", parts[1].trim())
        } else {
            item.original_line.clone()
        }
    }

    fn start_cclip_content_request(&mut self, rowid: &str, mime_type: &str, raw_content: bool) {
        if self.content_requests.contains_key(rowid) {
            return;
        }

        let rowid_owned = rowid.to_string();
        let mime_type_owned = mime_type.to_string();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let content = Command::new("cclip")
                .args(["get", &rowid_owned])
                .output()
                .ok()
                .filter(|output| output.status.success())
                .and_then(|output| {
                    decode_content_for_view(&mime_type_owned, output.stdout, raw_content)
                });
            let _ = tx.send(content);
        });
        self.content_requests.insert(rowid.to_string(), rx);
    }

    fn poll_cclip_content_request(&mut self, rowid: &str) -> Option<String> {
        let receiver = self.content_requests.get(rowid)?;
        match receiver.try_recv() {
            Ok(Some(content)) => {
                self.content_requests.remove(rowid);
                self.content_cache
                    .insert(rowid.to_string(), content.clone());
                Some(content)
            }
            Ok(None) | Err(TryRecvError::Disconnected) => {
                self.content_requests.remove(rowid);
                None
            }
            Err(TryRecvError::Empty) => None,
        }
    }

    fn add_diagnostics(&self, rowid: &str, mime_type: &str, content: String) -> String {
        if self.cclip_verbosity < 3 {
            return content;
        }

        format!("[cclip rowid={rowid} mime={mime_type} view=raw] {content}")
    }

    pub(super) fn get_cclip_diagnostics(&self, item: &crate::common::Item) -> Option<String> {
        if self.cclip_verbosity < 3 {
            return None;
        }

        let mut parts = item.original_line.splitn(3, '\t');
        let rowid = parts.next()?.trim();
        let mime_type = parts.next()?.trim();
        Some(format!("[cclip rowid={rowid} mime={mime_type} view=image]"))
    }

    /// Get image info for display in the preview panel.
    pub fn get_image_info(&self, item: &crate::common::Item) -> String {
        if !self.is_cclip_image_item(item) {
            return String::new();
        }

        let parts: Vec<&str> = item.original_line.splitn(4, '\t').collect();
        if parts.len() >= 3 {
            let preview = parts[2];
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

fn content_for_view(mime_type: &str, content: &str, raw_content: bool) -> String {
    if raw_content {
        content.to_string()
    } else {
        crate::modes::cclip::html::text_for_display(mime_type, content)
    }
}

fn decode_content_for_view(mime_type: &str, bytes: Vec<u8>, raw_content: bool) -> Option<String> {
    if !is_textual_mime(mime_type) {
        return None;
    }

    let content = String::from_utf8(bytes).ok()?;
    Some(content_for_view(mime_type, &content, raw_content))
}

fn is_textual_mime(mime_type: &str) -> bool {
    let essence = mime_type.split(';').next().unwrap_or(mime_type).trim();
    let normalized = essence.to_ascii_lowercase();
    normalized.starts_with("text/")
        || normalized == "application/json"
        || normalized == "application/xml"
        || normalized.ends_with("+json")
        || normalized.ends_with("+xml")
}

#[cfg(test)]
mod tests {
    use super::decode_content_for_view;

    #[test]
    fn fetched_html_is_rendered_by_default() {
        let content =
            decode_content_for_view("text/html", b"<p>Hello &amp; goodbye</p>".to_vec(), false);

        assert_eq!(content.as_deref(), Some("Hello & goodbye"));
    }

    #[test]
    fn verbose_fetched_html_stays_raw() {
        let html = "<p>Hello &amp; goodbye</p>";
        let content = decode_content_for_view("text/html", html.as_bytes().to_vec(), true);

        assert_eq!(content.as_deref(), Some(html));
    }

    #[test]
    fn png_bytes_are_never_lossily_rendered_as_text() {
        let png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();

        assert_eq!(decode_content_for_view("text/html", png, false), None);
    }

    #[test]
    fn binary_mime_is_rejected_even_when_its_bytes_are_valid_utf8() {
        let bytes = b"apparently readable binary".to_vec();

        assert_eq!(decode_content_for_view("image/svg", bytes, true), None);
    }
}
