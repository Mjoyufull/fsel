use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use super::xdg;

/// Application filtering and sorting facility
pub struct UI<'a> {
    /// Hidden apps (They don't match the current query)
    pub hidden: Vec<xdg::App>,
    /// Shown apps (They match the current query)
    pub shown: Vec<xdg::App>,
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
    #[doc(hidden)]
    // Matching algorithm
    matcher: SkimMatcherV2,
}

impl<'a> UI<'a> {
    /// Creates a new UI from a `Vec` of [Apps]
    ///
    /// The new items are hidden by default, filter with `self.filter()`
    /// [Apps]: `super::xdg::App`
    pub fn new(items: Vec<xdg::App>) -> UI<'a> {
        UI {
            shown: vec![],
            hidden: items,
            selected: Some(0),
            text: vec![],
            query: String::new(),
            verbose: 0,
            scroll_offset: 0,
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Set verbosity level
    pub fn verbosity(&mut self, b: u64) {
        self.verbose = b;
    }

    /// Update `self.info` to current selection
    ///
    /// Should be called every time `self.selected` changes
    pub fn info(&mut self, color: Color, fancy_mode: bool) {
        if let Some(selected) = self.selected {
            // If there's some selection, update info
            if fancy_mode {
                // In fancy mode, skip the app name (it's in the header) and just show description
                self.text = vec![
                    Line::from(Span::raw(self.shown[selected].description.clone())),
                ];
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
                        "Generic Name: {}", generic_name
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
                        self.text.push(Line::from(Span::raw(format!(
                            "Icon: {}", icon
                        ))));
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

    /// Updates shown and hidden apps with enhanced fuzzy matching
    ///
    /// Matches using [`fuzzy_matcher`] against name, generic name, keywords, and description
    /// with pattern being `self.query`
    ///
    /// Should be called every time user adds/removes characters from `self.query`
    pub fn filter(&mut self) {
        // Hide apps that do *not* match the current filter,
        // and update score for the ones that do
        let mut i = 0;
        while i != self.shown.len() {
            let score = self.calculate_match_score(&self.shown[i]);
            match score {
                // No match. Set score to 0 and move to self.hidden
                None => {
                    self.shown[i].score = 0;
                    self.hidden.push(self.shown.remove(i));
                }
                // Item matched query. Update score
                Some(score) => {
                    self.shown[i].score = score;
                    i += 1;
                }
            }
        }

        // Re-add hidden apps that *do* match the current filter, and update their score
        i = 0;
        while i != self.hidden.len() {
            if let Some(score) = self.calculate_match_score(&self.hidden[i]) {
                self.hidden[i].score = score;
                self.shown.push(self.hidden.remove(i));
            } else {
                i += 1;
            }
        }

        // Sort the vector (should use our custom Cmp)
        self.shown.sort();

        // Reset selection to beginning and scroll offset
        if self.shown.is_empty() {
            // Can't select anything if there's no items
            self.selected = None;
            self.scroll_offset = 0;
        } else {
            // The list changed, ensure we have a valid selection
            // Try to keep current selection if it's still valid, otherwise go to first
            if let Some(current_selected) = self.selected {
                if current_selected >= self.shown.len() {
                    // Current selection is out of bounds, go to first item
                    self.selected = Some(0);
                    self.scroll_offset = 0;
                } else {
                    // Current selection is still valid, keep it but reset scroll
                    self.scroll_offset = 0;
                }
            } else {
                // No selection, go to first item
                self.selected = Some(0);
                self.scroll_offset = 0;
            }
        }
    }
    
    /// Calculate match score against multiple app fields for better fuzzy matching
    fn calculate_match_score(&self, app: &xdg::App) -> Option<i64> {
        if self.query.is_empty() {
            return Some(0);
        }
        
        let mut best_score = None;
        
        // Match against app name (highest priority)
        if let Some(score) = self.matcher.fuzzy_match(&app.name, &self.query) {
            best_score = Some(score * 3); // Boost name matches
        }
        
        // Match against generic name
        if let Some(ref generic_name) = app.generic_name {
            if let Some(score) = self.matcher.fuzzy_match(generic_name, &self.query) {
                let boosted_score = score * 2;
                best_score = Some(best_score.map_or(boosted_score, |current| current.max(boosted_score)));
            }
        }
        
        // Match against keywords
        for keyword in &app.keywords {
            if let Some(score) = self.matcher.fuzzy_match(keyword, &self.query) {
                let boosted_score = score * 2;
                best_score = Some(best_score.map_or(boosted_score, |current| current.max(boosted_score)));
            }
        }
        
        // Match against description (lower priority)
        if let Some(score) = self.matcher.fuzzy_match(&app.description, &self.query) {
            best_score = Some(best_score.map_or(score, |current| current.max(score)));
        }
        
        // Match against categories (lower priority)
        for category in &app.categories {
            if let Some(score) = self.matcher.fuzzy_match(category, &self.query) {
                best_score = Some(best_score.map_or(score, |current| current.max(score)));
            }
        }
        
        best_score
    }
}
