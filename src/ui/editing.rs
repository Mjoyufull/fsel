//! Query editing and list paging helpers shared by every interactive mode.

/// Remove the last word from `text`, readline `unix-word-rubout` style:
/// trailing whitespace first, then everything back to the previous whitespace.
pub(crate) fn delete_word(text: &mut String) {
    let trimmed = text.trim_end().len();
    let start = text[..trimmed]
        .char_indices()
        .rev()
        .find(|(_, character)| character.is_whitespace())
        .map_or(0, |(index, character)| index + character.len_utf8());
    text.truncate(start);
}

/// Half of `capacity`, rounded down to whole rows of `row_len` items, never below one row.
pub(crate) fn half_page_step(capacity: usize, row_len: usize) -> usize {
    let items_per_row = row_len.max(1);
    (capacity / items_per_row / 2).max(1) * items_per_row
}

/// Index `step` items away from `selected`, clamped to the list instead of wrapping.
pub(crate) fn half_page_target(selected: usize, len: usize, step: usize, forward: bool) -> usize {
    if len == 0 {
        return selected;
    }
    if forward {
        selected.saturating_add(step).min(len - 1)
    } else {
        selected.saturating_sub(step)
    }
}

#[cfg(test)]
mod tests {
    use super::{delete_word, half_page_step, half_page_target};

    fn deleted(text: &str) -> String {
        let mut text = text.to_string();
        delete_word(&mut text);
        text
    }

    #[test]
    fn delete_word_removes_last_word_and_trailing_space() {
        assert_eq!(deleted("fire fox"), "fire ");
        assert_eq!(deleted("fire fox  "), "fire ");
        assert_eq!(deleted("firefox"), "");
        assert_eq!(deleted("   "), "");
        assert_eq!(deleted(""), "");
    }

    #[test]
    fn delete_word_handles_multibyte_characters() {
        assert_eq!(deleted("café 日本語"), "café ");
        assert_eq!(deleted("a\u{3000}b"), "a\u{3000}");
    }

    #[test]
    fn half_page_step_keeps_grid_columns_aligned() {
        assert_eq!(half_page_step(20, 1), 10);
        assert_eq!(half_page_step(1, 1), 1);
        assert_eq!(half_page_step(0, 1), 1);
        assert_eq!(half_page_step(12, 4), 4);
        assert_eq!(half_page_step(24, 4), 12);
    }

    #[test]
    fn half_page_target_clamps_at_list_ends() {
        assert_eq!(half_page_target(2, 10, 5, true), 7);
        assert_eq!(half_page_target(7, 10, 5, true), 9);
        assert_eq!(half_page_target(7, 10, 5, false), 2);
        assert_eq!(half_page_target(2, 10, 5, false), 0);
        assert_eq!(half_page_target(0, 0, 5, true), 0);
    }
}
