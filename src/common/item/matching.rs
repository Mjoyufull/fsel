use super::Item;
use nucleo_matcher::{Matcher, Utf32Str};

impl Item {
    /// Calculate fuzzy match score against query.
    #[inline]
    pub fn calculate_score(&self, query: &str, matcher: &mut Matcher) -> Option<i64> {
        if query.is_empty() {
            return Some(0);
        }

        let query_lower = query.to_lowercase();
        let mut query_chars = Vec::new();
        let query_utf32 = Utf32Str::new(&query_lower, &mut query_chars);

        if let Ok(cclip_item) =
            crate::modes::cclip::CclipItem::from_line(self.original_line.clone())
        {
            for tag in &cclip_item.tags {
                let tag_lower = tag.to_lowercase();
                if tag_lower == query_lower {
                    return Some(1_000_000);
                }
                if tag_lower.starts_with(&query_lower) {
                    return Some(800_000);
                }

                let mut tag_chars = Vec::new();
                let tag_utf32 = Utf32Str::new(&tag_lower, &mut tag_chars);
                if let Some(score) = matcher.fuzzy_match(tag_utf32, query_utf32) {
                    return Some((score as i64) * 10);
                }
            }
        }

        let display_lower = self.display_text.to_lowercase();
        let mut display_chars = Vec::new();
        let display_utf32 = Utf32Str::new(&display_lower, &mut display_chars);
        if let Some(score) = matcher.fuzzy_match(display_utf32, query_utf32) {
            return Some((score as i64) * 2);
        }

        let original_lower = self.original_line.to_lowercase();
        let mut original_chars = Vec::new();
        let original_utf32 = Utf32Str::new(&original_lower, &mut original_chars);
        matcher
            .fuzzy_match(original_utf32, query_utf32)
            .map(|score| score as i64)
    }

    /// Calculate exact match score against query.
    #[inline]
    pub fn calculate_exact_score(&self, query: &str) -> Option<i64> {
        if query.is_empty() {
            return Some(0);
        }

        let (is_quoted, search_query) = if query.len() >= 2
            && ((query.starts_with('"') && query.ends_with('"'))
                || (query.starts_with('\'') && query.ends_with('\'')))
        {
            (true, &query[1..query.len() - 1])
        } else {
            (false, query)
        };

        let query_lower = search_query.to_lowercase();
        let display_lower = self.display_text.to_lowercase();

        if is_quoted {
            return (display_lower == query_lower).then_some(1000);
        }
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

    /// Calculate match score based on `match_nth` columns.
    pub fn calculate_score_with_match_nth(
        &self,
        query: &str,
        matcher: &mut Matcher,
        match_nth: &[usize],
    ) -> Option<i64> {
        if query.is_empty() {
            return Some(0);
        }

        let query_lower = query.to_lowercase();
        let mut query_chars = Vec::new();
        let query_utf32 = Utf32Str::new(&query_lower, &mut query_chars);
        let mut best_score = None;

        for &column_index in match_nth {
            if column_index > 0 && column_index <= self.columns.len() {
                let col_text = &self.columns[column_index - 1];
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

    /// Get output based on `accept_nth` columns.
    pub fn get_accept_nth_output(&self, accept_nth: &[usize]) -> String {
        let accepted_cols: Vec<String> = accept_nth
            .iter()
            .filter_map(|&column_index| {
                if column_index > 0 && column_index <= self.columns.len() {
                    Some(self.columns[column_index - 1].clone())
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
}
