use eyre::{eyre, Result};
use ratatui::layout::Rect;
use ratatui::Frame;
use ratatui_image::picker::Picker;
use ratatui_image::{Resize, StatefulImage};
use std::collections::HashMap;
use std::process::Stdio;

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
pub static DISPLAY_STATE: tokio::sync::Mutex<DisplayState> =
    tokio::sync::Mutex::const_new(DisplayState::Empty);

/// Manages image loading and rendering using ratatui-image
pub struct ImageManager {
    picker: Picker,
    current_rowid: Option<String>,
    cache: HashMap<String, StatefulProtocol>,
}

impl ImageManager {
    /// Initialize the image manager by detecting terminal capabilities
    /// This should be called after entering alternate screen
    pub fn new(picker: Picker) -> Self {
        Self {
            picker,
            current_rowid: None,
            cache: HashMap::new(),
        }
    }

    pub fn is_sixel(&self) -> bool {
        matches!(self.picker.protocol_type(), ProtocolType::Sixel)
    }

    /// Check if an image is already in cache
    pub fn is_cached(&self, rowid: &str) -> bool {
        self.cache.contains_key(rowid)
    }

    /// Check if the terminal supports any high-resolution graphics protocol
    pub fn supports_graphics(&self) -> bool {
        !matches!(self.picker.protocol_type(), ProtocolType::Halfblocks)
    }

    /// Set current image to display (must be in cache)
    pub fn set_image(&mut self, rowid: &str) {
        if self.cache.contains_key(rowid) {
            self.current_rowid = Some(rowid.to_string());
        }
    }

    /// Load image data from cclip and prepare it for rendering
    pub async fn load_cclip_image(&mut self, rowid: &str) -> Result<()> {
        // Check cache first
        if self.cache.contains_key(rowid) {
            self.current_rowid = Some(rowid.to_string());
            return Ok(());
        }

        // Run cclip get to fetch image bytes using tokio
        let child = tokio::process::Command::new("cclip")
            .args(["get", rowid])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let output = child.wait_with_output().await?;
        if !output.status.success() {
            return Err(eyre!("cclip get failed for rowid: {}", rowid));
        }

        let bytes = output.stdout;
        if bytes.is_empty() {
            return Err(eyre!("No data received from cclip get {}", rowid));
        }

        let picker = self.picker.clone();
        let protocol = tokio::task::spawn_blocking(move || {
            let dyn_img = image::load_from_memory(&bytes)?;
            Ok::<_, eyre::Report>(picker.new_resize_protocol(dyn_img))
        })
        .await??;

        // Add to cache and set as current
        self.cache.insert(rowid.to_string(), protocol);
        self.current_rowid = Some(rowid.to_string());

        Ok(())
    }

    /// Render the current image into the given area
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        if let Some(rowid) = &self.current_rowid {
            if let Some(protocol) = self.cache.get_mut(rowid) {
                f.render_stateful_widget(
                    StatefulImage::default().resize(Resize::Fit(None)),
                    area,
                    protocol,
                );
            }
        }
    }

    /// Clear the current image from display
    pub fn clear(&mut self) {
        self.current_rowid = None;
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
                ProtocolType::Kitty | ProtocolType::Iterm2 => return Self::Kitty,
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
