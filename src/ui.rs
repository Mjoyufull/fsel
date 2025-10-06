use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use super::xdg;
use crate::dmenu::DmenuItem;

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

/// Dmenu-specific UI filtering and sorting facility
pub struct DmenuUI<'a> {
    /// Hidden items (They don't match the current query)
    pub hidden: Vec<DmenuItem>,
    /// Shown items (They match the current query)
    pub shown: Vec<DmenuItem>,
    /// Current selection (index of `self.shown`)
    pub selected: Option<usize>,
    /// Info text for content display
    pub text: Vec<Line<'a>>,
    /// User query (used for matching)
    pub query: String,
    /// Scroll offset for the list
    pub scroll_offset: usize,
    /// Whether to wrap long lines in content display
    pub wrap_long_lines: bool,
    /// Show line numbers
    pub show_line_numbers: bool,
    #[doc(hidden)]
    // Matching algorithm
    matcher: SkimMatcherV2,
}

impl<'a> DmenuUI<'a> {
    /// Creates a new DmenuUI from a Vec of DmenuItems
    pub fn new(items: Vec<DmenuItem>, wrap_long_lines: bool, show_line_numbers: bool) -> DmenuUI<'a> {
        DmenuUI {
            shown: vec![],
            hidden: items,
            selected: Some(0),
            text: vec![],
            query: String::new(),
            scroll_offset: 0,
            wrap_long_lines,
            show_line_numbers,
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Update `self.text` to show content for current selection
    pub fn info(&mut self, _color: Color) {
        self.info_with_image_support(_color, false, 0, 0);
    }
    
    /// Update `self.text` to show content with optional image preview support
    pub fn info_with_image_support(&mut self, _color: Color, enable_images: bool, panel_width: u16, panel_height: u16) {
        if let Some(selected) = self.selected {
            if selected < self.shown.len() {
                let item = &self.shown[selected];
                
                // Check if this is a cclip image item and image previews are enabled
                if enable_images && self.is_cclip_image_item(item) {
                    // Try to generate image preview
                    if let Ok(image_preview) = self.generate_cclip_image_preview(item, panel_width, panel_height) {
                        // Split image preview into lines
                        let preview_lines: Vec<Line> = image_preview
                            .lines()
                            .map(|line| Line::from(Span::raw(line.to_string())))
                            .collect();
                        self.text = preview_lines;
                        return;
                    }
                }
                
                // Fallback to regular content display
                let content = item.get_content_display();
                
                // Create content display with optional line numbers
                let mut lines = Vec::new();
                
                if self.show_line_numbers {
                    lines.push(Line::from(Span::raw(format!("Line {}: ", item.line_number))));
                }
                
                if self.wrap_long_lines {
                    // Split long content into multiple lines for better display
                    const MAX_WIDTH: usize = 80; // Reasonable default
                    if content.len() > MAX_WIDTH {
                        // Create owned strings to avoid lifetime issues
                        let mut start = 0;
                        while start < content.len() {
                            let end = std::cmp::min(start + MAX_WIDTH, content.len());
                            let chunk = content[start..end].to_string();
                            lines.push(Line::from(Span::raw(chunk)));
                            start = end;
                        }
                    } else {
                        lines.push(Line::from(Span::raw(content)));
                    }
                } else {
                    lines.push(Line::from(Span::raw(content)));
                }
                
                self.text = lines;
            }
        } else {
            // Clear info if no selection
            self.text.clear();
        }
    }
    
    /// Check if a DmenuItem is a cclip image item by parsing its original line
    fn is_cclip_image_item(&self, item: &crate::dmenu::DmenuItem) -> bool {
        // Parse the tab-separated cclip format to check mime type
        let parts: Vec<&str> = item.original_line.splitn(3, '\t').collect();
        if parts.len() >= 2 {
            let mime_type = parts[1];
            return mime_type.starts_with("image/");
        }
        false
    }
    
    /// Generate image preview for a cclip item
    fn generate_cclip_image_preview(&self, item: &crate::dmenu::DmenuItem, width: u16, height: u16) -> Result<String, String> {
        // Parse the tab-separated cclip format to get rowid
        let parts: Vec<&str> = item.original_line.splitn(3, '\t').collect();
        if parts.len() >= 1 {
            let rowid = parts[0];
            
            // Get image content from cclip
            match std::process::Command::new("cclip")
                .args(&["get", rowid])
                .output()
            {
                Ok(output) if output.status.success() => {
                    // Use cclip module's image preview function
                    match crate::cclip::generate_image_preview(&output.stdout, width, height) {
                        Ok(preview) => Ok(preview),
                        Err(_) => Err("Failed to generate image preview".to_string())
                    }
                }
                _ => Err("Failed to get image content from cclip".to_string())
            }
        } else {
            Err("Invalid cclip format".to_string())
        }
    }

    /// Updates shown and hidden items with fuzzy matching
    pub fn filter(&mut self) {
        // Hide items that don't match the current filter
        let mut i = 0;
        while i != self.shown.len() {
            let score = self.shown[i].calculate_score(&self.query, &self.matcher);
            match score {
                None => {
                    self.shown[i].set_score(0);
                    self.hidden.push(self.shown.remove(i));
                }
                Some(score) => {
                    self.shown[i].set_score(score);
                    i += 1;
                }
            }
        }

        // Re-add hidden items that now match
        i = 0;
        while i != self.hidden.len() {
            if let Some(score) = self.hidden[i].calculate_score(&self.query, &self.matcher) {
                self.hidden[i].set_score(score);
                self.shown.push(self.hidden.remove(i));
            } else {
                i += 1;
            }
        }

        // Sort by score
        self.shown.sort();

        // Reset selection and scroll
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
}
