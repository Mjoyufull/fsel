//! State Management using The Elm Architecture (TEA) pattern
//!
//! This module provides a centralized state and message-passing system for the app launcher.
//! Some types here are infrastructure for future UI refactoring and may appear unused.

use crate::desktop::App;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ScoreBreakdown {
    pub tier: String,
    pub bucket_score: i64,
    pub matcher_score: i64,
    pub frecency_boost: i64,
    pub raw_frecency_milli: i64,
}

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
}

impl State {
    pub fn new(
        apps: Vec<App>,
        _match_mode: crate::cli::MatchMode,
        frecency_data: std::collections::HashMap<String, FrecencyEntry>,
        prefix_depth: usize,
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
        };
        state.filter();
        state
    }

    /// Filter apps based on query
    pub fn filter(&mut self) {
        use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
        use nucleo_matcher::{Config, Matcher, Utf32Str};
        use std::time::Instant;

        if self.query.is_empty() {
            // Show all apps in original order when no query
            self.shown = self.apps.clone();
        } else {
            let filter_start = Instant::now();
            let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
            let pattern = Pattern::parse(&self.query, CaseMatching::Ignore, Normalization::Smart);

            let mut scored: Vec<(i64, App)> = self
                .apps
                .iter()
                .filter_map(|app| {
                    let query_lower = self.query.to_lowercase();
                    let app_name_lower = app.name.to_lowercase();
                    let exec_name = crate::strings::extract_exec_name(&app.command);
                    let exec_name_lower = exec_name.to_lowercase();
                    let generic_name_lower = app.generic_name.as_ref().map(|s| s.to_lowercase());

                    // 1. Calculate fuzzy match score as refined tie-breaker
                    let mut name_buf = Vec::new();
                    let name_haystack = Utf32Str::new(&app.name, &mut name_buf);
                    let name_score = pattern.score(name_haystack, &mut matcher).unwrap_or(0) as i64;

                    let mut exec_buf = Vec::new();
                    let exec_haystack = Utf32Str::new(exec_name, &mut exec_buf);
                    let exec_score = pattern.score(exec_haystack, &mut matcher).unwrap_or(0) as i64;

                    // Match on Keywords and Categories
                    let mut meta_score = 0;
                    let mut check_meta = |haystack: &str| {
                        let mut buf = Vec::new();
                        let hs = Utf32Str::new(haystack, &mut buf);
                        let s = pattern.score(hs, &mut matcher).unwrap_or(0) as i64;
                        if s > meta_score {
                            meta_score = s;
                        }
                    };

                    for keyword in &app.keywords {
                        check_meta(keyword);
                    }
                    for category in &app.categories {
                        check_meta(category);
                    }
                    if let Some(ref generic) = app.generic_name {
                        check_meta(generic);
                    }

                    let base_fuzzy_score =
                        std::cmp::max(std::cmp::max(name_score, exec_score * 2), meta_score);

                    // 2. Determine bucket
                    let name_exact = app_name_lower == query_lower;
                    let exec_exact = exec_name_lower == query_lower;

                    let name_prefix = app_name_lower.starts_with(&query_lower);
                    let exec_prefix = exec_name_lower.starts_with(&query_lower);

                    let check_word_start = |s: &str| {
                        s.starts_with(&query_lower) || s.contains(&format!(" {}", query_lower))
                    };

                    let name_word = check_word_start(&app_name_lower);
                    let exec_word = check_word_start(&exec_name_lower);

                    let meta_match = {
                        generic_name_lower
                            .as_ref()
                            .map(|s| check_word_start(s))
                            .unwrap_or(false)
                            || app
                                .keywords
                                .iter()
                                .any(|k| check_word_start(&k.to_lowercase()))
                            || app
                                .categories
                                .iter()
                                .any(|c| check_word_start(&c.to_lowercase()))
                    };

                    let within_depth = self.query.len() <= self.prefix_depth;

                    // 12+ Distinct Tiers
                    let bucket_score: i64 = if app.pinned {
                        if name_exact {
                            120_000_000
                        } else if exec_exact {
                            115_000_000
                        } else if name_prefix {
                            110_000_000
                        } else if exec_prefix {
                            105_000_000
                        } else if within_depth && name_word {
                            100_000_000
                        } else if within_depth && exec_word {
                            95_000_000
                        } else if within_depth && meta_match {
                            40_000_000
                        } else if base_fuzzy_score > 0 {
                            20_000_000
                        } else {
                            return None;
                        }
                    } else {
                        if name_exact {
                            90_000_000
                        } else if exec_exact {
                            85_000_000
                        } else if name_prefix {
                            80_000_000
                        } else if exec_prefix {
                            75_000_000
                        } else if within_depth && name_word {
                            70_000_000
                        } else if within_depth && exec_word {
                            65_000_000
                        } else if within_depth && meta_match {
                            30_000_000
                        } else if base_fuzzy_score > 0 {
                            0
                        } else {
                            return None;
                        }
                    };

                    // 3. Frecency boost (additive)
                    let frec_score = self
                        .frecency_data
                        .get(&app.name)
                        .map(|e| e.frecency())
                        .unwrap_or(0.0);
                    let frec_boost = (frec_score * 10.0) as i64;
                    let matcher_boost = base_fuzzy_score * 100;
                    let final_score = bucket_score + matcher_boost + frec_boost;

                    let tier_name = match bucket_score {
                        120_000_000 => "Pinned App Name Exact",
                        115_000_000 => "Pinned Exec Name Exact",
                        110_000_000 => "Pinned App Name Prefix",
                        105_000_000 => "Pinned Exec Name Prefix",
                        100_000_000 => "Pinned App Name Word-Start",
                        95_000_000 => "Pinned Exec Name Word-Start",
                        90_000_000 => "Normal App Name Exact",
                        85_000_000 => "Normal Exec Name Exact",
                        80_000_000 => "Normal App Name Prefix",
                        75_000_000 => "Normal Exec Name Prefix",
                        70_000_000 => "Normal App Name Word-Start",
                        65_000_000 => "Normal Exec Name Word-Start",
                        40_000_000 => "Pinned Metadata Match",
                        30_000_000 => "Normal Metadata Match",
                        20_000_000 => "Pinned Fuzzy Match",
                        0 => "Normal Fuzzy Match",
                        _ => "Unknown Tier",
                    }
                    .to_string();

                    let breakdown = ScoreBreakdown {
                        tier: tier_name,
                        bucket_score,
                        matcher_score: matcher_boost,
                        frecency_boost: frec_boost,
                        raw_frecency_milli: (frec_score * 1000.0) as i64,
                    };

                    let mut app_clone = app.clone();
                    app_clone.score = final_score;
                    app_clone.breakdown = Some(breakdown);
                    Some((final_score, app_clone))
                })
                .collect();

            // Sort by score descending, then by name
            scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.name.cmp(&b.1.name)));
            self.shown = scored.into_iter().map(|(_, app)| app).collect();

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
        if let Some(selected) = self.selected {
            if let Some(app) = self.shown.get(selected) {
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

// =============================================================================
// FRECENCY ALGORITHM (zoxide-style)
// =============================================================================

/// Frecency data for a single item
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FrecencyEntry {
    /// Accumulated access count
    pub score: u64,
    /// Last access time (Unix timestamp)
    pub last_access: u64,
}

impl Default for FrecencyEntry {
    fn default() -> Self {
        Self {
            score: 1,
            last_access: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }
}

impl FrecencyEntry {
    /// Record an access (increment score and update timestamp)
    pub fn access(&mut self) {
        self.score += 1;
        self.last_access = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
    }

    /// Calculate frecency score based on time-bucketed multipliers (zoxide algorithm)
    ///
    /// - Within 1 hour: score * 4
    /// - Within 1 day: score * 2
    /// - Within 1 week: score / 2
    /// - Older: score / 4
    pub fn frecency(&self) -> f64 {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let age_secs = now.saturating_sub(self.last_access);
        let score = self.score as f64;

        const HOUR: u64 = 3600;
        const DAY: u64 = 86400;
        const WEEK: u64 = 604800;

        if age_secs < HOUR {
            score * 4.0
        } else if age_secs < DAY {
            score * 2.0
        } else if age_secs < WEEK {
            score * 0.5
        } else {
            score * 0.25
        }
    }

    /// Age the entry by dividing score (for database maintenance)
    pub fn age(&mut self, factor: u64) {
        self.score /= factor;
    }
}

/// Sort apps by frecency (highest first)
pub fn sort_by_frecency(
    apps: &mut [App],
    frecency_data: &std::collections::HashMap<String, FrecencyEntry>,
) {
    // First populate last_access metadata
    for app in apps.iter_mut() {
        if let Some(entry) = frecency_data.get(&app.name) {
            app.last_access = Some(entry.last_access);
        }
    }

    apps.sort_by(|a, b| {
        // pinned apps get priority, obviously
        match (a.pinned, b.pinned) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }

        // Then by frecency (higher first)
        let a_frecency = frecency_data
            .get(&a.name)
            .map(|e| e.frecency())
            .unwrap_or(0.0);
        let b_frecency = frecency_data
            .get(&b.name)
            .map(|e| e.frecency())
            .unwrap_or(0.0);

        b_frecency
            .partial_cmp(&a_frecency)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

/// Age all entries when total score exceeds max (zoxide's aging mechanism)
pub fn age_entries(entries: &mut std::collections::HashMap<String, FrecencyEntry>, max_age: u64) {
    let total: u64 = entries.values().map(|e| e.score).sum();

    if total > max_age {
        // Calculate factor to reduce total to 90% of max
        let target = (max_age as f64 * 0.9) as u64;
        let factor = (total / target).max(2);

        // Age all entries
        entries.values_mut().for_each(|e| e.age(factor));

        // Remove entries with score < 1
        entries.retain(|_, e| e.score >= 1);
    }
}
