//! State Management using The Elm Architecture (TEA) pattern
//!
//! This module owns launcher state and message handling. Ranking policy lives in
//! `crate::core::ranking`.

use crate::core::ranking::FrecencyEntry;
use crate::desktop::App;
use ratatui::style::Color;
use std::time::SystemTime;

#[derive(Debug)]
pub struct State {
    /// All loaded applications
    pub apps: Vec<App>,
    /// Filtered/shown applications
    pub shown: Vec<App>,
    /// Current search query
    pub query: String,
    /// Currently selected index
    pub selected: Option<usize>,
    /// Scroll offset for virtualized list
    pub scroll_offset: usize,
    /// Info text to display
    pub text: String,
    /// Whether the app should exit
    pub should_exit: bool,
    /// Whether to execute the selected app
    pub should_launch: bool,
    /// Frecency data for boosting
    pub frecency_data: std::collections::HashMap<String, FrecencyEntry>,
    /// Character depth for prioritized prefix matching
    pub prefix_depth: usize,
    /// Ranking mode used for ordering and score boosts
    pub ranking_mode: crate::cli::RankingMode,
    /// Strategy for ordering pinned apps
    pub pinned_order_mode: crate::cli::PinnedOrderMode,
    /// First pinned timestamp by app name
    pub pin_timestamps: std::collections::HashMap<String, u64>,
}

impl State {
    pub fn new(
        apps: Vec<App>,
        _match_mode: crate::cli::MatchMode,
        frecency_data: std::collections::HashMap<String, FrecencyEntry>,
        prefix_depth: usize,
        ranking_mode: crate::cli::RankingMode,
        pinned_order_mode: crate::cli::PinnedOrderMode,
        pin_timestamps: std::collections::HashMap<String, u64>,
    ) -> Self {
        let mut state = Self {
            apps,
            shown: Vec::new(),
            query: String::new(),
            selected: None,
            scroll_offset: 0,
            text: String::new(),
            should_exit: false,
            should_launch: false,
            frecency_data,
            prefix_depth,
            ranking_mode,
            pinned_order_mode,
            pin_timestamps,
        };
        state.filter();
        state
    }

