//! State management for the launcher.
//!
//! This module owns launcher state and message handling. Ranking policy lives in
//! `crate::core::ranking`.

mod filter;
mod info;
mod update;

use crate::core::hidden_entries::{EntryKey, HiddenSummary, VisibilityOptions};
use crate::core::ranking::FrecencyEntry;
use crate::desktop::App;
use std::collections::{HashMap, HashSet};

pub use update::update;

#[derive(Debug)]
pub struct State {
    /// All loaded applications.
    pub apps: Vec<App>,
    /// Filtered/shown applications.
    pub shown: Vec<App>,
    /// Current search query.
    pub query: String,
    /// Currently selected index.
    pub selected: Option<usize>,
    /// Scroll offset for virtualized list.
    pub scroll_offset: usize,
    /// Info text to display.
    pub text: String,
    /// Whether the app should exit.
    pub should_exit: bool,
    /// Whether to execute the selected app.
    pub should_launch: bool,
    /// Frecency data for boosting.
    pub frecency_data: HashMap<String, FrecencyEntry>,
    /// Character depth for prioritized prefix matching.
    pub prefix_depth: usize,
    /// Ranking mode used for ordering and score boosts.
    pub ranking_mode: crate::cli::RankingMode,
    /// Strategy for ordering pinned apps.
    pub pinned_order_mode: crate::cli::PinnedOrderMode,
    /// First pinned timestamp by app name.
    pub pin_timestamps: HashMap<String, u64>,
    /// Match mode used for app filtering.
    pub match_mode: crate::cli::MatchMode,
    hidden_entry_keys: HashSet<EntryKey>,
    visibility_options: VisibilityOptions,
    hidden_summary: HiddenSummary,
    status_message: Option<String>,
}

impl State {
    pub fn new(
        apps: Vec<App>,
        match_mode: crate::cli::MatchMode,
        frecency_data: HashMap<String, FrecencyEntry>,
        prefix_depth: usize,
        ranking_mode: crate::cli::RankingMode,
        pinned_order_mode: crate::cli::PinnedOrderMode,
        pin_timestamps: HashMap<String, u64>,
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
            match_mode,
            hidden_entry_keys: HashSet::new(),
            visibility_options: VisibilityOptions::default(),
            hidden_summary: HiddenSummary::default(),
            status_message: None,
        };
        state.filter();
        state
    }

    pub(crate) fn set_hidden_entry_keys(&mut self, hidden_entry_keys: HashSet<EntryKey>) {
        self.hidden_entry_keys = hidden_entry_keys;
        self.filter();
    }

    pub(crate) fn set_visibility_options(&mut self, visibility_options: VisibilityOptions) {
        self.visibility_options = visibility_options;
        self.filter();
    }

    pub(crate) fn hide_entry(&mut self, entry_key: EntryKey) {
        let selected = self.selected.unwrap_or(0);
        let scroll_offset = self.scroll_offset;
        self.hidden_entry_keys.insert(entry_key);
        self.filter();
        if !self.shown.is_empty() {
            let nearest = selected.min(self.shown.len() - 1);
            self.selected = Some(nearest);
            self.scroll_offset = scroll_offset.min(nearest);
        }
    }

    pub(crate) fn unhide_entry(&mut self, entry_key: &EntryKey) {
        let selected_key = self
            .selected
            .and_then(|index| self.shown.get(index))
            .and_then(App::entry_key);
        let selected_index = self.selected.unwrap_or(0);
        let scroll_offset = self.scroll_offset;
        self.hidden_entry_keys.remove(entry_key);
        self.filter();
        if !self.shown.is_empty() {
            let restored_selection = selected_key
                .and_then(|key| {
                    self.shown
                        .iter()
                        .position(|app| app.entry_key().as_ref() == Some(&key))
                })
                .unwrap_or_else(|| selected_index.min(self.shown.len() - 1));
            self.selected = Some(restored_selection);
            self.scroll_offset = scroll_offset.min(restored_selection);
        }
    }

    pub(crate) fn hidden_summary(&self) -> &HiddenSummary {
        &self.hidden_summary
    }

    pub(crate) fn set_status_message(&mut self, message: impl Into<String>) {
        self.status_message = Some(message.into());
    }

    pub(crate) fn clear_status_message(&mut self) {
        self.status_message = None;
    }
}

/// Messages that can be sent to update state.
#[derive(Debug, Clone)]
pub enum Message {
    /// Character typed.
    CharInput(char),
    /// Backspace pressed.
    Backspace,
    /// Move selection up.
    MoveUp,
    /// Move selection down.
    MoveDown,
    /// Move to first item.
    MoveFirst,
    /// Move to last item.
    MoveLast,
    /// Select/launch current item.
    Select,
    /// Exit without selection.
    Exit,
    /// Select specific index (e.g. from mouse).
    SelectIndex(usize),
    /// Tick event (for animations, etc.).
    Tick,
}
