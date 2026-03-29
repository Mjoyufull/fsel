use crate::desktop::icons;
use eyre::{eyre, Report, Result};
use image::{DynamicImage, ImageReader, RgbaImage};
use ratatui::layout::Rect;
use ratatui::Frame;
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::{Resize, StatefulImage};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::process::Stdio;
use std::sync::Mutex;

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
    current_rowid: Option<String>,
    cache: HashMap<String, StatefulProtocol>,
    cache_order: VecDeque<String>,
    cache_capacity: usize,
}

impl ImageManager {
    /// Initialize the image manager by detecting terminal capabilities
    /// This should be called after entering alternate screen
    pub fn new(picker: Picker) -> Self {
        Self {
            picker,
            current_rowid: None,
            cache: HashMap::new(),
            cache_order: VecDeque::new(),
            cache_capacity: 2000,
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

    /// Returns icon names from `candidates` that are not yet in cache
    pub fn uncached<'a>(&self, candidates: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
        candidates
            .iter()
            .filter(|(id, _)| !self.cache.contains_key(*id))
            .copied()
            .collect()
    }

    pub fn insert_protocol(&mut self, id: String, protocol: StatefulProtocol) {
        self.cache.insert(id.clone(), protocol);
        self.update_lru(&id);
        if self.cache_order.len() > self.cache_capacity {
            if let Some(old) = self.cache_order.pop_front() {
                self.cache.remove(&old);
            }
        }
    }

    pub fn picker(&self) -> Picker {
        self.picker.clone()
    }

    /// Set current image to display (must be in cache)
    pub fn set_image(&mut self, rowid: &str) {
        if self.cache.contains_key(rowid) {
            self.current_rowid = Some(rowid.to_string());
            self.update_lru(rowid);
            self.update_display_state(DisplayState::Image(rowid.to_string()));
        }
    }

    /// High-level wrapper with default cell dimensions (4x2).
    #[allow(dead_code)]
    pub async fn loadicons(&mut self, id: String, name: String, size: u16) -> Result<()> {
        self.load_desktop_icon(id, name, size, (4, 2)).await
    }

    /// Unified logic for resolving, loading, and caching desktop icons.
    #[allow(dead_code)]
    pub async fn load_desktop_icon(
        &mut self,
        id: String,
        name: String,
        size: u16,
        cells: (u32, u32),
    ) -> Result<()> {
        // 1. Unified Cache/State Check
        if self.cache.contains_key(&id) {
            self.current_rowid = Some(id.clone());
            self.update_lru(&id);
            self.update_display_state(DisplayState::Image(id));
            return Ok(());
        }

        self.update_display_state(DisplayState::Loading(id.clone()));

        // 2 & 3. Process Image and Path (Async Blocking)
        let picker = self.picker.clone();
        let name_clone = name.clone();
        let protocol_res = tokio::task::spawn_blocking(move || {
            let path = icons::lookup(&name_clone, size).ok_or_else(|| {
                eyre!("Icon '{}' not found", name_clone)
            })?;

            let (fw, fh) = picker.font_size();
            let (tw, th) = (cells.0 * fw as u32, cells.1 * fh as u32);

            let img = if path.extension().and_then(|s| s.to_str()) == Some("svg") {
                let tree = usvg::Tree::from_data(&fs::read(&path)?, &usvg::Options::default())?;
                let mut pixmap = tiny_skia::Pixmap::new(tw, th).ok_or_else(|| eyre!("Pixmap"))?;
                let s = tree.size();
                let scale = (tw as f32 / s.width()).min(th as f32 / s.height());
                resvg::render(
                    &tree,
                    tiny_skia::Transform::from_scale(scale, scale),
                    &mut pixmap.as_mut(),
                );
                DynamicImage::ImageRgba8(
                    RgbaImage::from_raw(tw, th, pixmap.take()).ok_or_else(|| eyre!("Buffer"))?,
                )
            } else {
                ImageReader::open(&path)?.decode()?
            };

            Ok::<_, Report>(picker.new_resize_protocol(img))
        })
        .await;

        let protocol = match protocol_res {
            Ok(Ok(p)) => p,
            Ok(Err(e)) => {
                let err_msg = format!("Failed to process icon '{}': {}", name, e);
                self.update_display_state(DisplayState::Failed(err_msg));
                return Err(e);
            }
            Err(e) => {
                let err = eyre!("Task join error for '{}': {}", name, e);
                self.update_display_state(DisplayState::Failed(err.to_string()));
                return Err(err);
            }
        };

        // 4. Update Cache and State
        self.cache.insert(id.clone(), protocol);
        self.update_lru(&id);
        if self.cache_order.len() > self.cache_capacity {
            if let Some(old) = self.cache_order.pop_front() {
                self.cache.remove(&old);
            }
        }
        self.current_rowid = Some(id.clone());
        self.update_display_state(DisplayState::Image(id));

        Ok(())
    }

    /// Static async helper: resolve + encode icon without needing &mut self.
    /// Called from background tasks where we don't have access to the manager.
    pub async fn load_icon_as_protocol(
        picker: Picker,
        name: String,
        size: u16,
        cells: (u32, u32),
    ) -> Result<StatefulProtocol> {
        let name_clone = name.clone();
        let protocol_res = tokio::task::spawn_blocking(move || {
            let path = icons::lookup(&name_clone, size)
                .ok_or_else(|| eyre!("Icon '{}' not found", name_clone))?;

            let (fw, fh) = picker.font_size();
            let (tw, th) = (cells.0 * fw as u32, cells.1 * fh as u32);

            let img = if path.extension().and_then(|s| s.to_str()) == Some("svg") {
                let tree = usvg::Tree::from_data(&fs::read(&path)?, &usvg::Options::default())?;
                let mut pixmap = tiny_skia::Pixmap::new(tw, th)
                    .ok_or_else(|| eyre!("Pixmap allocation failed"))?;
                let s = tree.size();
                let scale = (tw as f32 / s.width()).min(th as f32 / s.height());
                resvg::render(
                    &tree,
                    tiny_skia::Transform::from_scale(scale, scale),
                    &mut pixmap.as_mut(),
                );
                DynamicImage::ImageRgba8(
                    RgbaImage::from_raw(tw, th, pixmap.take())
                        .ok_or_else(|| eyre!("Buffer conversion failed"))?,
                )
            } else {
                ImageReader::open(&path)?.decode()?
            };

            Ok::<_, Report>(picker.new_resize_protocol(img))
        })
        .await;

        match protocol_res {
            Ok(Ok(p)) => Ok(p),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(eyre!("Task join error for '{}': {}", name, e)),
        }
    }


    pub fn render_at(&mut self, f: &mut Frame, id: &str, area: Rect) {
        if let Some(protocol) = self.cache.get_mut(id) {
            f.render_stateful_widget(StatefulImage::default(), area, protocol);
        }
    }

    /// Update LRU order for a rowid
    fn update_lru(&mut self, rowid: &str) {
        if let Some(pos) = self.cache_order.iter().position(|r| r == rowid) {
            self.cache_order.remove(pos);
        }
        self.cache_order.push_back(rowid.to_string());
    }

    /// Update the global display state (sync version)
    fn update_display_state(&self, state: DisplayState) {
        let mut lock = DISPLAY_STATE.lock().unwrap_or_else(|e| e.into_inner());
        *lock = state;
    }

    /// Load image data from cclip and prepare it for rendering
    pub async fn load_cclip_image(&mut self, rowid: &str) -> Result<()> {
        // Check cache first
        if self.cache.contains_key(rowid) {
            self.current_rowid = Some(rowid.to_string());
            self.update_lru(rowid);

            // Update display state
            self.update_display_state(DisplayState::Image(rowid.to_string()));
            return Ok(());
        }

        // Run cclip get to fetch image bytes using tokio
        let child = tokio::process::Command::new("cclip")
            .args(["get", rowid])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        const CCLIP_GET_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
        let output = match tokio::time::timeout(CCLIP_GET_TIMEOUT, child.wait_with_output()).await {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => return Err(eyre!("cclip get io error for rowid {}: {}", rowid, e)),
            Err(_) => {
                // Timed out: the future is dropped and the Child inside it is dropped,
                // which terminates the cclip process.
                return Err(eyre!(
                    "cclip get timed out after {:?} for rowid: {}",
                    CCLIP_GET_TIMEOUT,
                    rowid
                ));
            }
        };
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
        self.update_lru(rowid);

        // Enforce cache capacity
        if self.cache_order.len() > self.cache_capacity {
            if let Some(old_rowid) = self.cache_order.pop_front() {
                self.cache.remove(&old_rowid);
            }
        }

        self.current_rowid = Some(rowid.to_string());

        // Update display state
        self.update_display_state(DisplayState::Image(rowid.to_string()));

        Ok(())
    }

    /// Render the current image into the given area
    pub fn render(&mut self, f: &mut Frame, area: Rect) -> Result<()> {
        if let Some(rowid) = &self.current_rowid {
            if let Some(protocol) = self.cache.get_mut(rowid) {
                f.render_stateful_widget(
                    StatefulImage::default().resize(Resize::Fit(None)),
                    area,
                    protocol,
                );

                // Propagate encoding/resize errors
                if let Some(Err(e)) = protocol.last_encoding_result() {
                    return Err(eyre!("Image encoding failed: {}", e));
                }
            }
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        self.current_rowid = None;
        self.cache.clear();
        self.cache_order.clear();
        self.update_display_state(DisplayState::Empty);
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
            match picker.protocol_type() {
                ProtocolType::Kitty | ProtocolType::Iterm2 => return Self::Kitty,
                ProtocolType::Sixel => return Self::Sixel,
                ProtocolType::Halfblocks => return Self::None,
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
