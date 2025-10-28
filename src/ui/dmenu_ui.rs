use nucleo_matcher::{Config, Matcher};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::common::Item;

/// Dmenu-specific UI for filtering and sorting
pub struct DmenuUI<'a> {
    /// Hidden items (They don't match the current query)
    pub hidden: Vec<Item>,
    /// Shown items (They match the current query)
    pub shown: Vec<Item>,
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
    pub tag_mode: TagMode,
    /// Cache for clipboard content to avoid repeated cclip calls
    content_cache: std::collections::HashMap<String, String>,
    /// Temporary error/info message with expiration time
    pub temp_message: Option<(String, std::time::Instant)>,
    #[doc(hidden)]
    // Matching algorithm (SIMD-accelerated)
    matcher: Matcher,
}

/// Tag mode state for cclip
#[derive(Debug, Clone, PartialEq)]
pub enum TagMode {
    /// Normal mode (not tagging)
    Normal,
    /// Prompting for tag name
    PromptingTagName { 
        input: String, 
        selected_item: Option<String>,
        available_tags: Vec<String>,
        selected_tag: Option<usize>,
    },
    /// Prompting for tag emoji
    PromptingTagEmoji { tag_name: String, input: String, selected_item: Option<String> },
    /// Prompting for tag color
    PromptingTagColor {
        tag_name: String,
        emoji: Option<String>,
        input: String,
        selected_item: Option<String>,
    },
    /// Prompting for tag removal (blank removes all)
    RemovingTag {
        input: String,
        tags: Vec<String>,
        selected: Option<usize>,
        selected_item: Option<String>,
    },
}

impl<'a> DmenuUI<'a> {
    /// Creates a new DmenuUI from a Vec of Items
    pub fn new(
        items: Vec<Item>,
        wrap_long_lines: bool,
        show_line_numbers: bool,
    ) -> DmenuUI<'a> {
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
            content_cache: std::collections::HashMap::new(),
            temp_message: None,
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

    /// Set a temporary message that expires after 2 seconds
    pub fn set_temp_message(&mut self, message: String) {
        self.temp_message = Some((message, std::time::Instant::now()));
    }

    /// Clear temporary message if expired (2 seconds) or on selection change
    pub fn clear_expired_message(&mut self) {
        if let Some((_, timestamp)) = &self.temp_message {
            if timestamp.elapsed() > std::time::Duration::from_secs(2) {
                self.temp_message = None;
            }
        }
    }

    /// Force clear temporary message
    #[allow(dead_code)]
    pub fn clear_temp_message(&mut self) {
        self.temp_message = None;
    }

    pub fn cycle_removal_selection(&mut self, direction: i32) {
        if let TagMode::RemovingTag { tags, selected, input, .. } = &mut self.tag_mode {
            if tags.is_empty() {
                *selected = None;
                return;
            }

            let len = tags.len() as i32;
            let current = selected.map(|idx| idx as i32).unwrap_or(0);
            let next = (current + direction).rem_euclid(len);
            *selected = Some(next as usize);
            
            // Update input with selected tag
            if let Some(idx) = *selected {
                if idx < tags.len() {
                    *input = tags[idx].clone();
                }
            }
        }
    }

