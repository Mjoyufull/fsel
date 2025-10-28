use nucleo_matcher::Utf32Str;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::desktop::App;

/// App filtering and sorting UI
pub struct UI<'a> {
    /// Hidden apps (They don't match the current query)
    pub hidden: Vec<App>,
    /// Shown apps (They match the current query)
    pub shown: Vec<App>,
    /// Current selection (index of `self.shown`)
    pub selected: Option<usize>,
    /// Info text
    pub text: Vec<Line<'a>>,
    /// User query (used for matching)
    pub query: String,
    /// Verbosity level
    pub verbose: u64,
    /// Scroll offset for the list (how many items are scrolled off the top)
    pub scroll_offset: usize,
}

impl<'a> UI<'a> {
    /// Create UI from app list (items start hidden until filtered)
    pub fn new(items: Vec<App>) -> UI<'a> {
        UI {
            shown: vec![],
            hidden: items,
            selected: Some(0),
            text: vec![],
            query: String::new(),
            verbose: 0,
            scroll_offset: 0,
        }
    }

    /// Set verbosity level
    pub fn verbosity(&mut self, b: u64) {
        self.verbose = b;
    }

    /// Update info panel with current selection
    pub fn info(&mut self, color: Color, fancy_mode: bool) {
        if let Some(selected) = self.selected {
            // If there's some selection, update info
            if fancy_mode {
                // In fancy mode, skip the app name (it's in the header) and just show description
                self.text = vec![Line::from(Span::raw(
                    self.shown[selected].description.clone(),
                ))];
            } else {
                // Normal mode: show app name and description
                self.text = vec![
                    Line::from(Span::styled(
                        self.shown[selected].name.clone(),
                        Style::default().fg(color),
                    )),
                    Line::from(Span::raw(self.shown[selected].description.clone())),
                ];
            }
            if self.verbose > 1 {
                self.text.push(Line::default());

                let mut text = if self.shown[selected].is_terminal {
                    vec![Span::raw("Exec (terminal): ")]
                } else {
                    vec![Span::raw("Exec: ")]
                };

                text.push(Span::styled(
                    self.shown[selected].command.to_string(),
                    Style::default(),
                ));

                self.text.push(Line::from(text));

                // Show generic name if available
                if let Some(ref generic_name) = self.shown[selected].generic_name {
                    self.text.push(Line::from(Span::raw(format!(
                        "Generic Name: {}",
                        generic_name
                    ))));
                }

                // Show categories if available
                if !self.shown[selected].categories.is_empty() {
                    self.text.push(Line::from(Span::raw(format!(
                        "Categories: {}",
                        self.shown[selected].categories.join(", ")
                    ))));
                }

                // Show keywords if available
                if !self.shown[selected].keywords.is_empty() {
                    self.text.push(Line::from(Span::raw(format!(
                        "Keywords: {}",
                        self.shown[selected].keywords.join(", ")
                    ))));
                }

                if self.verbose > 2 {
                    // Show MIME types if available
                    if !self.shown[selected].mime_types.is_empty() {
                        self.text.push(Line::from(Span::raw(format!(
                            "MIME Types: {}",
                            self.shown[selected].mime_types.join(", ")
                        ))));
                    }

                    // Show desktop entry type
                    self.text.push(Line::from(Span::raw(format!(
                        "Type: {}",
                        &self.shown[selected].entry_type
                    ))));

                    // Show icon if available
                    if let Some(ref icon) = self.shown[selected].icon {
                        self.text
                            .push(Line::from(Span::raw(format!("Icon: {}", icon))));
                    }

                    self.text.push(Line::from(Span::raw(format!(
                        "Times run: {}",
                        &self.shown[selected].history
                    ))));
                    self.text.push(Line::from(Span::raw(format!(
                        "\nMatching score: {}",
                        self.shown[selected].score
                    ))));
                }
            }
        } else {
            // Else, clear info
            self.text.clear();
        }
    }

    /// Filter apps based on query string using fuzzy or exact matching
    pub fn filter(&mut self, match_mode: crate::cli::MatchMode) {
        // Combine all apps for processing
        let mut all_apps = Vec::with_capacity(self.shown.len() + self.hidden.len());
        all_apps.append(&mut self.shown);
        all_apps.append(&mut self.hidden);

        // Empty query: all apps match with score 0
        if self.query.is_empty() {
            for app in &mut all_apps {
                app.score = 0;
            }
            self.shown = all_apps;
            self.hidden.clear();

            // Sort by history/pinned
            self.shown.sort();

            // Reset selection
            self.selected = if self.shown.is_empty() { None } else { Some(0) };
            self.scroll_offset = 0;
            return;
        }

        // Score all apps
        let query = self.query.clone();
        let scored_apps: Vec<(App, Option<i64>)> = all_apps
            .into_iter()
            .map(|mut app| {
                // Calculate score based on match mode
                let score = match match_mode {
                    crate::cli::MatchMode::Exact => {
                        Self::calculate_exact_match_score_static(&app, &query)
                    }
                    crate::cli::MatchMode::Fuzzy => {
                        Self::calculate_match_score_static(&app, &query)
                    }
                };

                if let Some(s) = score {
                    app.score = s;
                }

                (app, score)
            })
            .collect();

        // Partition into shown (matched) and hidden (not matched)
        self.shown.clear();
        self.hidden.clear();

        for (app, score) in scored_apps {
            if score.is_some() {
                self.shown.push(app);
            } else {
                self.hidden.push(app);
            }
        }

        // Sort shown apps by score
        self.shown.sort();

        // Reset selection to beginning and scroll offset
        if self.shown.is_empty() {
            self.selected = None;
            self.scroll_offset = 0;
        } else {
            if let Some(current_selected) = self.selected {
                if current_selected >= self.shown.len() {
                    self.selected = Some(0);
                    self.scroll_offset = 0;
                } else {
                    self.scroll_offset = 0;
                }
            } else {
                self.selected = Some(0);
                self.scroll_offset = 0;
            }
        }
    }

    /// Static version for parallel processing (thread-safe)
    fn calculate_match_score_static(app: &App, query: &str) -> Option<i64> {
        use nucleo_matcher::{Config, Matcher};
        let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
        Self::calculate_match_score_with_matcher(app, query, &mut matcher)
    }

    /// Shared implementation for match scoring
    fn calculate_match_score_with_matcher(
        app: &App,
        query: &str,
        matcher: &mut nucleo_matcher::Matcher,
    ) -> Option<i64> {
        if query.is_empty() {
            return Some(0);
        }

        let query_lower = query.to_lowercase();
        let mut query_chars = Vec::new();
        let query_utf32 = Utf32Str::new(&query_lower, &mut query_chars);
        let mut best_score: Option<i64> = None;

        // Extract executable name from command
        let exec_name = crate::strings::extract_exec_name(&app.command);
        if !exec_name.is_empty() {
            let exec_name_lower = exec_name.to_lowercase();

            // Check executable name first (highest priority)
            if exec_name_lower == query_lower {
                let mut score = 1_000_000;
                if app.pinned {
                    score += 50_000;
                }
                score += app.history as i64 * 10;
                return Some(score);
            } else if exec_name_lower.starts_with(&query_lower) {
                best_score = Some(best_score.map_or(900_000, |current| current.max(900_000)));
            } else {
                let mut exec_name_chars = Vec::new();
                let exec_name_utf32 = Utf32Str::new(&exec_name_lower, &mut exec_name_chars);
                if let Some(score) = matcher.fuzzy_match(exec_name_utf32, query_utf32) {
                    best_score = Some(best_score.map_or((score as i64) * 4, |current| {
                        current.max((score as i64) * 4)
                    }));
                }
            }
        }

        // Match against app name
        let app_name_lower = app.name.to_lowercase();
        if app_name_lower == query_lower {
            let mut score = 800_000;
            if app.pinned {
                score += 50_000;
            }
            score += app.history as i64 * 10;
            return Some(score);
        } else if app_name_lower.starts_with(&query_lower) {
            let score = 710_000; // Prefix boost over fuzzy matches
            best_score = Some(best_score.map_or(score, |current| current.max(score)));
        } else {
            let mut app_name_chars = Vec::new();
            let app_name_utf32 = Utf32Str::new(&app_name_lower, &mut app_name_chars);
            if let Some(score) = matcher.fuzzy_match(app_name_utf32, query_utf32) {
                let mut score_i64 = score as i64;
                if app_name_lower
                    .split_whitespace()
                    .any(|word| word.starts_with(&query_lower))
                {
                    score_i64 += 5_000;
                }
                let boosted_score = score_i64 * 3;
                best_score =
                    Some(best_score.map_or(boosted_score, |current| current.max(boosted_score)));
            }
        }

        // Match against generic name
        if let Some(ref generic_name) = app.generic_name {
            let generic_lower = generic_name.to_lowercase();
            let mut generic_name_chars = Vec::new();
            let generic_name_utf32 = Utf32Str::new(&generic_lower, &mut generic_name_chars);
            if let Some(score) = matcher.fuzzy_match(generic_name_utf32, query_utf32) {
                let mut score_i64 = score as i64;
                if generic_lower == query_lower {
                    score_i64 = 700_000;
                } else if generic_lower.starts_with(&query_lower) {
                    score_i64 += 8_000;
                }
                let boosted_score = score_i64 * 2;
                best_score =
                    Some(best_score.map_or(boosted_score, |current| current.max(boosted_score)));
            }
        }

        // Match against keywords
        for keyword in &app.keywords {
            let keyword_lower = keyword.to_lowercase();
            let mut keyword_chars = Vec::new();
            let keyword_utf32 = Utf32Str::new(&keyword_lower, &mut keyword_chars);
            if let Some(score) = matcher.fuzzy_match(keyword_utf32, query_utf32) {
                let mut score_i64 = score as i64;
                if keyword_lower == query_lower {
                    score_i64 = 600_000;
                } else if keyword_lower.starts_with(&query_lower) {
                    score_i64 += 6_000;
                }
                let boosted_score = score_i64 * 2;
                best_score =
                    Some(best_score.map_or(boosted_score, |current| current.max(boosted_score)));
            }
        }

        // Match against description (lower priority)
        let mut description_chars = Vec::new();
        let description_lower = app.description.to_lowercase();
        let description_utf32 = Utf32Str::new(&description_lower, &mut description_chars);
        if let Some(score) = matcher.fuzzy_match(description_utf32, query_utf32) {
            let score_i64 = score as i64;
            best_score = Some(best_score.map_or(score_i64, |current| current.max(score_i64)));
        }

        // Match against categories (lower priority)
        for category in &app.categories {
            let mut category_chars = Vec::new();
            let category_lower = category.to_lowercase();
            let category_utf32 = Utf32Str::new(&category_lower, &mut category_chars);
            if let Some(score) = matcher.fuzzy_match(category_utf32, query_utf32) {
                let score_i64 = score as i64;
                best_score = Some(best_score.map_or(score_i64, |current| current.max(score_i64)));
            }
        }

        // Apply pinned and history boosts
        if let Some(mut score) = best_score {
            if app.pinned {
                if score < 600_000 {
                    score += 500_000;
                } else {
                    score += 50_000;
                }
            }

            score = if score >= 600_000 {
                score + (app.history as i64 * 10)
            } else {
                score + (app.history as i64 * 100)
            };

            best_score = Some(score);
        }

        best_score
    }

    /// Static version for parallel processing (thread-safe)
    fn calculate_exact_match_score_static(app: &App, query: &str) -> Option<i64> {
        if query.is_empty() {
            return Some(0);
        }

        let query_lower = query.to_lowercase();

        // Extract executable name from command
        let exec_name = crate::strings::extract_exec_name(&app.command);
        let exec_name_lower = exec_name.to_lowercase();

        // Exact match on executable (highest priority) - early return
        if !exec_name.is_empty() && exec_name_lower == query_lower {
            let mut score = 100000;
            if app.pinned {
                score += 50000;
            }
            return Some(score);
        }

        // Exact match on name - early return
        let app_name_lower = app.name.to_lowercase();
        if app_name_lower == query_lower {
            let mut score = 90000;
            if app.pinned {
                score += 50000;
            }
            return Some(score);
        }

        // Exact match on generic name - early return
        if let Some(ref generic_name) = app.generic_name {
            let generic_lower = generic_name.to_lowercase();
            if generic_lower == query_lower {
                let mut score = 80000;
                if app.pinned {
                    score += 50000;
                }
                return Some(score);
            }
        }

        let mut score = None;

        // Prefix match on executable
        if !exec_name.is_empty() && exec_name_lower.starts_with(&query_lower) {
            score = Some(70000);
        }
        // Prefix match on name
        else if app_name_lower.starts_with(&query_lower) {
            score = Some(60000);
        }
        // Prefix match on generic name
        else if let Some(ref generic_name) = app.generic_name {
            if generic_name.to_lowercase().starts_with(&query_lower) {
                score = Some(50000);
            }
        }

        // Contains query in executable
        if score.is_none() && !exec_name.is_empty() && exec_name_lower.contains(&query_lower) {
            score = Some(4000);
        }
        // Contains query in name
        else if score.is_none() && app_name_lower.contains(&query_lower) {
            score = Some(3000);
        }

        // Check keywords (only if no match yet)
        if score.is_none() && !app.keywords.is_empty() {
            for keyword in &app.keywords {
                let keyword_lower = keyword.to_lowercase();
                if keyword_lower == query_lower {
                    score = Some(2000);
                    break;
                }
                if keyword_lower.contains(&query_lower) {
                    score = Some(1000);
                    break;
                }
            }
        }

        // Check description (only if no match yet)
        if score.is_none() && app.description.to_lowercase().contains(&query_lower) {
            score = Some(500);
        }

        // Apply pin boost
        if let Some(mut s) = score {
            if app.pinned {
                s += 50000;
            }

            score = Some(s);
        }

        score
    }
}
