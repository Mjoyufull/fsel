use eyre::{eyre, Result};
use ratatui::layout::Rect;
use ratatui::Frame;
use ratatui_image::picker::Picker;
use ratatui_image::{Resize, StatefulImage};
use std::process::Stdio;
use std::sync::Mutex;

use ratatui_image::picker::ProtocolType;
use ratatui_image::protocol::StatefulProtocol;

/// Combined display state to track what's currently on screen
#[derive(Debug, Clone, PartialEq)]
pub enum DisplayState {
    /// No content displayed
    Empty,
    /// Image content is displayed with rowid
    Image(String),
    /// Image is currently loading in background
    Loading(String),
    /// Failed to load with error message
    Failed(String),
}

/// Single atomic state tracker to eliminate lock contention
pub static DISPLAY_STATE: Mutex<DisplayState> = Mutex::new(DisplayState::Empty);

/// Manages image loading and rendering using ratatui-image
pub struct ImageManager {
    picker: Picker,
    protocol: Option<StatefulProtocol>,
}

impl ImageManager {
    /// Initialize the image manager by detecting terminal capabilities
    /// This should be called after entering alternate screen
    pub fn new(picker: Picker) -> Self {
        Self {
            picker,
            protocol: None,
        }
    }

    /// Check if the terminal supports any high-resolution graphics protocol
    /// Note: Halfblocks is considered a lower-fidelity but valid fallback.
    pub fn supports_graphics(&self) -> bool {
        !matches!(self.picker.protocol_type(), ProtocolType::Halfblocks)
    }

    /// Is the current protocol Sixel?
    pub fn is_sixel(&self) -> bool {
        matches!(self.picker.protocol_type(), ProtocolType::Sixel)
    }

    /// Load image data from cclip and prepare it for rendering
    pub async fn load_cclip_image(&mut self, rowid: &str) -> Result<()> {
        // Run cclip get to fetch image bytes using tokio
        let mut child = tokio::process::Command::new("cclip")
            .args(["get", rowid])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let mut child_stdout = child
            .stdout
            .take()
            .ok_or_else(|| eyre!("Failed to capture stdout"))?;

        let read_future = async move {
            let mut bytes = Vec::new();
            tokio::io::AsyncReadExt::read_to_end(&mut child_stdout, &mut bytes).await?;
            Ok::<Vec<u8>, std::io::Error>(bytes)
        };

        // Wrap the execution + stdout read in a timeout
        let bytes =
            match tokio::time::timeout(std::time::Duration::from_millis(1500), read_future).await {
                Ok(res) => {
                    // Also wrap child.wait() in a timeout to avoid blocking the executor
                    match tokio::time::timeout(std::time::Duration::from_millis(500), child.wait())
                        .await
                    {
                        Ok(wait_res) => {
                            let _ = wait_res;
                        }
                        Err(_) => {
                            let _ = child.kill().await;
                        }
                    }
                    res?
                }
                Err(_) => {
                    let _ = child.kill().await;
                    return Err(eyre!("Timeout reading image data from cclip get {}", rowid));
                }
            };

        if bytes.is_empty() {
            return Err(eyre!("No data received from cclip get {}", rowid));
        }

        let picker = self.picker.clone();

        let protocol = tokio::task::spawn_blocking(move || {
            let dyn_img = image::load_from_memory(&bytes)?;
            Ok::<_, eyre::Report>(picker.new_resize_protocol(dyn_img))
        })
        .await??;

        // Create new protocol state
        self.protocol = Some(protocol);

        Ok(())
    }

    /// Render the current image into the given area
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        if let Some(ref mut protocol) = self.protocol {
            f.render_stateful_widget(
                StatefulImage::default().resize(Resize::Fit(None)),
                area,
                protocol,
            );
        }
    }

    /// Clear the current image protocol
    pub fn clear(&mut self) {
        self.protocol = None;
    }
}

/// Legacy GraphicsAdapter enum to minimize breakage in matches
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GraphicsAdapter {
    Kitty,
    Sixel,
    None,
}

impl GraphicsAdapter {
    /// Detect the best graphics adapter (legacy)
    pub fn detect(picker: Option<&Picker>) -> Self {
        if let Some(picker) = picker {
            use ratatui_image::picker::ProtocolType;
            match picker.protocol_type() {
                ProtocolType::Kitty => return Self::Kitty,
                ProtocolType::Sixel => return Self::Sixel,
                _ => {}
            }
        }

        let term = std::env::var("TERM").unwrap_or_default();
        let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();

        if term_program == "kitty" || term.contains("kitty") {
            Self::Kitty
        } else if term.starts_with("foot")
            || term_program == "WezTerm"
            || term.contains("sixel")
            || term.contains("mlterm")
        {
            Self::Sixel
        } else {
            Self::None
        }
    }
}
