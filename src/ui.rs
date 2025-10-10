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
            matcher: SkimMatcherV2::default(),
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
        // Hide apps that do *not* match the current filter,
        // and update score for the ones that do
        let mut i = 0;
        while i != self.shown.len() {
            let score = match match_mode {
                crate::cli::MatchMode::Exact => self.calculate_exact_match_score(&self.shown[i]),
                crate::cli::MatchMode::Fuzzy => self.calculate_match_score(&self.shown[i]),
            };
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
            let score = match match_mode {
                crate::cli::MatchMode::Exact => self.calculate_exact_match_score(&self.hidden[i]),
                crate::cli::MatchMode::Fuzzy => self.calculate_match_score(&self.hidden[i]),
            };
            if let Some(score) = score {
                self.hidden[i].score = score;
                self.shown.push(self.hidden.remove(i));
            } else {
                i += 1;
            }
        }

        // Sort by score
        self.shown.sort();

        // Reset selection to beginning and scroll offset
        if self.shown.is_empty() {
            // Can't select anything if there's no items
            self.selected = None;
            self.scroll_offset = 0;
        } else {
            // Keep selection valid after filtering
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
    
    /// Calculate fuzzy match score across app fields
    /// Optimized to check high-priority fields first and exit early
    fn calculate_match_score(&self, app: &xdg::App) -> Option<i64> {
        if self.query.is_empty() {
            return Some(0);
        }
        
        // Reuse query_lower if already computed
        let query_lower = self.query.to_lowercase();
        let mut best_score = None;
        
        // Extract executable name from command
        let exec_name = crate::helpers::extract_exec_name(&app.command);
        
        // Only lowercase if we need to check it
        if !exec_name.is_empty() {
            let exec_name_lower = exec_name.to_lowercase();
        
            // Check executable name first (highest priority for direct matching)
            if exec_name_lower == query_lower {
                // Exact executable match - apply boosts and return early
                let mut score = 1_000_000;
                if app.pinned {
                    score += 50_000;
                }
                score += app.history as i64 * 10;
                return Some(score);
            } else if exec_name_lower.starts_with(&query_lower) {
                // Executable prefix match - very high priority
                best_score = Some(900_000);
            } else if let Some(score) = self.matcher.fuzzy_match(exec_name, &self.query) {
                // Fuzzy executable match (high priority)
                best_score = Some(score * 4);
            }
        }
        
        // Match against app name
        let app_name_lower = app.name.to_lowercase();
        if app_name_lower == query_lower {
            // Exact name match - apply boosts and return early
            let mut score = 800_000;
            if app.pinned {
                score += 50_000;
            }
            score += app.history as i64 * 10;
            return Some(score);
        } else if app_name_lower.starts_with(&query_lower) {
            // Prefix match - high priority
            let score = 700_000 + 10000;
            best_score = Some(best_score.map_or(score, |current| current.max(score)));
        } else if let Some(mut score) = self.matcher.fuzzy_match(&app.name, &self.query) {
            // Word boundary matches (e.g., "fire" matches "Firefox")
            if app_name_lower.split_whitespace().any(|word| word.starts_with(&query_lower)) {
                score += 5000;
            }
            
            let boosted_score = score * 3;
            best_score = Some(best_score.map_or(boosted_score, |current| current.max(boosted_score)));
        }
        
        // Match against generic name
        if let Some(ref generic_name) = app.generic_name {
            let generic_lower = generic_name.to_lowercase();
            if let Some(mut score) = self.matcher.fuzzy_match(generic_name, &self.query) {
                if generic_lower == query_lower {
                    score = 700_000;
                } else if generic_lower.starts_with(&query_lower) {
                    score += 8000;
                }
                let boosted_score = score * 2;
                best_score = Some(best_score.map_or(boosted_score, |current| current.max(boosted_score)));
            }
        }
        
        // Match against keywords
        for keyword in &app.keywords {
            let keyword_lower = keyword.to_lowercase();
            if let Some(mut score) = self.matcher.fuzzy_match(keyword, &self.query) {
                if keyword_lower == query_lower {
                    score = 600_000;
                } else if keyword_lower.starts_with(&query_lower) {
                    score += 6000;
                }
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
        
        // add pinned app boost (highest priority after exact matches)
        if let Some(mut score) = best_score {
            if app.pinned {
                // pinned apps get massive boost, but not above exact matches
                if score < 600_000 {
                    score += 500_000; // boost fuzzy matches significantly
                } else {
                    score += 50_000; // boost exact matches slightly
                }
            }
            
            // add usage history boost (but don't let it dominate exact/prefix matches)
            score = if score >= 600_000 {
                // for exact/prefix matches, history is just a tiebreaker
                score + (app.history as i64 * 10)
            } else {
                // for fuzzy matches, history adds significant boost
                score + (app.history as i64 * 100)
            };
            
            best_score = Some(score);
        }
        
        best_score
    }
    
    /// Calculate exact match score (case-insensitive)
    /// Optimized to minimize string allocations and exit early
    fn calculate_exact_match_score(&self, app: &xdg::App) -> Option<i64> {
        if self.query.is_empty() {
            return Some(0);
        }
        
        let query_lower = self.query.to_lowercase();
        
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
    /// Match mode (exact or fuzzy)
    pub match_mode: crate::cli::MatchMode,
    /// Match against specific columns
    pub match_nth: Option<Vec<usize>>,
    /// Tag mode state
    /// DISABLED: Waiting for cclip maintainer to add tag support
    #[allow(dead_code)]
    pub tag_mode: TagMode,
    #[doc(hidden)]
    // Matching algorithm
    matcher: SkimMatcherV2,
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
            matcher: SkimMatcherV2::default(),
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
        // Hide items that don't match the current filter
        let mut i = 0;
        while i != self.shown.len() {
            let score = self.calculate_item_score(&self.shown[i]);
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
            if let Some(score) = self.calculate_item_score(&self.hidden[i]) {
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
    
    /// Calculate score for an item based on match mode and match_nth
    fn calculate_item_score(&self, item: &DmenuItem) -> Option<i64> {
        if let Some(ref match_cols) = self.match_nth {
            // Match against specific columns
            item.calculate_score_with_match_nth(&self.query, &self.matcher, match_cols)
        } else {
            // Match against display text
            match self.match_mode {
                crate::cli::MatchMode::Exact => item.calculate_exact_score(&self.query),
                crate::cli::MatchMode::Fuzzy => item.calculate_score(&self.query, &self.matcher),
            }
        }
    }
}
