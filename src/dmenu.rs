use std::io::{self, BufRead};
use is_terminal::IsTerminal;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
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

/// Represents a dmenu item with column parsing capabilities
#[derive(Clone, Debug)]
pub struct DmenuItem {
    /// The original full line from stdin
    pub original_line: String,
    /// The display text (after column filtering)
    pub display_text: String,
    /// Parsed columns (split by delimiter)
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
                        Some(columns[col_idx - 1].clone()) // Convert 1-based to 0-based
                    } else {
                        None
                    }
                })
                .collect();
            displayed_cols.join(" ")
        } else {
            // Show the full line
            original_line.clone()
        };

        Self {
            original_line,
            display_text,
            columns,
            score: 0,
            line_number,
        }
    }

    /// Calculate fuzzy match score against query
    pub fn calculate_score(&self, query: &str, matcher: &SkimMatcherV2) -> Option<i64> {
        if query.is_empty() {
            return Some(0);
        }

        // Try to match against display text first
        if let Some(score) = matcher.fuzzy_match(&self.display_text, query) {
            return Some(score * 2); // Boost display text matches
        }

        // Fallback to matching against original line
        matcher.fuzzy_match(&self.original_line, query)
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
        if self.is_image() {
            format!("[IMAGE] {}", self.original_line)
        } else {
            self.original_line.clone()
        }
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
