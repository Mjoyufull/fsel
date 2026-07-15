use crate::cli::Opts;
use crate::core::state::State;
use crate::desktop::IconResolver;
use crate::ui::{AppIconPreview, GraphicsAdapter, ImageManager};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub(super) struct IconRuntime {
    enabled: bool,
    adapter: GraphicsAdapter,
    resolver: Arc<Mutex<IconResolver>>,
    image_manager: ImageManager,
    selected_icon: Option<String>,
    current_key: Option<String>,
    needs_terminal_clear: bool,
    generation: u64,
    active_request: Option<JoinHandle<()>>,
    result_tx: mpsc::UnboundedSender<IconResult>,
    result_rx: mpsc::UnboundedReceiver<IconResult>,
}

pub(super) struct IconResult {
    generation: u64,
    path: Result<Option<PathBuf>, String>,
}

impl IconRuntime {
    pub(super) fn new(cli: &Opts) -> Self {
        let (result_tx, result_rx) = mpsc::unbounded_channel();
        let adapter = GraphicsAdapter::detect(None);
        Self {
            enabled: cli.desktop_icon_mode.shows_preview(),
            adapter,
            resolver: Arc::new(Mutex::new(IconResolver::from_environment(
                cli.desktop_icon_theme.as_deref(),
                cli.desktop_icon_size,
            ))),
            image_manager: ImageManager::new(adapter.picker()),
            selected_icon: None,
            current_key: None,
            needs_terminal_clear: false,
            generation: 0,
            active_request: None,
            result_tx,
            result_rx,
        }
    }

    pub(super) fn request_if_changed(&mut self, state: &State) {
        if !self.enabled {
            return;
        }

        let icon = state
            .selected
            .and_then(|selected| state.shown.get(selected))
            .and_then(|app| app.icon.clone());
        if self.selected_icon == icon {
            return;
        }

        if let Some(task) = self.active_request.take() {
            task.abort();
        }
        self.generation = self.generation.wrapping_add(1);
        self.selected_icon.clone_from(&icon);
        self.needs_terminal_clear =
            self.current_key.is_some() && !matches!(self.adapter, GraphicsAdapter::None);
        self.current_key = None;

        let Some(icon) = icon else {
            return;
        };
        let generation = self.generation;
        let resolver = Arc::clone(&self.resolver);
        let result_tx = self.result_tx.clone();
        self.active_request = Some(tokio::task::spawn_blocking(move || {
            let path = resolver
                .lock()
                .map_err(|_| "Desktop icon resolver lock was poisoned".to_string())
                .map(|mut resolver| resolver.resolve(&icon));
            let _ = result_tx.send(IconResult { generation, path });
        }));
    }

    pub(super) async fn next_result(&mut self) -> Option<IconResult> {
        self.result_rx.recv().await
    }

    pub(super) async fn apply_result(&mut self, result: IconResult) {
        if result.generation != self.generation {
            return;
        }
        self.active_request = None;

        let Ok(Some(path)) = result.path else {
            return;
        };
        let key = path.to_string_lossy().into_owned();
        if self
            .image_manager
            .load_image_path(&key, &path)
            .await
            .is_ok()
        {
            self.current_key = Some(key);
        }
    }

    pub(super) fn preview(&mut self) -> Option<AppIconPreview<'_>> {
        let key = self.current_key.as_deref()?;
        Some(AppIconPreview {
            image_manager: &mut self.image_manager,
            key,
        })
    }

    pub(super) fn take_terminal_clear(&mut self) -> bool {
        std::mem::take(&mut self.needs_terminal_clear)
    }
}

impl Drop for IconRuntime {
    fn drop(&mut self) {
        if let Some(task) = self.active_request.take() {
            task.abort();
        }
    }
}
