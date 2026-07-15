use super::State;

impl State {
    /// Filter apps based on the current query.
    pub fn filter(&mut self) {
        use std::time::Instant;

        let (eligible_apps, hidden_summary) = crate::core::hidden_entries::eligible_apps(
            &self.apps,
            &self.hidden_entry_keys,
            &self.visibility_options,
        );
        self.hidden_summary = hidden_summary;

        if self.query.is_empty() {
            self.shown = eligible_apps;
        } else {
            let filter_start = Instant::now();
            let now_secs = crate::core::ranking::current_unix_seconds();
            self.shown = crate::core::ranking::filter_apps(
                &eligible_apps,
                crate::core::ranking::FilterOptions {
                    query: &self.query,
                    match_mode: self.match_mode,
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

#[cfg(test)]
mod tests {
    use super::State;
    use crate::cli::{MatchMode, PinnedOrderMode, RankingMode};
    use crate::desktop::App;
    use std::collections::{HashMap, HashSet};
    use std::path::Path;

    #[test]
    fn manual_hidden_keys_filter_and_restore_exact_sources() {
        let mut app = App::parse(
            "[Desktop Entry]\nType=Application\nName=Editor\nExec=/usr/bin/editor",
            false,
        )
        .expect("desktop entry should parse");
        app.desktop_id = Some("editor.desktop".to_string());
        app.set_source_path(Path::new("/usr/share/applications/editor.desktop"));
        let entry_key = app
            .entry_key()
            .expect("desktop app should have an entry key");
        let mut state = State::new(
            vec![app],
            MatchMode::Fuzzy,
            HashMap::new(),
            3,
            RankingMode::Frecency,
            PinnedOrderMode::Ranking,
            HashMap::new(),
        );

        state.set_hidden_entry_keys(HashSet::from([entry_key.clone()]));
        assert!(state.shown.is_empty());

        state.unhide_entry(&entry_key);
        assert_eq!(state.shown.len(), 1);
    }

    #[test]
    fn hide_keeps_the_nearest_selection() {
        let mut apps = ["One", "Two", "Three"]
            .into_iter()
            .map(|name| {
                let mut app = App::parse(
                    format!("[Desktop Entry]\nType=Application\nName={name}\nExec=/bin/true"),
                    false,
                )
                .expect("fixture should parse");
                app.desktop_id = Some(format!("{}.desktop", name.to_lowercase()));
                app.set_source_path(Path::new(&format!("/{name}.desktop")));
                app
            })
            .collect::<Vec<_>>();
        let hidden_key = apps[1].entry_key().expect("app should have an entry key");
        let mut state = State::new(
            std::mem::take(&mut apps),
            MatchMode::Fuzzy,
            HashMap::new(),
            3,
            RankingMode::Frecency,
            PinnedOrderMode::Ranking,
            HashMap::new(),
        );
        state.selected = Some(1);

        state.hide_entry(hidden_key);

        assert_eq!(state.selected, Some(1));
        assert_eq!(state.shown[1].name, "Three");
    }
}
