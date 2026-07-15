use crate::cli::Opts;
use crate::core::state::State;
use crate::desktop::IconResolver;
use crate::ui::{AppIconPreview, GraphicsAdapter, ImageManager};
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub(super) struct IconRuntime {
    enabled: bool,
    adapter: GraphicsAdapter,
    image_manager: ImageManager,
    selected_icon: Option<String>,
    current_key: Option<String>,
    needs_terminal_clear: bool,
    generation: u64,
    request_tx: mpsc::UnboundedSender<Option<IconRequest>>,
    worker: JoinHandle<()>,
    result_rx: mpsc::UnboundedReceiver<IconResult>,
}

#[derive(Clone)]
struct IconRequest {
    generation: u64,
    icon: String,
}

pub(super) struct IconResult {
    generation: u64,
    prepared: Result<Option<PreparedIcon>, String>,
}

struct PreparedIcon {
    key: String,
    protocol: Box<StatefulProtocol>,
}

impl IconRuntime {
    pub(super) fn new(cli: &Opts) -> Self {
        let (result_tx, result_rx) = mpsc::unbounded_channel();
        let (request_tx, mut request_rx) = mpsc::unbounded_channel::<Option<IconRequest>>();
        let adapter = GraphicsAdapter::detect(None);
        let picker = adapter.picker();
        let worker_picker = picker.clone();
        let mut resolver = IconResolver::from_environment(
            cli.desktop_icon_theme.as_deref(),
            cli.desktop_icon_size,
        );
        let worker = tokio::task::spawn_blocking(move || {
            while let Some(mut request) = request_rx.blocking_recv() {
                while let Ok(latest) = request_rx.try_recv() {
                    request = latest;
                }
                let Some(request) = request else {
                    continue;
                };
                let prepared = prepare_icon(&mut resolver, worker_picker.clone(), &request.icon);
                let _ = result_tx.send(IconResult {
                    generation: request.generation,
                    prepared,
                });
            }
        });
        Self {
            enabled: cli.desktop_icon_mode.shows_preview(),
            adapter,
            image_manager: ImageManager::new(picker),
            selected_icon: None,
            current_key: None,
            needs_terminal_clear: false,
            generation: 0,
            request_tx,
            worker,
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

        self.generation = self.generation.wrapping_add(1);
        self.selected_icon.clone_from(&icon);
        self.needs_terminal_clear =
            self.current_key.is_some() && !matches!(self.adapter, GraphicsAdapter::None);
        self.current_key = None;

        let Some(icon) = icon else {
            let _ = self.request_tx.send(None);
            return;
        };
        let _ = self.request_tx.send(Some(IconRequest {
            generation: self.generation,
            icon,
        }));
    }

    pub(super) async fn next_result(&mut self) -> Option<IconResult> {
        self.result_rx.recv().await
    }

    pub(super) fn apply_result(&mut self, result: IconResult) {
        if result.generation != self.generation {
            return;
        }
        let Ok(Some(prepared)) = result.prepared else {
            return;
        };
        self.image_manager
            .insert_protocol(prepared.key.clone(), *prepared.protocol);
        self.current_key = Some(prepared.key);
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
        self.worker.abort();
    }
}

fn prepare_icon(
    resolver: &mut IconResolver,
    picker: Picker,
    icon: &str,
) -> Result<Option<PreparedIcon>, String> {
    let Some(path) = resolver.resolve(icon) else {
        return Ok(None);
    };
    prepare_resolved_icon(picker, path).map(Some)
}

fn prepare_resolved_icon(picker: Picker, path: PathBuf) -> Result<PreparedIcon, String> {
    let key = path.to_string_lossy().into_owned();
    let protocol = ImageManager::prepare_image_path(picker, &path)
        .map_err(|error| format!("Failed to load desktop icon {}: {error}", path.display()))?;
    Ok(PreparedIcon {
        key,
        protocol: Box::new(protocol),
    })
}
