use super::state::CclipOptions;
use crate::ui::{AsyncInput, DISPLAY_STATE, DisplayState, DmenuUI, ImageManager, TagMode};
use eyre::{Result, WrapErr};
use futures::FutureExt;
use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use std::collections::HashSet;
use std::io;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

pub(super) struct ImageRuntime {
    image_manager: Option<Arc<Mutex<ImageManager>>>,
    failed_rowids: Arc<Mutex<HashSet<String>>>,
    redraw_tx: mpsc::UnboundedSender<()>,
    pub(super) redraw_rx: mpsc::UnboundedReceiver<()>,
    image_preview_enabled: bool,
    cached_is_sixel: bool,
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
        if let Some(manager) = &image_manager {
            let manager_lock = manager.lock().await;
            image_preview_enabled = options.image_preview_enabled(manager_lock.supports_graphics());
            cached_is_sixel = manager_lock.is_sixel();
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
            previous_was_image: false,
            current_is_image: false,
            current_rowid: None,
            force_buffer_sync: false,
        }
    }

    pub(super) fn preview_enabled(&self) -> bool {
        self.image_preview_enabled
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
        self.previous_was_image = false;
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

    pub(super) async fn show_fullscreen_preview(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stderr>>,
        input: &mut AsyncInput,
    ) -> Result<()> {
        if !self.current_is_image {
            return Ok(());
        }

        let Some(manager) = &mut self.image_manager else {
            return Ok(());
        };

        let mut consecutive_errors: u8 = 0;
        loop {
            let mut render_error = Ok(());
            terminal.draw(|frame| {
                if let Ok(mut manager_lock) = manager.try_lock()
                    && let Err(error) = manager_lock.render(frame, frame.area())
                {
                    render_error = Err(error);
                }
            })?;
            render_error?;

            match input.next().await {
                Some(crate::ui::InputEvent::Input(key_event)) => {
                    consecutive_errors = 0;
                    match (key_event.code, key_event.modifiers) {
                        (crossterm::event::KeyCode::Esc, _)
                        | (crossterm::event::KeyCode::Char('q'), _)
                        | (
                            crossterm::event::KeyCode::Char('c'),
                            crossterm::event::KeyModifiers::CONTROL,
                        ) => break,
                        _ => {}
                    }
                }
                Some(_) => {
                    consecutive_errors = 0;
                }
                None => {
                    consecutive_errors += 1;
                    if consecutive_errors >= 3 {
                        break;
                    }
                }
            }
        }

        terminal.clear().wrap_err("Failed to clear terminal")?;
        self.restore_display_state();
        self.request_buffer_sync();
        Ok(())
    }

    fn restore_display_state(&self) {
        let mut state = DISPLAY_STATE
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *state = match &self.current_rowid {
            Some(rowid) => DisplayState::Image(rowid.clone()),
            None => DisplayState::Empty,
        };
    }

    async fn ensure_image_loaded(&mut self) {
        let Some(rowid) = self.current_rowid.clone() else {
            return;
        };
        let Some(manager) = &mut self.image_manager else {
            return;
        };

        let mut already_loaded = false;
        let mut is_loading = false;

        if let Ok(mut manager_lock) = manager.try_lock()
            && manager_lock.is_cached(&rowid)
        {
            manager_lock.set_image(&rowid);
            already_loaded = true;
        }

        if !already_loaded {
            let state = DISPLAY_STATE
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            match &*state {
                DisplayState::Image(id) if id == &rowid => already_loaded = true,
                DisplayState::Loading(id) if id == &rowid => is_loading = true,
                _ => {}
            }
        }

        if already_loaded || is_loading {
            return;
        }

        let is_failed = self.failed_rowids.lock().await.contains(&rowid);
        if is_failed {
            return;
        }

        {
            let mut state = DISPLAY_STATE
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            *state = DisplayState::Loading(rowid.clone());
        }

        let manager_clone = manager.clone();
        let failed_rowids = Arc::clone(&self.failed_rowids);
        let redraw_tx = self.redraw_tx.clone();
        tokio::spawn(async move {
            let result = AssertUnwindSafe(async {
                let mut manager_lock = manager_clone.lock().await;
                let load_result = manager_lock.load_cclip_image(&rowid).await;
                drop(manager_lock);
                load_result
            })
            .catch_unwind()
            .await;

            match result {
                Ok(Ok(_)) => {
                    failed_rowids.lock().await.remove(&rowid);
                    let mut state = DISPLAY_STATE
                        .lock()
                        .unwrap_or_else(|error| error.into_inner());
                    *state = DisplayState::Image(rowid.clone());
                }
                Ok(Err(error)) => {
                    failed_rowids.lock().await.insert(rowid.clone());
                    if let Ok(mut manager_lock) = manager_clone.try_lock() {
                        manager_lock.clear();
                    }
                    let mut state = DISPLAY_STATE
                        .lock()
                        .unwrap_or_else(|error| error.into_inner());
                    *state = DisplayState::Failed(error.to_string());
                }
                Err(_) => {
                    failed_rowids.lock().await.insert(rowid.clone());
                    if let Ok(mut manager_lock) = manager_clone.try_lock() {
                        manager_lock.clear();
                    }
                    let mut state = DISPLAY_STATE
                        .lock()
                        .unwrap_or_else(|error| error.into_inner());
                    *state = DisplayState::Failed("Task panicked during image load".to_string());
                }
            }

            let _ = redraw_tx.send(());
        });
    }
}
