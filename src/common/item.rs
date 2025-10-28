// Shared item structure used by dmenu and cclip modes
// This represents a filterable/searchable item with metadata

use nucleo_matcher::{Matcher, Utf32Str};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::ListItem;

/// Represents a filterable item with column parsing capabilities
/// Used by dmenu mode and cclip mode for displaying and filtering items
#[derive(Clone, Debug)]
pub struct Item {
    pub original_line: String,
    pub display_text: String,
    /// Columns split by delimiter
    pub columns: Vec<String>,
    /// Matching score for fuzzy search
    pub score: i64,
    /// Line number (1-based) for display
    pub line_number: usize,
    /// Tags for cclip items (None for dmenu items)
    pub tags: Option<Vec<String>>,
}

impl Item {
    /// Create a new Item from a line and parsing options
    pub fn new(
        original_line: String,
        line_number: usize,
        delimiter: &str,
        with_nth: Option<&Vec<usize>>,
    ) -> Self {
        // Split the line by delimiter
        let columns: Vec<String> = if delimiter == " " {
            // Special case for space: split by whitespace and filter empty
            original_line
                .split_whitespace()
                .map(|s| s.to_string())
                .collect()
        } else if delimiter == "\t" {
            // Special handling for tab delimiter - preserve tabs for column parsing
            original_line.split('\t').map(|s| s.to_string()).collect()
        } else {
            original_line
                .split(delimiter)
                .map(|s| s.to_string())
                .collect()
        };

        // Determine display text based on with_nth
        let display_text = if let Some(nth_cols) = with_nth {
            // Show only specified columns
            let displayed_cols: Vec<String> = nth_cols
                .iter()
                .filter_map(|&col_idx| {
                    if col_idx > 0 && col_idx <= columns.len() {
                        let col_text = columns[col_idx - 1].clone(); // Convert 1-based to 0-based
                        if !col_text.is_empty() {
                            Some(col_text)
                        } else {
                            Some("<empty>".to_string()) // Show placeholder for empty columns
                        }
                    } else {
                        None // Column index out of bounds
                    }
                })
                .collect();
            let result = displayed_cols.join(" ");
            if result.is_empty() {
                format!(
                    "<no column {} found>",
                    nth_cols
                        .iter()
                        .map(|n| n.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                )
            } else {
                result
            }
        } else {
            // Show the full line - format tab-separated content nicely
            if delimiter == "\t" && columns.len() > 1 {
                // For tab-separated content like cliphist, format nicely with padding
                if columns[0].parse::<u64>().is_ok() {
                    // Numeric first column (cliphist ID), add padding
                    format!("{:<6} {}", columns[0], columns[1..].join(" "))
                } else {
                    // Non-numeric, just use regular spacing
                    columns.join("  ")
                }
            } else {
                // For other content, replace tabs with double spaces
                original_line.replace('\t', "  ")
            }
        };

        Self {
            original_line,
            display_text,
            columns,
            score: 0,
            line_number,
            tags: None, // dmenu items don't have tags
        }
    }

    /// Create a new Item with simple display text (used for cclip integration)
    pub fn new_simple(original_line: String, display_text: String, line_number: usize) -> Self {
        let columns = vec![original_line.clone()];
        Self {
            original_line,
            display_text,
            columns,
            score: 0,
            line_number,
            tags: None, // will be set by From<CclipItem> if applicable
        }
    }

    /// Calculate fuzzy match score against query (SIMD-accelerated)
    /// Optimized to check display text first with early return
    /// For cclip items, prioritizes tag name matches
    #[inline]
    pub fn calculate_score(&self, query: &str, matcher: &mut Matcher) -> Option<i64> {
        if query.is_empty() {
            return Some(0);
        }

        let query_lower = query.to_lowercase();
        let mut query_chars = Vec::new();
        let query_utf32 = Utf32Str::new(&query_lower, &mut query_chars);

        // For cclip items, check if query matches any tag names first (highest priority)
        if let Ok(cclip_item) =
            crate::modes::cclip::CclipItem::from_line(self.original_line.clone())
        {
            for tag in &cclip_item.tags {
                let tag_lower = tag.to_lowercase();

                // Exact tag name match gets highest priority
                if tag_lower == query_lower {
                    return Some(1_000_000);
                }

                // Tag name prefix match gets very high priority
                if tag_lower.starts_with(&query_lower) {
                    return Some(800_000);
                }

                // Fuzzy tag name match gets high priority
                let mut tag_chars = Vec::new();
                let tag_utf32 = Utf32Str::new(&tag_lower, &mut tag_chars);
                if let Some(score) = matcher.fuzzy_match(tag_utf32, query_utf32) {
                    return Some((score as i64) * 10); // 10x boost for tag matches
                }
            }
        }

        // Try to match against display text (normal priority)
        let display_lower = self.display_text.to_lowercase();
        let mut display_chars = Vec::new();
        let display_utf32 = Utf32Str::new(&display_lower, &mut display_chars);
        if let Some(score) = matcher.fuzzy_match(display_utf32, query_utf32) {
            return Some((score as i64) * 2); // Boost display text matches
        }

        // Fallback to matching against original line
        let original_lower = self.original_line.to_lowercase();
        let mut original_chars = Vec::new();
        let original_utf32 = Utf32Str::new(&original_lower, &mut original_chars);
        matcher
            .fuzzy_match(original_utf32, query_utf32)
            .map(|s| s as i64)
    }

    /// Calculate exact match score against query
    /// Supports quoted strings for exact matching (e.g., "firefox")
    /// Optimized with early returns
    #[inline]
    pub fn calculate_exact_score(&self, query: &str) -> Option<i64> {
        if query.is_empty() {
            return Some(0);
        }

        // Check if query is wrapped in quotes for exact match
        let (is_quoted, search_query) = if (query.starts_with('"') && query.ends_with('"'))
            || (query.starts_with('\'') && query.ends_with('\''))
        {
            // Strip quotes and require exact match
            (true, &query[1..query.len() - 1])
        } else {
            (false, query)
        };

        let query_lower = search_query.to_lowercase();
        let display_lower = self.display_text.to_lowercase();

        if is_quoted {
            // Quoted: only exact match
            if display_lower == query_lower {
                return Some(1000);
            }
            return None;
        }

        // Unquoted: exact, prefix, or contains
        if display_lower == query_lower {
            return Some(1000);
        }

        if display_lower.starts_with(&query_lower) {
            return Some(500);
        }

        if display_lower.contains(&query_lower) {
            return Some(100);
        }

        None
    }

    /// Calculate match score based on match_nth columns (SIMD-accelerated)
    pub fn calculate_score_with_match_nth(
        &self,
        query: &str,
        matcher: &mut Matcher,
        match_nth: &[usize],
    ) -> Option<i64> {
        if query.is_empty() {
            return Some(0);
        }

        let mut best_score = None;

        let query_lower = query.to_lowercase();
        let mut query_chars = Vec::new();
        let query_utf32 = Utf32Str::new(&query_lower, &mut query_chars);

        for &col_idx in match_nth {
            if col_idx > 0 && col_idx <= self.columns.len() {
                let col_text = &self.columns[col_idx - 1];
                let col_lower = col_text.to_lowercase();
                let mut col_chars = Vec::new();
                let col_utf32 = Utf32Str::new(&col_lower, &mut col_chars);
                if let Some(score) = matcher.fuzzy_match(col_utf32, query_utf32) {
                    best_score = Some(
                        best_score.map_or(score as i64, |current: i64| current.max(score as i64)),
                    );
                }
            }
        }

        best_score
    }

    /// Get output based on accept_nth columns
    pub fn get_accept_nth_output(&self, accept_nth: &[usize]) -> String {
        let accepted_cols: Vec<String> = accept_nth
            .iter()
            .filter_map(|&col_idx| {
                if col_idx > 0 && col_idx <= self.columns.len() {
                    Some(self.columns[col_idx - 1].clone())
                } else {
                    None
                }
            })
            .collect();

        if accepted_cols.is_empty() {
            self.original_line.clone()
        } else {
            accepted_cols.join("\t")
        }
    }

    /// Update the score
    pub fn set_score(&mut self, score: i64) {
        self.score = score;
    }

    /// Check if this item contains an image (basic heuristic)
    pub fn is_image(&self) -> bool {
        let text = self.original_line.to_lowercase();
        text.contains(".png")
            || text.contains(".jpg")
            || text.contains(".jpeg")
            || text.contains(".gif")
            || text.contains(".bmp")
            || text.contains(".webp")
            || text.contains(".svg")
            || text.contains("image/")
            || text.contains("img:")
    }

    /// Get content for display in the main panel (description area)
    pub fn get_content_display(&self) -> String {
        let content = if self.is_image() {
            format!("[IMAGE] {}", self.original_line)
        } else {
            self.original_line.clone()
        };

        // Format tab-separated content nicely for display (cclip adds optional tags column after preview)
        if content.contains('\t') {
            let parts: Vec<&str> = content.split('\t').collect();
            if parts.len() >= 2 && parts[0].parse::<u64>().is_ok() {
                // Numeric ID, add padding
                format!("{:<6} {}", parts[0], parts[1..].join("  "))
            } else {
                // Replace tabs with spaces
                content.replace('\t', "  ")
            }
        } else {
            content
        }
    }

    /// Get the original line with any terminal escape sequences stripped
    pub fn get_clean_original_line(&self) -> String {
        // Strip terminal escape sequences (ANSI codes)
        strip_ansi_escapes::strip_str(&self.original_line)
    }

    /// Create a ListItem with optional tag metadata formatting
    pub fn to_list_item<'a>(
        &'a self,
        tag_metadata: Option<&'a crate::modes::cclip::TagMetadataFormatter>,
    ) -> ListItem<'a> {
        if let Some(actual_tags) = &self.tags {
            if !actual_tags.is_empty() {
                if let Some(formatter) = tag_metadata {
                    // Use actual tags from self.tags, not parsed from display_text
                    let mut spans = Vec::new();

                    // Find where tags are in display_text to split properly
                    if let Some(tag_start) = self.display_text.find('[') {
                        if let Some(tag_end) = self.display_text.find(']') {
                            // Add text before tags
                            if tag_start > 0 {
                                spans.push(Span::raw(&self.display_text[..tag_start]));
                            }

                            // Get first tag color for brackets
                            let first_tag_color = actual_tags
                                .first()
                                .and_then(|tag| formatter.get_color(tag))
                                .unwrap_or(ratatui::style::Color::Green);

                            // Opening bracket with first tag color
                            spans.push(Span::styled("[", Style::default().fg(first_tag_color)));

                            // Format each tag individually with its own color
                            // Extract the formatted tags from display_text to preserve color names
                            let formatted_tags =
                                if let Some(tag_start_idx) = self.display_text.find('[') {
                                    if let Some(tag_end_idx) = self.display_text.find(']') {
                                        let tags_str =
                                            &self.display_text[tag_start_idx + 1..tag_end_idx];
                                        tags_str.split(", ").collect::<Vec<&str>>()
                                    } else {
                                        vec![]
                                    }
                                } else {
                                    vec![]
                                };

                            for (idx, tag_name) in actual_tags.iter().enumerate() {
                                let tag_color = formatter
                                    .get_color(tag_name)
                                    .unwrap_or(ratatui::style::Color::Green);

                                // Use the formatted tag from display_text if available (includes color names)
                                let display = if idx < formatted_tags.len() {
                                    formatted_tags[idx].to_string()
                                } else {
                                    // Fallback: format manually
                                    let mut display = String::new();
                                    if let Some(meta) = formatter.metadata.get(tag_name) {
                                        if let Some(emoji) = &meta.emoji {
                                            display.push_str(emoji);
                                            display.push(' ');
                                        }
                                    }
                                    display.push_str(tag_name);
                                    display
                                };

                                spans.push(Span::styled(display, Style::default().fg(tag_color)));

                                // Add comma separator if not last tag
                                if idx < actual_tags.len() - 1 {
                                    spans.push(Span::styled(
                                        ", ",
                                        Style::default().fg(first_tag_color),
                                    ));
                                }
                            }

                            // Closing bracket with first tag color
                            spans.push(Span::styled("]", Style::default().fg(first_tag_color)));

                            // Add content after tags
                            if tag_end + 2 < self.display_text.len() {
                                spans.push(Span::raw(&self.display_text[tag_end + 2..]));
                            }

                            return ListItem::new(Line::from(spans));
                        }
                    }
                }
            }
        }

        // Fallback to simple display
        ListItem::new(self.display_text.clone())
    }
}

impl<'a> From<Item> for ListItem<'a> {
    fn from(item: Item) -> ListItem<'a> {
        ListItem::new(item.display_text)
    }
}

impl<'a> From<&'a Item> for ListItem<'a> {
    fn from(item: &'a Item) -> ListItem<'a> {
        ListItem::new(item.display_text.clone())
    }
}

impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        self.original_line == other.original_line
    }
}

impl Eq for Item {}

impl PartialOrd for Item {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Item {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Sort by score (higher scores first), then by line number (original order)
        self.score
            .cmp(&other.score)
            .reverse()
            .then(self.line_number.cmp(&other.line_number))
    }
}

impl std::fmt::Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_text)
    }
}

impl AsRef<str> for Item {
    fn as_ref(&self) -> &str {
        &self.display_text
    }
}
