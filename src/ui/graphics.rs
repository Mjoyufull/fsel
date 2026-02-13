use ratatui::layout::Rect;
use ratatui::Frame;
use ratatui_image::picker::Picker;
use ratatui_image::{Resize, StatefulImage};
use std::io::{self, Read};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use eyre::{eyre, Result};

use ratatui_image::protocol::StatefulProtocol;

/// Combined display state to track what's currently on screen
#[derive(Debug, Clone, PartialEq)]
pub enum DisplayState {
    /// No content displayed
    Empty,
    /// Image content is displayed with area and rowid
    Image(Rect, String),
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
    pub fn new() -> io::Result<Self> {
        let picker = Picker::from_query_stdio()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{:?}", e)))?;
        Ok(Self {
            picker,
            protocol: None,
        })
    }

    /// Check if the terminal supports any high-resolution graphics protocol
    pub fn supports_graphics(&self) -> bool {
        use ratatui_image::picker::ProtocolType;
        match self.picker.protocol_type() {
            ProtocolType::Halfblocks => false,
            _ => true,
        }
    }

    /// Is the current protocol Kitty? (Used for specific clearing logic)
    pub fn is_kitty(&self) -> bool {
        use ratatui_image::picker::ProtocolType;
        matches!(self.picker.protocol_type(), ProtocolType::Kitty)
    }

    /// Is the current protocol Sixel?
    pub fn is_sixel(&self) -> bool {
        use ratatui_image::picker::ProtocolType;
        matches!(self.picker.protocol_type(), ProtocolType::Sixel)
    }

    /// Load image data from cclip and prepare it for rendering
    pub fn load_cclip_image(&mut self, rowid: &str) -> Result<()> {
        // Run cclip get to fetch image bytes
        let mut child = Command::new("cclip")
            .args(["get", rowid])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let mut bytes = Vec::new();
        if let Some(mut stdout) = child.stdout.take() {
            stdout.read_to_end(&mut bytes)?;
        }
        child.wait()?;

        if bytes.is_empty() {
            return Err(eyre!("No data received from cclip get {}", rowid));
        }

        // Decode image bytes
        let dyn_img = image::load_from_memory(&bytes)?;

        // Create new protocol state
        self.protocol = Some(self.picker.new_resize_protocol(dyn_img));

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
    pub fn detect() -> Self {
        let term = std::env::var("TERM").unwrap_or_default();
        let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();

        if term_program == "kitty" || term.contains("kitty") {
            Self::Kitty
        } else if term.starts_with("foot") || term.contains("xterm") || term_program == "WezTerm" {
            Self::Sixel
        } else {
            Self::None
        }
    }
}
