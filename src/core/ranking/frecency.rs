use std::collections::HashMap;
use std::time::SystemTime;

/// Returns the current Unix timestamp in seconds.
pub fn current_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

/// Frecency data for a single item.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FrecencyEntry {
    /// Accumulated access count.
    pub score: u64,
    /// Last access time (Unix timestamp).
    pub last_access: u64,
}

impl Default for FrecencyEntry {
    fn default() -> Self {
        Self {
            score: 1,
            last_access: current_unix_seconds(),
        }
    }
}

impl FrecencyEntry {
    /// Records an access and updates the timestamp to now.
    pub fn access(&mut self) {
        self.access_at(current_unix_seconds());
    }

    /// Records an access at a specific timestamp.
    pub fn access_at(&mut self, now_secs: u64) {
        self.score += 1;
        self.last_access = now_secs;
    }

    /// Calculates the frecency score using the current time.
    pub fn frecency(&self) -> f64 {
        self.frecency_at(current_unix_seconds())
    }

    /// Calculates the frecency score at a specific timestamp.
    ///
    /// - Within 1 hour: score * 4
    /// - Within 1 day: score * 2
    /// - Within 1 week: score / 2
    /// - Older: score / 4
    pub fn frecency_at(&self, now_secs: u64) -> f64 {
        let age_secs = now_secs.saturating_sub(self.last_access);
        let score = self.score as f64;

        const HOUR: u64 = 3600;
        const DAY: u64 = 86400;
        const WEEK: u64 = 604800;

        if age_secs < HOUR {
            score * 4.0
        } else if age_secs < DAY {
            score * 2.0
        } else if age_secs < WEEK {
            score * 0.5
        } else {
            score * 0.25
        }
    }

    /// Ages the entry by dividing the score by `factor`.
    pub fn age(&mut self, factor: u64) {
        self.score /= factor;
    }
}

/// Ages all entries when the total score exceeds `max_age`.
pub fn age_entries(entries: &mut HashMap<String, FrecencyEntry>, max_age: u64) {
    let total: u64 = entries.values().map(|entry| entry.score).sum();

    if total > max_age {
        let target = (max_age as f64 * 0.9) as u64;
        let factor = (total / target).max(2);

        entries.values_mut().for_each(|entry| entry.age(factor));
        entries.retain(|_, entry| entry.score >= 1);
    }
}

#[cfg(test)]
mod tests {
    use super::{FrecencyEntry, age_entries};
    use std::collections::HashMap;

    #[test]
    fn frecency_at_uses_expected_time_buckets() {
        let entry = FrecencyEntry {
            score: 4,
            last_access: 1_000,
        };

        assert_eq!(entry.frecency_at(1_100), 16.0);
        assert_eq!(entry.frecency_at(5_000), 8.0);
        assert_eq!(entry.frecency_at(90_000), 2.0);
        assert_eq!(entry.frecency_at(900_000), 1.0);
    }

    #[test]
    fn age_entries_scales_scores_and_drops_empty_entries() {
        let mut entries = HashMap::from([
            (
                "alpha".to_string(),
                FrecencyEntry {
                    score: 10,
                    last_access: 100,
                },
            ),
            (
                "beta".to_string(),
                FrecencyEntry {
                    score: 1,
                    last_access: 100,
                },
            ),
        ]);

        age_entries(&mut entries, 5);

        assert_eq!(entries["alpha"].score, 5);
        assert!(!entries.contains_key("beta"));
    }
}
