//! State management for the launcher.
//!
//! This module owns launcher state and message handling. Ranking policy lives in
//! `crate::core::ranking`.

mod filter;
mod info;
mod update;

use crate::core::ranking::FrecencyEntry;
use crate::desktop::App;

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
    pub frecency_data: std::collections::HashMap<String, FrecencyEntry>,
    /// Character depth for prioritized prefix matching.
    pub prefix_depth: usize,
    /// Ranking mode used for ordering and score boosts.
    pub ranking_mode: crate::cli::RankingMode,
    /// Strategy for ordering pinned apps.
    pub pinned_order_mode: crate::cli::PinnedOrderMode,
    /// First pinned timestamp by app name.
    pub pin_timestamps: std::collections::HashMap<String, u64>,
    /// Match mode used for app filtering.
    pub match_mode: crate::cli::MatchMode,
}

impl State {
    pub fn new(
        apps: Vec<App>,
        match_mode: crate::cli::MatchMode,
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
            match_mode,
        };
        state.filter();
        state
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
