//! Cell-width wrapping for pager rows, preserving text and grapheme boundaries.

use ratatui::style::Style;
use ratatui::text::Line;
use unicode_width::UnicodeWidthStr;

pub(super) fn display_rows(content: &str, width: u16, wrap: bool) -> Vec<&str> {
    let mut rows = Vec::new();
    for source in content.split('\n') {
        if wrap {
            wrap_line(source, usize::from(width).max(1), &mut rows);
        } else {
            rows.push(source);
        }
    }
    rows
}

fn wrap_line<'a>(source: &'a str, width: usize, rows: &mut Vec<&'a str>) {
    let line = Line::from(source);
    let mut start = 0;
    let mut offset = 0;
    let mut cells = 0;
    for grapheme in line.styled_graphemes(Style::default()) {
        let grapheme_width = grapheme.symbol.width();
        if cells + grapheme_width > width && offset > start {
            rows.push(&source[start..offset]);
            start = offset;
            cells = 0;
        }
        cells += grapheme_width;
        offset += grapheme.symbol.len();
    }
    rows.push(&source[start..]);
}

#[cfg(test)]
mod tests {
    use super::display_rows;

    #[test]
    fn wrapping_preserves_indentation_blank_lines_and_exact_width_rows() {
        assert_eq!(
            display_rows("    abcdef\n\n1234\n", 4, true),
            ["    ", "abcd", "ef", "", "1234", ""]
        );
    }

    #[test]
    fn wrapping_keeps_wide_and_combining_graphemes_together() {
        assert_eq!(
            display_rows("界界e\u{301}x", 3, true),
            ["界", "界e\u{301}", "x"]
        );
        assert_eq!(display_rows("👩‍💻ab", 2, true), ["👩‍💻", "ab"]);
    }

    #[test]
    fn narrow_or_zero_width_never_loses_text() {
        for width in [0, 1, 2] {
            let rows = display_rows("界ab", width, true);
            assert_eq!(rows.concat(), "界ab");
            assert!(rows.iter().all(|row| !row.is_empty()));
        }
    }

    #[test]
    fn unwrapped_rows_preserve_long_lines() {
        assert_eq!(display_rows("abcdef\n  ghi", 2, false), ["abcdef", "  ghi"]);
    }

    #[test]
    fn wrapping_does_not_limit_large_content_to_u16_rows() {
        let content = "x".repeat(70_000);
        assert_eq!(display_rows(&content, 1, true).len(), 70_000);
    }
}
