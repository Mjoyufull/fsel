use crate::cli::Opts;
use crate::core::state::State;
use crate::desktop::IconResolver;
use crate::ui::{AppIcons, GraphicsAdapter, ImageManager};
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use std::collections::{HashMap, HashSet, VecDeque};
#[cfg(unix)]
use std::io::{self, Write};
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub(super) struct IconRuntime {
    preview_enabled: bool,
    list_enabled: bool,
    adapter: GraphicsAdapter,
    image_manager: ImageManager,
    selected_icon: Option<String>,
    current_key: Option<String>,
    icon_keys: HashMap<String, String>,
    preview_pending: bool,
    preview_failed: bool,
    list_signature: Vec<Option<String>>,
    list_rendered: bool,
    list_keys: HashMap<String, String>,
    failed_list_icons: HashSet<String>,
    list_inflight: HashSet<String>,
    list_attempted: HashSet<String>,
    needs_terminal_clear: bool,
    preview_generation: u64,
    list_generation: u64,
    request_tx: mpsc::UnboundedSender<WorkRequest>,
    worker: JoinHandle<()>,
    result_rx: mpsc::UnboundedReceiver<IconResult>,
}

#[derive(Clone)]
struct IconRequest {
    generation: u64,
    icon: String,
}

struct WorkRequest {
    preview_generation: u64,
    preview: Option<IconRequest>,
    list_generation: u64,
    list_icons: VecDeque<String>,
}

pub(super) enum IconResult {
    Preview {
        generation: u64,
        prepared: Result<Option<PreparedIcon>, String>,
    },
    List {
        generation: u64,
        icon: String,
        prepared: Result<Option<PreparedIcon>, String>,
    },
}

pub(super) struct PreparedIcon {
    key: String,
    protocol: Box<StatefulProtocol>,
    decoded_bytes: u64,
}

impl IconRuntime {
    pub(super) fn new(cli: &Opts) -> Self {
        let (result_tx, result_rx) = mpsc::unbounded_channel();
        let (request_tx, request_rx) = mpsc::unbounded_channel();
        let preview_enabled = cli.desktop_icon_mode.shows_preview();
        let list_enabled = cli.desktop_icon_mode.shows_list();
        let fallback_adapter = GraphicsAdapter::detect(None);
        let picker = if preview_enabled || list_enabled {
            picker_from_terminal_output(&fallback_adapter.picker())
        } else {
            fallback_adapter.picker()
        };
        let adapter = GraphicsAdapter::detect(Some(&picker));
        let resolver = IconResolver::from_environment(
            cli.desktop_icon_theme.as_deref(),
            cli.desktop_icon_size,
        );
        let worker = spawn_worker(resolver, picker.clone(), request_rx, result_tx);

        Self {
            preview_enabled,
            list_enabled,
            adapter,
            image_manager: ImageManager::new(picker).with_cache_weight_limit(128 * 1024 * 1024),
            selected_icon: None,
            current_key: None,
            icon_keys: HashMap::new(),
            preview_pending: false,
            preview_failed: false,
            list_signature: Vec::new(),
            list_rendered: false,
            list_keys: HashMap::new(),
            failed_list_icons: HashSet::new(),
            list_inflight: HashSet::new(),
            list_attempted: HashSet::new(),
            needs_terminal_clear: false,
            preview_generation: 0,
            list_generation: 0,
            request_tx,
            worker,
            result_rx,
        }
    }

    pub(super) fn request_if_changed(&mut self, state: &State, max_visible: usize) {
        let previous_preview_generation = self.preview_generation;
        let preview = self.preview_request(state);
        let preview_changed = self.preview_generation != previous_preview_generation;
        let previous_list_generation = self.list_generation;
        let list_icons = self.list_requests(state, max_visible);
        let list_changed = self.list_generation != previous_list_generation;
        if !preview_changed && !list_changed && list_icons.is_empty() {
            return;
        }
        let _ = self.request_tx.send(WorkRequest {
            preview_generation: self.preview_generation,
            preview,
            list_generation: self.list_generation,
            list_icons: list_icons.into(),
        });
    }

