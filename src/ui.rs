use nucleo_matcher::{Matcher, Config, Utf32Str};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use super::xdg;
use crate::dmenu::DmenuItem;

/// App filtering and sorting UI
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
}

impl<'a> UI<'a> {
    /// Create UI from app list (items start hidden until filtered)
    pub fn new(items: Vec<xdg::App>) -> UI<'a> {
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
        let scored_apps: Vec<(xdg::App, Option<i64>)> = all_apps
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
        } else {
            self.selected = Some(0);
        }
        self.scroll_offset = 0;
    }
    
    /// Static version for parallel processing (thread-safe)
    fn calculate_match_score_static(app: &xdg::App, query: &str) -> Option<i64> {
        use nucleo_matcher::{Matcher, Config};
        let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
        Self::calculate_match_score_with_matcher(app, query, &mut matcher)
    }
    

    
    /// Shared implementation for match scoring
    fn calculate_match_score_with_matcher(app: &xdg::App, query: &str, matcher: &mut nucleo_matcher::Matcher) -> Option<i64> {
        if query.is_empty() {
            return Some(0);
        }

        let query_lower = query.to_lowercase();
        let mut query_chars = Vec::new();
        let query_utf32 = Utf32Str::new(&query_lower, &mut query_chars);
        let mut best_score: Option<i64> = None;

        // Extract executable name from command
        let exec_name = crate::helpers::extract_exec_name(&app.command);
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
                    best_score = Some(best_score.map_or((score as i64) * 4, |current| current.max((score as i64) * 4)));
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
                best_score = Some(best_score.map_or(boosted_score, |current| current.max(boosted_score)));
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
                best_score = Some(best_score.map_or(boosted_score, |current| current.max(boosted_score)));
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
                best_score = Some(best_score.map_or(boosted_score, |current| current.max(boosted_score)));
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
    fn calculate_exact_match_score_static(app: &xdg::App, query: &str) -> Option<i64> {
        if query.is_empty() {
            return Some(0);
        }
        
        let query_lower = query.to_lowercase();
        
        // Extract executable name from command
        let exec_name = crate::helpers::extract_exec_name(&app.command);
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

/// Dmenu-specific UI for filtering and sorting
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
    /// Match mode (exact or fuzzy)
    pub match_mode: crate::cli::MatchMode,
    /// Match against specific columns
    pub match_nth: Option<Vec<usize>>,
    /// Tag mode state
    /// DISABLED: Waiting for cclip maintainer to add tag support
    #[allow(dead_code)]
    pub tag_mode: TagMode,
    #[doc(hidden)]
    // Matching algorithm (SIMD-accelerated)
    matcher: Matcher,
}

/// Tag mode state for cclip
/// DISABLED: Waiting for cclip maintainer to add tag support
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum TagMode {
    /// Normal mode (not tagging)
    Normal,
    /// Prompting for tag name
    PromptingTagName { input: String },
    /// Prompting for tag color
    PromptingTagColor { tag_name: String, input: String },
    /// Prompting for tag emoji
    PromptingTagEmoji { tag_name: String, color: Option<String>, input: String },
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
            match_mode: crate::cli::MatchMode::Fuzzy,
            match_nth: None,
            tag_mode: TagMode::Normal,
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
        }
    }
    
    /// Set match mode
    pub fn set_match_mode(&mut self, mode: crate::cli::MatchMode) {
        self.match_mode = mode;
    }
    
    /// Set match_nth columns
    pub fn set_match_nth(&mut self, columns: Option<Vec<usize>>) {
        self.match_nth = columns;
    }

    /// Update `self.text` to show content for current selection
    pub fn info(&mut self, _color: Color) {
        self.info_with_image_support(_color, false, false, 0, 0);
    }
    
    /// Update `self.text` to show content with optional image preview support
    pub fn info_with_image_support(&mut self, _color: Color, enable_images: bool, hide_image_message: bool, _panel_width: u16, _panel_height: u16) {
        if let Some(selected) = self.selected {
            if selected < self.shown.len() {
                let item = &self.shown[selected];
                
                // Check if this is a cclip image item and image previews are enabled
                if enable_images && self.is_cclip_image_item(item) {
                    if hide_image_message {
                        // Show minimal or blank content for images
                        self.text = vec![Line::from(Span::raw("".to_string()))];
                    } else {
                        // Placeholder text (image drawn after ratatui)
                        let image_info = self.get_image_info(item);
                        let info_lines = vec![
                            Line::from(Span::raw("  [INLINE IMAGE PREVIEW]".to_string())),
                            Line::from(Span::raw(image_info)),
                            Line::from(Span::raw("".to_string())),
                            Line::from(Span::raw("󱇛 Press 'i' for fullscreen view".to_string())),
                            Line::from(Span::raw(" Press 'Enter' to copy to clipboard".to_string())),
                            Line::from(Span::raw("".to_string())),
                            Line::from(Span::raw("Loading image preview...".to_string())),
                        ];
                        self.text = info_lines;
                    }
                    return;
                }
                
                // For cclip items, get the actual clipboard content
                let content = if self.is_cclip_item(item) {
                    self.get_cclip_content_for_display(item)
                } else {
                    item.get_content_display()
                };
                
                // Simple content handling - just limit length, don't filter aggressively
                let safe_content = if content.len() > 5000 {
                    format!("{}...", &content[..5000])
                } else {
                    content
                };
                
                // Create content display with optional line numbers
                let mut lines = Vec::new();
                
                // Add line number if enabled
                let display_content = if self.show_line_numbers {
                    format!("{}  {}", item.line_number, safe_content)
                } else {
                    safe_content
                };
                
                if self.wrap_long_lines {
                    // Simple line wrapping
                    const MAX_WIDTH: usize = 80;
                    for line in display_content.lines() {
                        if line.chars().count() > MAX_WIDTH {
                            // Hard wrap long lines at character boundaries
                            let chars: Vec<char> = line.chars().collect();
                            let mut start = 0;
                            while start < chars.len() {
                                let end = std::cmp::min(start + MAX_WIDTH, chars.len());
                                let chunk: String = chars[start..end].iter().collect();
                                lines.push(Line::from(Span::raw(chunk)));
                                start = end;
                            }
                        } else {
                            lines.push(Line::from(Span::raw(line.to_string())));
                        }
                    }
                } else {
                    lines.push(Line::from(Span::raw(display_content)));
                }
                
                self.text = lines;
            }
        } else {
            // Clear info if no selection
            self.text.clear();
        }
    }
    
    /// Check if a DmenuItem is a cclip item (has tab-separated format with rowid)
    fn is_cclip_item(&self, item: &crate::dmenu::DmenuItem) -> bool {
        // Parse the tab-separated cclip format
        if item.original_line.trim().is_empty() {
            return false;
        }
        
        let parts: Vec<&str> = item.original_line.splitn(3, '\t').collect();
        if parts.len() >= 2 {
            // Check if first part looks like a cclip rowid (numeric)
            return parts[0].trim().parse::<u64>().is_ok();
        }
        false
    }
    
    /// Check if a DmenuItem is a cclip image item by parsing its original line
    fn is_cclip_image_item(&self, item: &crate::dmenu::DmenuItem) -> bool {
        // Parse the tab-separated cclip format to check mime type
        // Format: rowid\tmime_type\tpreview[\ttag]
        if item.original_line.trim().is_empty() {
            return false;
        }
        
        let parts: Vec<&str> = item.original_line.splitn(4, '\t').collect();
        if parts.len() >= 2 {
            let mime_type = parts[1].trim();
            return !mime_type.is_empty() && mime_type.starts_with("image/");
        }
        false
    }
    
    /// Get actual clipboard content for display (simplified fallback for now)
    fn get_cclip_content_for_display(&self, item: &crate::dmenu::DmenuItem) -> String {
        let parts: Vec<&str> = item.original_line.splitn(4, '\t').collect();
        if parts.len() >= 3 {
            // For now, just show the preview part instead of doing blocking I/O
            let preview = parts[2].trim();
            if !preview.is_empty() {
                preview.to_string()
            } else {
                format!("[Content for rowid {}]", parts[0].trim())
            }
        } else if parts.len() >= 2 {
            // Show mime type info  
            format!("[{} content]", parts[1].trim())
        } else {
            // Fallback
            item.original_line.clone()
        }
    }
    
    /// Get image info for display in the preview panel
    fn get_image_info(&self, item: &crate::dmenu::DmenuItem) -> String {
        let parts: Vec<&str> = item.original_line.splitn(3, '\t').collect();
        if parts.len() >= 3 {
            let mime_type = parts[1].trim();
            let preview = parts[2].trim();
            format!("Type: {}\nInfo: {}", mime_type, preview)
        } else {
            "Image information unavailable".to_string()
        }
    }
    
    /// Get the rowid for a cclip item to retrieve image data
    pub fn get_cclip_rowid(&self, item: &crate::dmenu::DmenuItem) -> Option<String> {
        if !self.is_cclip_image_item(item) {
            return None;
        }
        
        let parts: Vec<&str> = item.original_line.splitn(3, '\t').collect();
        if parts.len() >= 1 {
            Some(parts[0].trim().to_string())
        } else {
            None
        }
    }
    
    /// Get the rowid for ANY cclip item (for tagging, etc)
    /// DISABLED: Waiting for cclip maintainer to add tag support
    #[allow(dead_code)]
    pub fn get_cclip_rowid_any(&self, item: &crate::dmenu::DmenuItem) -> Option<String> {
        let parts: Vec<&str> = item.original_line.splitn(3, '\t').collect();
        if parts.len() >= 1 {
            Some(parts[0].trim().to_string())
        } else {
            None
        }
    }
    
    
    /// Display image directly to terminal, bypassing ratatui
    /// Returns true if image was displayed successfully
    pub fn display_image_to_terminal(&self, item: &crate::dmenu::DmenuItem) -> bool {
        if !self.is_cclip_image_item(item) {
            return false;
        }
        
        // Parse the tab-separated cclip format to get rowid
        let parts: Vec<&str> = item.original_line.splitn(3, '\t').collect();
        if parts.len() >= 1 {
            let rowid = parts[0].trim();
            
            // Detect terminal and choose appropriate format for fullscreen display
            let terminal_type = std::env::var("TERM").unwrap_or_default();
            let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
            
            // Try multiple image formats
            let formats = if term_program == "kitty" || terminal_type.contains("kitty") {
                vec!["kitty", "sixels"]
            } else if terminal_type.starts_with("foot") {
                vec!["sixels"] // Foot specifically (includes foot-extra)
            } else {
                vec!["sixels", "iterm2"] // Default for wezterm, xterm, etc.
            };
            
            // Get terminal size for proper centering
            let (term_width, term_height) = if let Ok((w, h)) = crossterm::terminal::size() {
                (w as usize, h as usize)
            } else {
                (80, 24) // fallback
            };
            
            // Use most of the terminal but leave some padding
            let image_width = (term_width * 90 / 100).max(40); // 90% of width, minimum 40
            let image_height = (term_height * 85 / 100).max(20); // 85% of height, minimum 20
            
            // try multiple formats until one works
            let mut success = false;
            
            for format in formats {
                // clear screen first (use TERM=xterm-256color to avoid foot-extra warning)
                let clear_result = std::process::Command::new("clear")
                    .env("TERM", "xterm-256color")
                    .status();
                
                if clear_result.is_err() {
                    continue;
                }
                
                // pipe cclip output to chafa
                let cclip_child = std::process::Command::new("cclip")
                    .args(&["get", rowid])
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::null())
                    .spawn();
                
                if let Ok(mut cclip) = cclip_child {
                    if let Some(cclip_stdout) = cclip.stdout.take() {
                        let size_arg = format!("{}x{}", image_width, image_height);
                        let chafa_child = std::process::Command::new("chafa")
                            .args(&["--size", &size_arg, "--align", "center", "-f", format, "-"])
                            .stdin(std::process::Stdio::piped())
                            .stdout(std::process::Stdio::inherit())
                            .stderr(std::process::Stdio::null())
                            .spawn();
                        
                        if let Ok(mut chafa) = chafa_child {
                            if let Some(mut chafa_stdin) = chafa.stdin.take() {
                                std::thread::spawn(move || {
                                    let mut cclip_stdout = cclip_stdout;
                                    std::io::copy(&mut cclip_stdout, &mut chafa_stdin).ok();
                                });
                                
                                let _ = cclip.wait();
                                if let Ok(status) = chafa.wait() {
                                    if status.success() {
                                        success = true;
                                        break; // found a working format
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            success
        } else {
            false
        }
    }
    

    /// Updates shown and hidden items with matching (fuzzy or exact)
    pub fn filter(&mut self) {
        // Optimized filtering for large datasets
        // Key optimizations:
        // 1. Use swap_remove instead of remove (O(1) vs O(n))
        // 2. Minimize allocations
        // 3. Process in-place where possible
        
        // Early return for empty query - everything matches
        let query_is_empty = self.query.is_empty();
        
        // Filter shown items - remove non-matching
        let mut i = 0;
        while i < self.shown.len() {
            // Calculate score for this item
            let score = if query_is_empty {
                Some(0)
            } else if let Some(ref match_cols) = self.match_nth {
                self.shown[i].calculate_score_with_match_nth(&self.query, &mut self.matcher, match_cols)
            } else {
                match self.match_mode {
                    crate::cli::MatchMode::Exact => self.shown[i].calculate_exact_score(&self.query),
                    crate::cli::MatchMode::Fuzzy => self.shown[i].calculate_score(&self.query, &mut self.matcher),
                }
            };
            
            match score {
                None => {
                    // Doesn't match - move to hidden using swap_remove (O(1))
                    self.shown[i].set_score(0);
                    let item = self.shown.swap_remove(i);
                    self.hidden.push(item);
                    // Don't increment i since we swapped the last element here
                }
                Some(score) => {
                    self.shown[i].set_score(score);
                    i += 1;
                }
            }
        }

        // Re-add hidden items that now match
        let mut i = 0;
        while i < self.hidden.len() {
            // Calculate score for this item
            let score = if query_is_empty {
                Some(0)
            } else if let Some(ref match_cols) = self.match_nth {
                self.hidden[i].calculate_score_with_match_nth(&self.query, &mut self.matcher, match_cols)
            } else {
                match self.match_mode {
                    crate::cli::MatchMode::Exact => self.hidden[i].calculate_exact_score(&self.query),
                    crate::cli::MatchMode::Fuzzy => self.hidden[i].calculate_score(&self.query, &mut self.matcher),
                }
            };
            
            if let Some(score) = score {
                self.hidden[i].set_score(score);
                let item = self.hidden.swap_remove(i);
                self.shown.push(item);
                // Don't increment i since we swapped
            } else {
                i += 1;
            }
        }

        // Sort by score (descending - higher scores first)
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