    pub fn cycle_tag_creation_selection(&mut self, direction: i32) {
        if let TagMode::PromptingTagName { available_tags, selected_tag, input, .. } = &mut self.tag_mode {
            if available_tags.is_empty() {
                *selected_tag = None;
                return;
            }

            let len = available_tags.len() as i32;
            let current = selected_tag.map(|idx| idx as i32).unwrap_or(-1);
            let next = (current + direction).rem_euclid(len);
            *selected_tag = Some(next as usize);
            
            // Update input with selected tag name only (not full display)
            if let Some(idx) = *selected_tag {
                if idx < available_tags.len() {
                    // Extract just the tag name without color/emoji formatting
                    let tag = &available_tags[idx];
                    // Remove any formatting like (color) or emoji prefix
                    let clean_tag = tag.split('(').next().unwrap_or(tag).trim();
                    let clean_tag = clean_tag.trim_start_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '-');
                    *input = clean_tag.to_string();
                }
            }
        }
    }

    /// Replace the underlying items while preserving the current query & match settings
    #[allow(dead_code)]
    pub fn set_items(&mut self, items: Vec<Item>) {
        self.hidden = items;
        self.shown.clear();
        // Don't reset selection here - let filter() handle it properly
        self.scroll_offset = 0;
        // Clear content cache when items change
        self.content_cache.clear();
        self.filter();
    }

    /// Update `self.text` to show content for current selection
    pub fn info(&mut self, color: Color) {
        self.info_with_image_support(color, false, false, 0, 0);
    }

    /// Update `self.text` to show content with optional image preview support
    pub fn info_with_image_support(
        &mut self,
        highlight_color: Color,
        enable_images: bool,
        hide_image_message: bool,
        panel_width: u16,
        _panel_height: u16,
    ) {
        match &self.tag_mode {
            TagMode::PromptingTagName { input, available_tags, selected_tag, .. } => {
                let mut text = vec![
                    Line::from(vec![Span::styled(
                        "Tagging Mode",
                        Style::default().add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(""),
                    Line::from("Enter a tag name for this clipboard item."),
                    Line::from("Use Up/Down to browse existing tags."),
                    Line::from(""),
                ];

                // Show available tags if any
                if !available_tags.is_empty() {
                    text.push(Line::from("Existing tags:"));
                    for (idx, tag) in available_tags.iter().enumerate() {
                        let marker = if Some(idx) == *selected_tag { "▶" } else { " " };
                        text.push(Line::from(vec![
                            Span::styled(marker, Style::default().fg(highlight_color)),
                            Span::raw(" "),
                            Span::raw(tag.clone()),
                        ]));
                    }
                    text.push(Line::from(""));
                } else {
                    text.push(Line::from("Examples: prompt, code, important, todo"));
                    text.push(Line::from(""));
                }

                text.extend_from_slice(&[
                    Line::from(vec![
                        Span::styled("Tag: ", Style::default().fg(highlight_color)),
                        Span::styled(input.clone(), Style::default().fg(ratatui::style::Color::White)),
                        Span::styled("▌", Style::default().fg(highlight_color)),
                    ]),
                    Line::from(""),
                ]);
                
                // Show temp message if present
                if let Some((ref msg, _)) = self.temp_message {
                    text.push(Line::from(vec![
                        Span::styled(msg.clone(), Style::default().fg(ratatui::style::Color::Yellow)),
                    ]));
                    text.push(Line::from(""));
                }
                
                text.push(Line::from("Press Enter to continue, Esc to cancel."));

                self.text = text;
                return;
            }
            TagMode::PromptingTagEmoji { tag_name, input, .. } => {
                let mut text = vec![
                    Line::from(vec![Span::styled(
                        "Tag Emoji",
                        Style::default().add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(""),
                ];
                
                // Show temp message if present (e.g., "already applied (editing)")
                if let Some((ref msg, _)) = self.temp_message {
                    text.push(Line::from(vec![
                        Span::styled(msg.clone(), Style::default().fg(ratatui::style::Color::Yellow)),
                    ]));
                    text.push(Line::from(""));
                }
                
                text.extend_from_slice(&[
                    Line::from(format!("Tag: {}", tag_name)),
                    Line::from(""),
                    Line::from("Enter an emoji to prefix the tag (optional):"),
                    Line::from("  Examples: 📌 🔥 ⭐ 💡 📝"),
                    Line::from("  Leave blank to keep existing emoji"),
                    Line::from(vec![
                        Span::styled("Emoji: ", Style::default().fg(highlight_color)),
                        Span::styled(input.clone(), Style::default().fg(ratatui::style::Color::White)),
                        Span::styled("▌", Style::default().fg(highlight_color)),
                    ]),
                    Line::from(""),
                    Line::from("Press Enter to continue, Esc to cancel."),
                ]);
                
                self.text = text;
                return;
            }
            TagMode::PromptingTagColor {
                tag_name,
                emoji,
                input,
                ..
            } => {
                let emoji_display = emoji.as_deref().unwrap_or("(none)");
                let mut text = vec![
                    Line::from(vec![Span::styled(
                        "Tag Color",
                        Style::default().add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(""),
                ];
                
                // Show temp message if present (e.g., "already applied (editing)")
                if let Some((ref msg, _)) = self.temp_message {
                    text.push(Line::from(vec![
                        Span::styled(msg.clone(), Style::default().fg(ratatui::style::Color::Yellow)),
                    ]));
                    text.push(Line::from(""));
                }
                
                text.extend_from_slice(&[
                    Line::from(format!("Tag: {}", tag_name)),
                    Line::from(format!("Emoji: {}", emoji_display)),
                    Line::from(""),
                    Line::from("Enter a color (optional):"),
                    Line::from("  - Hex: #ff0000 or #f00"),
                    Line::from("  - RGB: rgb(255,0,0)"),
                    Line::from("  - Named: red, blue, green, etc."),
                    Line::from("  - Leave blank to keep existing color"),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Color: ", Style::default().fg(highlight_color)),
                        Span::styled(input.clone(), Style::default().fg(ratatui::style::Color::White)),
                        Span::styled("▌", Style::default().fg(highlight_color)),
                    ]),
                    Line::from(""),
                    Line::from("Press Enter to finish, Esc to cancel."),
                ]);
                
                self.text = text;
                return;
            }
            TagMode::RemovingTag {
                input,
                tags,
                selected,
                ..
            } => {
                let mut text = vec![
                    Line::from(vec![Span::styled(
                        "Remove Tag",
                        Style::default().add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(""),
                ];

                if tags.is_empty() {
                    text.push(Line::from("No tags assigned to this entry."));
                    text.push(Line::from(""));
                } else {
                    text.push(Line::from("Use Up/Down to choose a tag, Enter to confirm."));
                    text.push(Line::from(
                        "Leave blank and press Enter to remove all tags.",
                    ));
                    text.push(Line::from(""));

                    for (idx, tag) in tags.iter().enumerate() {
                        let marker = if Some(idx) == *selected { "▶" } else { " " };
                        text.push(Line::from(vec![
                            Span::styled(marker, Style::default().fg(highlight_color)),
                            Span::raw(" "),
                            Span::raw(tag.clone()),
                        ]));
                    }

                    text.push(Line::from(""));
                }

                text.extend_from_slice(&[
                    Line::from("Type to filter or add a tag name manually."),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Tag: ", Style::default().fg(highlight_color)),
                        Span::styled(input.clone(), Style::default().fg(ratatui::style::Color::White)),
                        Span::styled("▌", Style::default().fg(highlight_color)),
                    ]),
                    Line::from(""),
                    Line::from("Press Enter to confirm, Esc to cancel."),
                ]);

                self.text = text;
                return;
            }
            TagMode::Normal => {}
        }

        if let Some(selected) = self.selected {
            if selected < self.shown.len() {
                let item_clone = self.shown[selected].clone();

                // Check if this is a cclip image item and image previews are enabled
                if enable_images && self.is_cclip_image_item(&item_clone) {
                    if hide_image_message {
                        // Show minimal or blank content for images
                        self.text = vec![Line::from(Span::raw(String::new()))];
                    } else {
                        // Placeholder text (image drawn after ratatui)
                        let image_info = self.get_image_info(&item_clone);
                        self.text = vec![
                            Line::from(Span::raw("  [INLINE IMAGE PREVIEW]".to_string())),
                            Line::from(Span::raw(image_info)),
                            Line::from(Span::raw(String::new())),
                            Line::from(Span::raw("󱇛 Press 'i' for fullscreen view".to_string())),
                            Line::from(Span::raw(
                                " Press 'Enter' to copy to clipboard".to_string(),
                            )),
                            Line::from(Span::raw(String::new())),
                            Line::from(Span::raw("Loading image preview...".to_string())),
                        ];
                    }
                    return;
                }

                // For cclip items, get the actual clipboard content
                let content = if self.is_cclip_item(&item_clone) {
                    self.get_cclip_content_for_display(&item_clone)
                } else {
                    item_clone.get_content_display()
                };

                // Simple content handling - just limit length, don't filter aggressively
                let safe_content = if content.is_empty() {
                    "[Empty content]".to_string()
                } else if content.len() > 5000 {
                    format!("{}...", &content[..5000])
                } else {
                    content
                };

                // Create content display with optional line numbers
                let mut lines = Vec::new();

                // Add line number if enabled
                let mut display_content = if self.show_line_numbers {
                    format!("{}  {}", item_clone.line_number, safe_content)
                } else {
                    safe_content
                };

                // ALWAYS replace newlines to prevent Foot terminal rendering issues
                // Foot's ascii_printer_generic mode (used with sixel images) breaks with newlines
                display_content = display_content.replace('\n', " ");

                if self.wrap_long_lines {
                    // Wrap by actual panel width (accounting for borders and padding)
                    let max_width = (panel_width.saturating_sub(4)) as usize; // -2 for borders, -2 for padding
                    let max_width = max_width.max(20); // Minimum 20 chars
                    
                    let chars: Vec<char> = display_content.chars().collect();
                    let mut start = 0;
                    while start < chars.len() {
                        let end = std::cmp::min(start + max_width, chars.len());
                        let chunk: String = chars[start..end].iter().collect();
                        lines.push(Line::from(Span::raw(chunk)));
                        start = end;
                    }
                } else {
                    // Keep as single line
                    lines.push(Line::from(Span::raw(display_content)));
                }

                // Ensure we always have at least one line to prevent empty display
                if lines.is_empty() {
                    lines.push(Line::from(Span::raw("[No content]")));
                }

                // For Sixel/Foot: Pad with empty lines to fill the entire panel height
                // This ensures text overwrites the ENTIRE sixel area, not just where text is
                let target_height = (_panel_height as usize).saturating_sub(2); // Account for borders
                while lines.len() < target_height {
                    lines.push(Line::from(Span::raw(" "))); // Empty line with space
                }

                self.text = lines;
            }
        } else {
            // Clear info if no selection
            self.text.clear();
        }
    }

    /// Check if an Item is a cclip item (has tab-separated format with rowid)
    fn is_cclip_item(&self, item: &crate::common::Item) -> bool {
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

    /// Check if an Item is a cclip image item by parsing its original line
    fn is_cclip_image_item(&self, item: &crate::common::Item) -> bool {
        // Parse the tab-separated cclip format to check mime type
        // Format: rowid\tmime_type\tpreview[\ttags]
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

    /// Get actual clipboard content for display
    fn get_cclip_content_for_display(&mut self, item: &crate::common::Item) -> String {
        let parts: Vec<&str> = item.original_line.splitn(4, '\t').collect();
        
        if parts.len() >= 3 {
            let rowid = parts[0].trim();
            let _mime_type = parts[1].trim();
            let preview = parts[2].trim();
            
            // Check cache first
            if let Some(cached_content) = self.content_cache.get(rowid) {
                return cached_content.clone();
            }
            
            // Always try to get the full content - no filtering, show everything
            if let Ok(output) = std::process::Command::new("cclip")
                .args(&["get", rowid])
                .output()
            {
                if output.status.success() {
                    if let Ok(content) = String::from_utf8(output.stdout) {
                        // Don't cache empty content
                        if !content.trim().is_empty() {
                            self.content_cache.insert(rowid.to_string(), content.clone());
                            return content;
                        }
                    }
                }
            }
            
            // Only fallback to preview if cclip get completely fails
            if !preview.is_empty() {
                preview.to_string()
            } else {
                format!("[Failed to get content for rowid {}]", rowid)
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
    fn get_image_info(&self, item: &crate::common::Item) -> String {
        let parts: Vec<&str> = item.original_line.splitn(3, '\t').collect();
        if parts.len() >= 3 {
            let mime_type = parts[1].trim();
            let preview = parts[2].trim();
            format!("Type: {}\nInfo: {}", mime_type, preview)
        } else {
            "Image information unavailable".to_string()
        }
    }

    /// Get the rowid for any cclip item (not just images)
    pub fn get_cclip_rowid(&self, item: &crate::common::Item) -> Option<String> {
        if !self.is_cclip_item(item) {
            return None;
        }

        let parts: Vec<&str> = item.original_line.splitn(3, '\t').collect();
        if parts.len() >= 1 {
            Some(parts[0].trim().to_string())
        } else {
            None
        }
    }

    /// Display image directly to terminal, bypassing ratatui
    /// Returns true if image was displayed successfully
    pub fn display_image_to_terminal(&self, item: &crate::common::Item) -> bool {
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
                // Clear screen - skip for Foot terminal to avoid rendering issues
                if !terminal_type.starts_with("foot") {
                    let _ = std::process::Command::new("clear")
                        .env("TERM", "xterm-256color")
                        .status();
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
                self.shown[i].calculate_score_with_match_nth(
                    &self.query,
                    &mut self.matcher,
                    match_cols,
                )
            } else {
                match self.match_mode {
                    crate::cli::MatchMode::Exact => {
                        self.shown[i].calculate_exact_score(&self.query)
                    }
                    crate::cli::MatchMode::Fuzzy => {
                        self.shown[i].calculate_score(&self.query, &mut self.matcher)
                    }
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
                self.hidden[i].calculate_score_with_match_nth(
                    &self.query,
                    &mut self.matcher,
                    match_cols,
                )
            } else {
                match self.match_mode {
                    crate::cli::MatchMode::Exact => {
                        self.hidden[i].calculate_exact_score(&self.query)
                    }
                    crate::cli::MatchMode::Fuzzy => {
                        self.hidden[i].calculate_score(&self.query, &mut self.matcher)
                    }
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

        // Reset selection and scroll on any filter change (like rofi)
        if self.shown.is_empty() {
            self.selected = None;
            self.scroll_offset = 0;
        } else {
            self.selected = Some(0);
            self.scroll_offset = 0;
        }
    }
}
