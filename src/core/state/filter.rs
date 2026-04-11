use super::State;

impl State {
    /// Filter apps based on the current query.
    pub fn filter(&mut self) {
        use std::time::Instant;

        if self.query.is_empty() {
            self.shown = self.apps.clone();
        } else {
            let filter_start = Instant::now();
            let now_secs = crate::core::ranking::current_unix_seconds();
            self.shown = crate::core::ranking::filter_apps(
                &self.apps,
                crate::core::ranking::FilterOptions {
                    query: &self.query,
                    frecency_data: &self.frecency_data,
                    prefix_depth: self.prefix_depth,
                    ranking_mode: self.ranking_mode,
                    pinned_order_mode: self.pinned_order_mode,
                    pin_timestamps: &self.pin_timestamps,
                    now_secs,
                },
            );

            let filter_time = filter_start.elapsed().as_millis() as u64;
            if crate::cli::DEBUG_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                crate::core::debug_logger::log_search_snapshot(
                    &self.query,
                    &self.shown,
                    self.prefix_depth,
                    filter_time,
                );
            }
        }

        if !self.shown.is_empty() {
            self.selected = Some(0);
            self.scroll_offset = 0;
        } else {
            self.selected = None;
            self.scroll_offset = 0;
        }
    }
}
