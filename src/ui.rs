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

                if self.verbose > 2 {
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

    /// Updates shown and hidden apps
    ///
    /// Matches using [`fuzzy_matcher`], with pattern being `self.query`
    ///
    /// Should be called every time user adds/removes characters from `self.query`
    pub fn filter(&mut self) {
        // Hide apps that do *not* match the current filter,
        // and update score for the ones that do
        let mut i = 0;
        while i != self.shown.len() {
            match self.matcher.fuzzy_match(&self.shown[i].name, &self.query) {
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
            if let Some(score) = self.matcher.fuzzy_match(&self.hidden[i].name, &self.query) {
                self.hidden[i].score = score;
                self.shown.push(self.hidden.remove(i));
            } else {
                i += 1;
            }
        }

        // Sort the vector (should use our custom Cmp)
        self.shown.sort();

        // Reset selection to beginning (don't want to have the user go to the start
        if self.shown.is_empty() {
            // Can't select anything if there's no items
            self.selected = None;
        } else {
            // The list changed, go to first item
            self.selected = Some(0);
        }
    }
}
