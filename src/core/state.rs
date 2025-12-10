//! State Management using The Elm Architecture (TEA) pattern
//!
//! This module provides a centralized state and message-passing system for the app launcher.
//! Some types here are infrastructure for future UI refactoring and may appear unused.

#![allow(dead_code)]

use crate::cli::MatchMode;
use crate::desktop::App;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::style::Color;
use std::time::SystemTime;

/// Central application state
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
    /// Match mode for filtering
    pub match_mode: MatchMode,
}

impl State {
    pub fn new(apps: Vec<App>, match_mode: MatchMode) -> Self {
        let mut state = Self {
            apps,
            shown: Vec::new(),
            query: String::new(),
            selected: None,
            scroll_offset: 0,
            text: String::new(),
            should_exit: false,
            should_launch: false,
            match_mode,
        };
        state.filter();
        state
    }

    /// Filter apps based on query
    pub fn filter(&mut self) {
        use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
        use nucleo_matcher::{Config, Matcher};

        if self.query.is_empty() {
            // Show all apps in original order when no query
            self.shown = self.apps.clone();
        } else {
            let mut matcher = Matcher::new(Config::DEFAULT);
            let pattern = Pattern::parse(&self.query, CaseMatching::Ignore, Normalization::Smart);

            let mut scored: Vec<(i64, App)> = self
                .apps
                .iter()
                .filter_map(|app| {
                    let mut buf = Vec::new();
                    let haystack = nucleo_matcher::Utf32Str::new(&app.name, &mut buf);
                    pattern.score(haystack, &mut matcher).map(|score| (score as i64, app.clone()))
                })
                .collect();

            // Sort by score descending, then by name
            scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.name.cmp(&b.1.name)));
            self.shown = scored.into_iter().map(|(_, app)| app).collect();
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
                        self.text.push_str(&format!("Exec (terminal): {}\n", app.command));
                    } else {
                        self.text.push_str(&format!("Exec: {}\n", app.command));
                    }

                    if let Some(ref generic) = app.generic_name {
                        self.text.push_str(&format!("Generic Name: {}\n", generic));
                    }

                    if !app.categories.is_empty() {
                         self.text.push_str(&format!("Categories: {}\n", app.categories.join(", ")));
                    }
                    
                    if !app.keywords.is_empty() {
                         self.text.push_str(&format!("Keywords: {}\n", app.keywords.join(", ")));
                    }

                    if verbose > 2 {
                         if !app.mime_types.is_empty() {
                             self.text.push_str(&format!("MIME Types: {}\n", app.mime_types.join(", ")));
                         }
                         self.text.push_str(&format!("Type: {}\n", app.entry_type));
                         if let Some(ref icon) = app.icon {
                             self.text.push_str(&format!("Icon: {}\n", icon));
                         }
                         // Recency first (before Times run) - matches screenshot order
                         if let Some(ts) = app.last_access {
                             self.text.push_str(&format!("Last Run: {}\n", format_recency(ts)));
                         }
                         self.text.push_str(&format!("Times run: {}\n", app.history));
                         self.text.push_str(&format!("Matching score: {}\n", app.score));
                    }
                }
            }
        }
    }
}

/// Messages that can be sent to update state
#[derive(Debug, Clone)]
pub enum Message {
    /// Key input received
    KeyInput { code: KeyCode, modifiers: KeyModifiers },
    /// Mouse event
    Mouse { row: u16, column: u16, kind: MouseEventKind },
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
    /// Toggle pin on current item
    TogglePin,
    /// Exit without selection
    Exit,
    /// Select specific index (e.g. from mouse)
    SelectIndex(usize),
    /// Tick event (for animations, etc.)
    Tick,
    /// Render event
    Render,
    /// Resize event
    Resize { width: u16, height: u16 },
}

#[derive(Debug, Clone)]
pub enum MouseEventKind {
    Click,
    Scroll { delta: i8 },
    Move,
}

/// Update function - pure state transition
pub fn update(state: &mut State, msg: Message, hard_stop: bool, max_visible: usize) {
    match msg {
        Message::SelectIndex(idx) => {
             if idx < state.shown.len() {
                 state.selected = Some(idx);
             }
        }
        Message::CharInput(c) => {
            state.query.push(c);
            state.filter();
        }
        Message::Backspace => {
            state.query.pop();
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
        Message::TogglePin => {
            // Handled externally due to database access
        }
        Message::Tick | Message::Render | Message::Resize { .. } | Message::Mouse { .. } | Message::KeyInput { .. } => {
            // No-op for these in pure state update
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
    /// Create a new entry with initial score
    pub fn new() -> Self {
        Self::default()
    }

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
pub fn sort_by_frecency(apps: &mut [App], frecency_data: &std::collections::HashMap<String, FrecencyEntry>) {
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
        let a_frecency = frecency_data.get(&a.name).map(|e| e.frecency()).unwrap_or(0.0);
        let b_frecency = frecency_data.get(&b.name).map(|e| e.frecency()).unwrap_or(0.0);
        
        b_frecency.partial_cmp(&a_frecency)
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