    /// Filter apps based on query
    pub fn filter(&mut self) {
        use std::time::Instant;

        if self.query.is_empty() {
            // Show all apps in original order when no query
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

        // Reset selection
        if !self.shown.is_empty() {
            self.selected = Some(0);
            self.scroll_offset = 0;
        } else {
            self.selected = None;
        }
    }

    /// Update info text based on selected app
    pub fn update_info(&mut self, _highlight_color: Color, fancy_mode: bool, verbose: u64) {
        if let Some(selected) = self.selected
            && let Some(app) = self.shown.get(selected)
        {
            // the basics
            self.text = if fancy_mode {
                // In fancy mode, skip the app name (it's in the header) and just show description
                app.description.clone()
            } else {
                // Normal mode: show app name and description
                format!("{}\n\n{}", app.name, app.description)
            };

            // Helper to format recency
            let format_recency = |ts: u64| -> String {
                let now = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let diff = now.saturating_sub(ts);

                if diff < 60 {
                    format!("{}s ago", diff)
                } else if diff < 3600 {
                    format!("{}m ago", diff / 60)
                } else if diff < 86400 {
                    format!("{}h ago", diff / 3600)
                } else {
                    format!("{}d ago", diff / 86400)
                }
            };

            // extra deets for the nerds
            if verbose > 1 {
                self.text.push_str("\n\n");

                if app.is_terminal {
                    self.text
                        .push_str(&format!("Exec (terminal): {}\n", app.command));
                } else {
                    self.text.push_str(&format!("Exec: {}\n", app.command));
                }

                if let Some(ref generic) = app.generic_name {
                    self.text.push_str(&format!("Generic Name: {}\n", generic));
                }

                if !app.categories.is_empty() {
                    self.text
                        .push_str(&format!("Categories: {}\n", app.categories.join(", ")));
                }

                if !app.keywords.is_empty() {
                    self.text
                        .push_str(&format!("Keywords: {}\n", app.keywords.join(", ")));
                }

                if verbose > 2 {
                    if !app.mime_types.is_empty() {
                        self.text
                            .push_str(&format!("MIME Types: {}\n", app.mime_types.join(", ")));
                    }
                    self.text.push_str(&format!("Type: {}\n", app.entry_type));
                    if let Some(ref icon) = app.icon {
                        self.text.push_str(&format!("Icon: {}\n", icon));
                    }
                    // Recency first (before Times run) - matches screenshot order
                    if let Some(ts) = app.last_access {
                        self.text
                            .push_str(&format!("Last Run: {}\n", format_recency(ts)));
                    }
                    self.text.push_str(&format!("Times run: {}\n", app.history));
                    self.text
                        .push_str(&format!("Matching score: {}\n", app.score));
                }
            }
        }
    }
}

/// Messages that can be sent to update state
#[derive(Debug, Clone)]
pub enum Message {
    /// Character typed
    CharInput(char),
    /// Backspace pressed
    Backspace,
    /// Move selection up
    MoveUp,
    /// Move selection down
    MoveDown,
    /// Move to first item
    MoveFirst,
    /// Move to last item
    MoveLast,
    /// Select/launch current item
    Select,
    /// Exit without selection
    Exit,
    /// Select specific index (e.g. from mouse)
    SelectIndex(usize),
    /// Tick event (for animations, etc.)
    Tick,
}

/// Update function - pure state transition
pub fn update(state: &mut State, msg: Message, hard_stop: bool, max_visible: usize) {
    match msg {
        Message::SelectIndex(idx) => {
            if idx < state.shown.len() {
                let app_name = state.shown.get(idx).map(|a| a.name.as_str());
                state.selected = Some(idx);
                if crate::cli::DEBUG_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                    crate::core::debug_logger::log_selection_change(
                        state.selected,
                        app_name,
                        state.scroll_offset,
                    );
                }
            }
        }
        Message::CharInput(c) => {
            let old_query = state.query.clone();
            state.query.push(c);
            if crate::cli::DEBUG_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                crate::core::debug_logger::log_query_change(
                    &old_query,
                    &state.query,
                    &format!("User typed '{}'", c),
                );
            }
            state.filter();
        }
        Message::Backspace => {
            let old_query = state.query.clone();
            state.query.pop();
            if crate::cli::DEBUG_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                crate::core::debug_logger::log_query_change(
                    &old_query,
                    &state.query,
                    "User pressed backspace",
                );
            }
            state.filter();
        }
        Message::MoveUp => {
            if let Some(selected) = state.selected {
                state.selected = if selected > 0 {
                    Some(selected - 1)
                } else if !hard_stop && !state.shown.is_empty() {
                    Some(state.shown.len() - 1)
                } else {
                    Some(selected)
                };
                // Auto-scroll
                if let Some(new_sel) = state.selected {
                    if new_sel < state.scroll_offset {
                        state.scroll_offset = new_sel;
                    } else if new_sel >= state.scroll_offset + max_visible {
                        state.scroll_offset = new_sel.saturating_sub(max_visible - 1);
                    }
                }
            }
        }
        Message::MoveDown => {
            if let Some(selected) = state.selected {
                state.selected = if selected < state.shown.len().saturating_sub(1) {
                    Some(selected + 1)
                } else if !hard_stop {
                    Some(0)
                } else {
                    Some(selected)
                };
                // Auto-scroll
                if let Some(new_sel) = state.selected {
                    if new_sel >= state.scroll_offset + max_visible {
                        state.scroll_offset = new_sel.saturating_sub(max_visible - 1);
                    } else if new_sel < state.scroll_offset {
                        state.scroll_offset = new_sel;
                    }
                }
            }
        }
        Message::MoveFirst => {
            if !state.shown.is_empty() {
                state.selected = Some(0);
                state.scroll_offset = 0;
            }
        }
        Message::MoveLast => {
            if !state.shown.is_empty() {
                state.selected = Some(state.shown.len() - 1);
                let last = state.shown.len() - 1;
                if last >= max_visible {
                    state.scroll_offset = last - max_visible + 1;
                }
            }
        }
        Message::Select => {
            state.should_launch = true;
        }
        Message::Exit => {
            state.selected = None;
            state.should_exit = true;
        }
        Message::Tick => {
            // No-op for tick events
        }
    }
}