    fn preview_request(&mut self, state: &State) -> Option<IconRequest> {
        if !self.preview_enabled {
            return None;
        }

        let icon = state
            .selected
            .and_then(|selected| state.shown.get(selected))
            .and_then(|app| app.icon.clone());
        let cached = self
            .current_key
            .as_deref()
            .is_some_and(|key| self.image_manager.is_cached(key));
        if self.selected_icon == icon
            && (icon.is_none() || cached || self.preview_pending || self.preview_failed)
        {
            return None;
        }

        self.preview_generation = self.preview_generation.wrapping_add(1);
        self.selected_icon.clone_from(&icon);
        self.preview_pending = icon.is_some();
        self.preview_failed = false;
        self.needs_terminal_clear |=
            self.current_key.is_some() && !matches!(self.adapter, GraphicsAdapter::None);
        self.current_key = None;

        let icon = icon?;
        if let Some(key) = self.icon_keys.get(&icon)
            && self.image_manager.is_cached(key)
        {
            self.preview_pending = false;
            self.current_key = Some(key.clone());
            return None;
        }
        Some(IconRequest {
            generation: self.preview_generation,
            icon,
        })
    }

    fn list_requests(&mut self, state: &State, max_visible: usize) -> Vec<String> {
        if !self.list_enabled {
            return Vec::new();
        }

        self.image_manager
            .ensure_cache_capacity(max_visible.saturating_add(1));
        let signature = state
            .shown
            .iter()
            .skip(state.scroll_offset)
            .take(max_visible)
            .map(|app| app.icon.clone())
            .collect::<Vec<_>>();
        let signature_changed = self.list_signature != signature;
        let all_ready = signature.iter().flatten().all(|icon| {
            let cached = self
                .list_keys
                .get(icon)
                .is_some_and(|key| self.image_manager.is_cached(key));
            !should_queue_list_icon(
                self.failed_list_icons.contains(icon),
                self.list_inflight.contains(icon),
                self.list_attempted.contains(icon),
                cached,
            )
        });
        if !signature_changed && all_ready {
            return Vec::new();
        }

        if signature_changed {
            self.needs_terminal_clear |=
                rendered_icons_need_clear(self.adapter, self.list_rendered);
            self.list_generation = self.list_generation.wrapping_add(1);
            self.list_inflight.clear();
            self.list_attempted.clear();
            self.list_signature = signature;
            self.list_rendered = false;
        } else if !all_ready && self.list_inflight.is_empty() {
            self.list_generation = self.list_generation.wrapping_add(1);
        }

        let mut unique = HashSet::new();
        let icons = self
            .list_signature
            .iter()
            .flatten()
            .filter(|icon| unique.insert((*icon).clone()))
            .filter(|icon| {
                let cached = self
                    .list_keys
                    .get(*icon)
                    .is_some_and(|key| self.image_manager.is_cached(key));
                should_queue_list_icon(
                    self.failed_list_icons.contains(*icon),
                    self.list_inflight.contains(*icon),
                    self.list_attempted.contains(*icon),
                    cached,
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        self.list_inflight.extend(icons.iter().cloned());
        icons
    }

    pub(super) async fn next_result(&mut self) -> Option<IconResult> {
        self.result_rx.recv().await
    }

    pub(super) fn apply_result(&mut self, result: IconResult) {
        match result {
            IconResult::Preview {
                generation,
                prepared,
            } => self.apply_preview_result(generation, prepared),
            IconResult::List {
                generation,
                icon,
                prepared,
            } => self.apply_list_result(generation, icon, prepared),
        }
    }

    fn apply_preview_result(
        &mut self,
        generation: u64,
        prepared: Result<Option<PreparedIcon>, String>,
    ) {
        if generation != self.preview_generation {
            return;
        }
        self.preview_pending = false;
        let Ok(Some(prepared)) = prepared else {
            self.preview_failed = true;
            return;
        };
        self.preview_failed = false;
        self.image_manager.insert_protocol_with_weight(
            prepared.key.clone(),
            *prepared.protocol,
            prepared.decoded_bytes,
        );
        if let Some(icon) = &self.selected_icon {
            self.icon_keys.insert(icon.clone(), prepared.key.clone());
        }
        self.current_key = Some(prepared.key);
    }

    fn apply_list_result(
        &mut self,
        generation: u64,
        icon: String,
        prepared: Result<Option<PreparedIcon>, String>,
    ) {
        if generation != self.list_generation {
            return;
        }
        self.list_inflight.remove(&icon);
        self.list_attempted.insert(icon.clone());
        let Ok(Some(prepared)) = prepared else {
            self.failed_list_icons.insert(icon);
            return;
        };
        self.image_manager.insert_protocol_with_weight(
            prepared.key.clone(),
            *prepared.protocol,
            prepared.decoded_bytes,
        );
        self.list_keys.insert(icon, prepared.key);
    }

    pub(super) fn render_state(&mut self) -> Option<AppIcons<'_>> {
        if !self.preview_enabled && !self.list_enabled {
            return None;
        }
        let preview_key = self
            .current_key
            .as_deref()
            .filter(|key| self.image_manager.is_cached(key));
        Some(AppIcons {
            image_manager: &mut self.image_manager,
            preview_key,
            list_keys: &self.list_keys,
            failed_list_icons: &mut self.failed_list_icons,
        })
    }

    pub(super) fn handle_render_failures(&mut self, preview_failed: bool, list_failed: bool) {
        if preview_failed {
            self.current_key = None;
            self.preview_failed = true;
        }
        if preview_failed || list_failed {
            self.needs_terminal_clear |= !matches!(self.adapter, GraphicsAdapter::None);
        }
    }

    pub(super) fn finish_render(&mut self, list_rendered: bool) {
        self.list_rendered = list_rendered;
    }

    pub(super) fn handle_terminal_resize(&mut self) {
        let rendered_an_icon = self.current_key.is_some() || self.list_rendered;
        self.needs_terminal_clear |= rendered_icons_need_clear(self.adapter, rendered_an_icon);
    }

    pub(super) fn take_terminal_clear(&mut self) -> bool {
        std::mem::take(&mut self.needs_terminal_clear)
    }
}

fn rendered_icons_need_clear(adapter: GraphicsAdapter, rendered_an_icon: bool) -> bool {
    rendered_an_icon && !matches!(adapter, GraphicsAdapter::None)
}

fn should_queue_list_icon(failed: bool, inflight: bool, attempted: bool, cached: bool) -> bool {
    !failed && !inflight && !attempted && !cached
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

fn spawn_worker(
    mut resolver: IconResolver,
    picker: Picker,
    mut request_rx: mpsc::UnboundedReceiver<WorkRequest>,
    result_tx: mpsc::UnboundedSender<IconResult>,
) -> JoinHandle<()> {
    tokio::task::spawn_blocking(move || {
        while let Some(mut request) = request_rx.blocking_recv() {
            loop {
                while let Ok(latest) = request_rx.try_recv() {
                    merge_work(&mut request, latest);
                }
                if let Some(preview) = request.preview.take() {
                    if !prepare_preview(&mut resolver, &picker, &result_tx, preview) {
                        return;
                    }
                    continue;
                }
                let Some(icon) = request.list_icons.pop_front() else {
                    break;
                };
                let prepared = prepare_icon(&mut resolver, picker.clone(), &icon, "desktop-list");
                if result_tx
                    .send(IconResult::List {
                        generation: request.list_generation,
                        icon,
                        prepared,
                    })
                    .is_err()
                {
                    return;
                }
            }
        }
    })
}

fn merge_work(request: &mut WorkRequest, latest: WorkRequest) {
    let WorkRequest {
        preview_generation,
        preview,
        list_generation,
        list_icons,
    } = latest;
    if preview_generation != request.preview_generation {
        request.preview_generation = preview_generation;
        request.preview = preview;
    }
    if list_generation != request.list_generation {
        request.list_generation = list_generation;
        request.list_icons = list_icons;
    } else {
        for icon in list_icons {
            if !request.list_icons.contains(&icon) {
                request.list_icons.push_back(icon);
            }
        }
    }
}

fn prepare_preview(
    resolver: &mut IconResolver,
    picker: &Picker,
    result_tx: &mpsc::UnboundedSender<IconResult>,
    preview: IconRequest,
) -> bool {
    let prepared = prepare_icon(resolver, picker.clone(), &preview.icon, "desktop-preview");
    result_tx
        .send(IconResult::Preview {
            generation: preview.generation,
            prepared,
        })
        .is_ok()
}

fn prepare_icon(
    resolver: &mut IconResolver,
    picker: Picker,
    icon: &str,
    namespace: &str,
) -> Result<Option<PreparedIcon>, String> {
    let Some(path) = resolver.resolve(icon) else {
        return Ok(None);
    };
    prepare_resolved_icon(picker, path, namespace).map(Some)
}

fn prepare_resolved_icon(
    picker: Picker,
    path: PathBuf,
    namespace: &str,
) -> Result<PreparedIcon, String> {
    let key = format!("{namespace}:{}", path.to_string_lossy());
    let (protocol, decoded_bytes) = ImageManager::prepare_image_path_with_weight(picker, &path)
        .map_err(|error| format!("Failed to load desktop icon {}: {error}", path.display()))?;
    Ok(PreparedIcon {
        key,
        protocol: Box::new(protocol),
        decoded_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        IconRequest, WorkRequest, merge_work, rendered_icons_need_clear, should_queue_list_icon,
    };
    use crate::ui::GraphicsAdapter;
    use std::collections::VecDeque;

    fn request(
        preview_generation: u64,
        preview_icon: Option<&str>,
        list_generation: u64,
        list_icons: &[&str],
    ) -> WorkRequest {
        WorkRequest {
            preview_generation,
            preview: preview_icon.map(|icon| IconRequest {
                generation: preview_generation,
                icon: icon.to_string(),
            }),
            list_generation,
            list_icons: list_icons
                .iter()
                .map(|icon| (*icon).to_string())
                .collect::<VecDeque<_>>(),
        }
    }

    #[test]
    fn newer_no_icon_selection_clears_pending_preview() {
        let mut pending = request(1, Some("old"), 1, &[]);

        merge_work(&mut pending, request(2, None, 1, &[]));

        assert_eq!(pending.preview_generation, 2);
        assert!(pending.preview.is_none());
    }

    #[test]
    fn list_only_update_preserves_pending_preview() {
        let mut pending = request(1, Some("selected"), 1, &["old-list"]);

        merge_work(&mut pending, request(1, None, 2, &[]));

        assert_eq!(
            pending.preview.as_ref().map(|item| item.icon.as_str()),
            Some("selected")
        );
        assert_eq!(pending.list_generation, 2);
        assert!(pending.list_icons.is_empty());
    }

    #[test]
    fn newer_list_generation_discards_stale_icons() {
        let mut pending = request(1, None, 1, &["old-a", "old-b"]);

        merge_work(&mut pending, request(1, None, 2, &["new"]));

        assert_eq!(pending.list_icons, VecDeque::from(["new".to_string()]));
    }

    #[test]
    fn rendered_list_icons_require_clearing_even_after_cache_eviction() {
        assert!(rendered_icons_need_clear(GraphicsAdapter::Kitty, true));
        assert!(rendered_icons_need_clear(GraphicsAdapter::Sixel, true));
        assert!(!rendered_icons_need_clear(GraphicsAdapter::None, true));
        assert!(!rendered_icons_need_clear(GraphicsAdapter::Kitty, false));
    }

    #[test]
    fn attempted_list_icon_is_not_requeued_after_cache_eviction() {
        assert!(!should_queue_list_icon(false, false, true, false));
        assert!(should_queue_list_icon(false, false, false, false));
    }
}
