//! Shared item structure used by dmenu and cclip modes.

mod display;
mod matching;

use ratatui::widgets::ListItem;

/// Represents a filterable item with column parsing capabilities.
#[derive(Clone, Debug)]
pub struct Item {
    pub original_line: String,
    pub display_text: String,
    /// Columns split by delimiter.
    pub columns: Vec<String>,
    /// Matching score for fuzzy search.
    pub score: i64,
    /// Line number (1-based) for display.
    pub line_number: usize,
    /// Tags for cclip items (`None` for dmenu items).
    pub tags: Option<Vec<String>>,
}

impl Item {
    /// Create a new `Item` from a line and parsing options.
    pub fn new(
        original_line: String,
        line_number: usize,
        delimiter: &str,
        with_nth: Option<&Vec<usize>>,
    ) -> Self {
        let columns = parse_columns(&original_line, delimiter);
        let display_text = build_display_text(&original_line, &columns, delimiter, with_nth);

        Self {
            original_line,
            display_text,
            columns,
            score: 0,
            line_number,
            tags: None,
        }
    }

    /// Create a new item with simple display text (used for cclip integration).
    pub fn new_simple(original_line: String, display_text: String, line_number: usize) -> Self {
        let columns = vec![original_line.clone()];
        Self {
            original_line,
            display_text,
            columns,
            score: 0,
            line_number,
            tags: None,
        }
    }

    /// Update the score.
    pub fn set_score(&mut self, score: i64) {
        self.score = score;
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
        self.score == other.score && self.line_number == other.line_number
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

fn parse_columns(original_line: &str, delimiter: &str) -> Vec<String> {
    if delimiter == " " {
        original_line
            .split_whitespace()
            .map(|part| part.to_string())
            .collect()
    } else if delimiter == "\t" {
        original_line
            .split('\t')
            .map(|part| part.to_string())
            .collect()
    } else {
        original_line
            .split(delimiter)
            .map(|part| part.to_string())
            .collect()
    }
}

fn build_display_text(
    original_line: &str,
    columns: &[String],
    delimiter: &str,
    with_nth: Option<&Vec<usize>>,
) -> String {
    if let Some(nth_cols) = with_nth {
        let displayed_cols: Vec<String> = nth_cols
            .iter()
            .filter_map(|&col_idx| {
                if col_idx > 0 && col_idx <= columns.len() {
                    let col_text = columns[col_idx - 1].clone();
                    if !col_text.is_empty() {
                        Some(col_text)
                    } else {
                        Some("<empty>".to_string())
                    }
                } else {
                    None
                }
            })
            .collect();

        let display_text = displayed_cols.join(" ");
        if display_text.is_empty() {
            format!(
                "<no column {} found>",
                nth_cols
                    .iter()
                    .map(|index| index.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            )
        } else {
            display_text
        }
    } else if delimiter == "\t" && columns.len() > 1 {
        if columns[0].parse::<u64>().is_ok() {
            format!("{:<6} {}", columns[0], columns[1..].join(" "))
        } else {
            columns.join("  ")
        }
    } else {
        original_line.replace('\t', "  ")
    }
}
