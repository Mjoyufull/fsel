use std::io::{self, BufRead};
use is_terminal::IsTerminal;
use nucleo_matcher::{Matcher, Utf32Str};
use ratatui::widgets::ListItem;

/// Check if stdin is being piped to us
pub fn is_stdin_piped() -> bool {
    !io::stdin().is_terminal()
}

/// Read all lines from stdin into a vector
pub fn read_stdin_lines() -> io::Result<Vec<String>> {
    let stdin = io::stdin();
    let lines: Result<Vec<String>, io::Error> = stdin.lock().lines().collect();
    lines
}

/// Read null-separated input from stdin
pub fn read_stdin_null_separated() -> io::Result<Vec<String>> {
    use std::io::Read;
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    
    let lines: Vec<String> = buffer
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    
    Ok(lines)
}

/// Represents a dmenu item with column parsing capabilities
#[derive(Clone, Debug)]
pub struct DmenuItem {
    pub original_line: String,
    pub display_text: String,
    /// Columns split by delimiter
    pub columns: Vec<String>,
    /// Matching score for fuzzy search
    pub score: i64,
    /// Line number (1-based) for display
    pub line_number: usize,
}

impl DmenuItem {
    /// Create a new DmenuItem from a line and parsing options
    pub fn new(
        original_line: String,
        line_number: usize,
        delimiter: &str,
        with_nth: Option<&Vec<usize>>,
    ) -> Self {
        // Split the line by delimiter
        let columns: Vec<String> = if delimiter == " " {
            // Special case for space: split by whitespace and filter empty
            original_line.split_whitespace().map(|s| s.to_string()).collect()
        } else if delimiter == "\t" {
            // Special handling for tab delimiter - preserve tabs for column parsing
            original_line.split('\t').map(|s| s.to_string()).collect()
        } else {
            original_line.split(delimiter).map(|s| s.to_string()).collect()
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
                format!("<no column {} found>", nth_cols.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(","))
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
        }
    }
    
    /// Create a new DmenuItem with simple display text (used for cclip integration)
    pub fn new_simple(original_line: String, display_text: String, line_number: usize) -> Self {
        let columns = vec![original_line.clone()];
        Self {
            original_line,
            display_text,
            columns,
            score: 0,
            line_number,
        }
    }

    /// Calculate fuzzy match score against query (SIMD-accelerated)
    /// Optimized to check display text first with early return
    #[inline]
    pub fn calculate_score(&self, query: &str, matcher: &mut Matcher) -> Option<i64> {
        if query.is_empty() {
            return Some(0);
        }

        // Try to match against display text first (most common case)
        let query_lower = query.to_lowercase();
        let mut query_chars = Vec::new();
        let query_utf32 = Utf32Str::new(&query_lower, &mut query_chars);
        
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
        matcher.fuzzy_match(original_utf32, query_utf32).map(|s| s as i64)
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
            || (query.starts_with('\'') && query.ends_with('\'')) {
            // Strip quotes and require exact match
            (true, &query[1..query.len()-1])
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
                    best_score = Some(best_score.map_or(score as i64, |current: i64| current.max(score as i64)));
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
        
        // Format tab-separated content nicely for display
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
}

impl PartialEq for DmenuItem {
    fn eq(&self, other: &Self) -> bool {
        self.original_line == other.original_line
    }
}

impl Eq for DmenuItem {}

impl PartialOrd for DmenuItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DmenuItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Sort by score (higher scores first), then by line number (original order)
        self.score
            .cmp(&other.score)
            .reverse()
            .then(self.line_number.cmp(&other.line_number))
    }
}

impl std::fmt::Display for DmenuItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_text)
    }
}

impl AsRef<str> for DmenuItem {
    fn as_ref(&self) -> &str {
        &self.display_text
    }
}

impl<'a> From<DmenuItem> for ListItem<'a> {
    fn from(item: DmenuItem) -> ListItem<'a> {
        ListItem::new(item.display_text)
    }
}

impl<'a> From<&'a DmenuItem> for ListItem<'a> {
    fn from(item: &'a DmenuItem) -> ListItem<'a> {
        ListItem::new(item.display_text.clone())
    }
}

/// Parse stdin lines into DmenuItems
pub fn parse_stdin_to_items(
    lines: Vec<String>,
    delimiter: &str,
    with_nth: Option<&Vec<usize>>,
) -> Vec<DmenuItem> {
    lines
        .into_iter()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty()) // Skip empty lines
        .map(|(idx, line)| DmenuItem::new(line, idx + 1, delimiter, with_nth))
        .collect()
}
