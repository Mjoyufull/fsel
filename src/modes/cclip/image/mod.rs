mod fullscreen;
mod loading;

use super::state::CclipOptions;
use crate::ui::{DISPLAY_STATE, DisplayState, DmenuUI, ImageManager, TagMode};
use eyre::Result;
use ratatui::Frame;
use ratatui::layout::Rect;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

pub(super) struct ImageRuntime {
    image_manager: Option<Arc<Mutex<ImageManager>>>,
    failed_rowids: Arc<Mutex<HashSet<String>>>,
    redraw_tx: mpsc::UnboundedSender<()>,
    pub(super) redraw_rx: mpsc::UnboundedReceiver<()>,
    image_preview_enabled: bool,
    cached_is_sixel: bool,
    detected_adapter: crate::ui::GraphicsAdapter,
    previous_was_image: bool,
    current_is_image: bool,
    current_rowid: Option<String>,
    force_buffer_sync: bool,
}

impl ImageRuntime {
    pub(super) async fn new(options: &CclipOptions, ui: &mut DmenuUI<'_>) -> Self {
        let picker = ratatui_image::picker::Picker::from_query_stdio().ok();
        let image_manager = Some(Arc::new(Mutex::new(ImageManager::new(
            picker
                .clone()
                .unwrap_or_else(ratatui_image::picker::Picker::halfblocks),
        ))));
        let failed_rowids = Arc::new(Mutex::new(HashSet::<String>::new()));
        let (redraw_tx, redraw_rx) = mpsc::unbounded_channel::<()>();

        let mut image_preview_enabled = false;
        let mut cached_is_sixel = false;
        let mut detected_adapter = crate::ui::GraphicsAdapter::None;
        if let Some(manager) = &image_manager {
            let manager_lock = manager.lock().await;
            image_preview_enabled = options.image_preview_enabled(manager_lock.supports_graphics());
            cached_is_sixel = manager_lock.is_sixel();
            detected_adapter = crate::ui::GraphicsAdapter::detect(Some(manager_lock.picker()));
        }

        if picker.is_none() && image_preview_enabled {
            ui.set_temp_message(
                "image_preview enabled but terminal graphics detection failed (using half-block fallback)".to_string(),
            );
        }

        Self {
            image_manager,
            failed_rowids,
            redraw_tx,
            redraw_rx,
            image_preview_enabled,
            cached_is_sixel,
            detected_adapter,
            previous_was_image: false,
            current_is_image: false,
            current_rowid: None,
            force_buffer_sync: false,
        }
    }

    pub(super) fn preview_enabled(&self) -> bool {
        self.image_preview_enabled
    }

    pub(super) fn detected_adapter(&self) -> crate::ui::GraphicsAdapter {
        self.detected_adapter
    }

    pub(super) fn current_is_image(&self) -> bool {
        self.current_is_image
    }

    pub(super) fn request_buffer_sync(&mut self) {
        self.force_buffer_sync = true;
    }

    pub(super) fn consume_buffer_sync(&mut self) -> bool {
        let force_buffer_sync = self.force_buffer_sync;
        self.force_buffer_sync = false;
        force_buffer_sync
    }

    pub(super) fn needs_terminal_clear(&self) -> bool {
        self.image_preview_enabled
            && self.cached_is_sixel
            && self.previous_was_image != self.current_is_image
    }

    pub(super) fn finish_draw(&mut self) {
        self.previous_was_image = self.current_is_image;
    }

    pub(super) fn clear_inline_image(&mut self) {
        if let Some(manager) = &mut self.image_manager
            && let Ok(mut manager_lock) = manager.try_lock()
        {
            manager_lock.clear();
        }
        if let Ok(mut failed) = self.failed_rowids.try_lock() {
            failed.clear();
        }
        self.current_is_image = false;
        self.current_rowid = None;
    }

    pub(super) async fn prepare_for_draw(&mut self, ui: &DmenuUI<'_>) {
        self.current_is_image = false;
        self.current_rowid = None;

        if self.image_preview_enabled
            && matches!(ui.tag_mode, TagMode::Normal)
            && let Some(selected) = ui.selected
            && selected < ui.shown.len()
        {
            let item = &ui.shown[selected];
            if ui.is_cclip_image_item(item) {
                self.current_is_image = true;
                self.current_rowid = ui.get_cclip_rowid(item);
            }
        }

        if !self.image_preview_enabled {
            return;
        }

        if self.current_is_image {
            self.ensure_image_loaded().await;
        } else if self.previous_was_image {
            self.clear_inline_image();
        }
    }

    pub(super) fn render_inline_image(&mut self, frame: &mut Frame, area: Rect) -> Result<bool> {
        if !self.current_is_image {
            return Ok(false);
        }

        if let Some(manager) = &mut self.image_manager
            && let Ok(mut manager_lock) = manager.try_lock()
        {
            manager_lock.render(frame, area)?;
            return Ok(true);
        }

        Ok(false)
    }

    fn restore_display_state(&self) {
        let mut state = DISPLAY_STATE
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(rowid) = &self.current_rowid {
            let is_cached = self
                .image_manager
                .as_ref()
                .and_then(|manager| manager.try_lock().ok())
                .is_some_and(|manager| manager.is_cached(rowid));

            if is_cached {
                *state = DisplayState::Image(rowid.clone());
            }
        } else {
            *state = DisplayState::Empty;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ImageRuntime;
    use crate::ui::{DISPLAY_STATE, DisplayState, GraphicsAdapter};
    use std::collections::HashSet;
    use std::sync::Arc;
    use tokio::sync::{Mutex, mpsc};

    #[test]
    fn restore_display_state_keeps_loading_state_for_uncached_selection() {
        let (redraw_tx, redraw_rx) = mpsc::unbounded_channel();
        let runtime = ImageRuntime {
            image_manager: None,
            failed_rowids: Arc::new(Mutex::new(HashSet::new())),
            redraw_tx,
            redraw_rx,
            image_preview_enabled: true,
            cached_is_sixel: false,
            detected_adapter: GraphicsAdapter::None,
            previous_was_image: false,
            current_is_image: true,
            current_rowid: Some("42".to_string()),
            force_buffer_sync: false,
        };

        {
            let mut state = DISPLAY_STATE
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            *state = DisplayState::Loading("42".to_string());
        }

        runtime.restore_display_state();

        let state = DISPLAY_STATE
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        assert_eq!(*state, DisplayState::Loading("42".to_string()));
    }
}
