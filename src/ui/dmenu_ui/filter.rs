use super::DmenuUI;

impl<'a> DmenuUI<'a> {
    /// Updates shown and hidden items with matching (fuzzy or exact).
    pub fn filter(&mut self) {
        let query_is_empty = self.query.is_empty();

        let mut index = 0;
        while index < self.shown.len() {
            let score = if query_is_empty {
                Some(0)
            } else if let Some(ref match_cols) = self.match_nth {
                self.shown[index].calculate_score_with_match_nth(
                    &self.query,
                    &mut self.matcher,
                    match_cols,
                )
            } else {
                match self.match_mode {
                    crate::cli::MatchMode::Exact => {
                        self.shown[index].calculate_exact_score(&self.query)
                    }
                    crate::cli::MatchMode::Fuzzy => {
                        self.shown[index].calculate_score(&self.query, &mut self.matcher)
                    }
                }
            };

            match score {
                None => {
                    self.shown[index].set_score(0);
                    let item = self.shown.swap_remove(index);
                    self.hidden.push(item);
                }
                Some(score) => {
                    self.shown[index].set_score(score);
                    index += 1;
                }
            }
        }

        let mut index = 0;
        while index < self.hidden.len() {
            let score = if query_is_empty {
                Some(0)
            } else if let Some(ref match_cols) = self.match_nth {
                self.hidden[index].calculate_score_with_match_nth(
                    &self.query,
                    &mut self.matcher,
                    match_cols,
                )
            } else {
                match self.match_mode {
                    crate::cli::MatchMode::Exact => {
                        self.hidden[index].calculate_exact_score(&self.query)
                    }
                    crate::cli::MatchMode::Fuzzy => {
                        self.hidden[index].calculate_score(&self.query, &mut self.matcher)
                    }
                }
            };

            if let Some(score) = score {
                self.hidden[index].set_score(score);
                let item = self.hidden.swap_remove(index);
                self.shown.push(item);
            } else {
                index += 1;
            }
        }

        self.shown.sort();

        if self.shown.is_empty() {
            self.selected = None;
            self.scroll_offset = 0;
        } else {
            self.selected = Some(0);
            self.scroll_offset = 0;
        }
    }
}
