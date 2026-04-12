use super::ImageRuntime;
use crate::ui::{DISPLAY_STATE, DisplayState};
use futures::FutureExt;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;

impl ImageRuntime {
    pub(super) async fn ensure_image_loaded(&mut self) {
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
