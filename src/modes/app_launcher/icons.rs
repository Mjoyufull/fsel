use crate::cli::Opts;
use crate::core::state::State;
use crate::desktop::IconResolver;
use crate::ui::{AppIconPreview, GraphicsAdapter, ImageManager};
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use std::collections::HashMap;
#[cfg(unix)]
use std::io::{self, Write};
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub(super) struct IconRuntime {
    enabled: bool,
    adapter: GraphicsAdapter,
    image_manager: ImageManager,
    selected_icon: Option<String>,
    current_key: Option<String>,
    icon_keys: HashMap<String, String>,
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
    icon: String,
    prepared: Result<Option<PreparedIcon>, String>,
}

struct PreparedIcon {
    key: String,
    protocol: Box<StatefulProtocol>,
    decoded_bytes: u64,
}

impl IconRuntime {
    pub(super) fn new(cli: &Opts) -> Self {
        let (result_tx, result_rx) = mpsc::unbounded_channel();
        let (request_tx, mut request_rx) = mpsc::unbounded_channel::<Option<IconRequest>>();
        let enabled = cli.desktop_icon_mode.shows_preview();
        let fallback_adapter = GraphicsAdapter::detect(None);
        let picker = if enabled {
            picker_from_terminal_output(&fallback_adapter.picker())
        } else {
            fallback_adapter.picker()
        };
        let adapter = GraphicsAdapter::detect(Some(&picker));
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
                    icon: request.icon,
                    prepared,
                });
            }
        });
        Self {
            enabled,
            adapter,
            image_manager: ImageManager::new(picker).with_cache_weight_limit(128 * 1024 * 1024),
            selected_icon: None,
            current_key: None,
            icon_keys: HashMap::new(),
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
        if let Some(key) = self.icon_keys.get(&icon)
            && self.image_manager.is_cached(key)
        {
            self.current_key = Some(key.clone());
            return;
        }
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
        self.image_manager.insert_protocol_with_weight(
            prepared.key.clone(),
            *prepared.protocol,
            prepared.decoded_bytes,
        );
        self.icon_keys.insert(result.icon, prepared.key.clone());
        self.current_key = Some(prepared.key);
    }

    pub(super) fn preview(&mut self) -> Option<AppIconPreview<'_>> {
        let key = self.current_key.as_deref()?;
        Some(AppIconPreview {
            image_manager: &mut self.image_manager,
            key,
        })
    }

    pub(super) fn clear_failed_preview(&mut self) {
        if self.current_key.take().is_some() {
            self.needs_terminal_clear |= !matches!(self.adapter, GraphicsAdapter::None);
        }
    }

    pub(super) fn take_terminal_clear(&mut self) -> bool {
        std::mem::take(&mut self.needs_terminal_clear)
    }
}

#[cfg(unix)]
fn picker_from_terminal_output(fallback: &Picker) -> Picker {
    let _ = io::stdout().flush();
    // The picker library queries through stdout/stdin. The launcher owns the
    // terminal on stderr and reserves stdout for selected-command output, so
    // redirect only the probe's writes while keeping its stdin response path.
    let Ok(saved_stdout) = rustix::io::dup(rustix::stdio::stdout()) else {
        return fallback.clone();
    };
    if rustix::stdio::dup2_stdout(rustix::stdio::stderr()).is_err() {
        return fallback.clone();
    }

    struct StdoutGuard(rustix::fd::OwnedFd);
    impl Drop for StdoutGuard {
        fn drop(&mut self) {
            let _ = rustix::stdio::dup2_stdout(&self.0);
        }
    }

    let guard = StdoutGuard(saved_stdout);
    let picker = Picker::from_query_stdio().unwrap_or_else(|_| fallback.clone());
    drop(guard);
    picker
}

#[cfg(not(unix))]
fn picker_from_terminal_output(fallback: &Picker) -> Picker {
    fallback.clone()
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
    let (protocol, decoded_bytes) = ImageManager::prepare_image_path_with_weight(picker, &path)
        .map_err(|error| format!("Failed to load desktop icon {}: {error}", path.display()))?;
    Ok(PreparedIcon {
        key,
        protocol: Box::new(protocol),
        decoded_bytes,
    })
}
