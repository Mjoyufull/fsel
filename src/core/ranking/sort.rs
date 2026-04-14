use super::FrecencyEntry;
use crate::cli::{PinnedOrderMode, RankingMode};
use crate::desktop::App;
use std::cmp::Ordering;
use std::collections::HashMap;

pub(super) fn compare_names(a: &str, b: &str) -> Ordering {
    a.to_lowercase().cmp(&b.to_lowercase()).then(a.cmp(b))
}

pub(super) fn compare_pinned_order(
    a: &App,
    b: &App,
    pinned_order_mode: PinnedOrderMode,
    pin_timestamps: &HashMap<String, u64>,
) -> Ordering {
    if !(a.pinned && b.pinned) {
        return Ordering::Equal;
    }

    match pinned_order_mode {
        PinnedOrderMode::Ranking => Ordering::Equal,
        PinnedOrderMode::Alphabetical => compare_names(&a.name, &b.name),
        PinnedOrderMode::OldestPinned => {
            let by_time = match (
                pin_timestamps.get(&a.name).copied(),
                pin_timestamps.get(&b.name).copied(),
            ) {
                (Some(a_ts), Some(b_ts)) => a_ts.cmp(&b_ts),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            };
            by_time.then_with(|| compare_names(&a.name, &b.name))
        }
        PinnedOrderMode::NewestPinned => {
            let by_time = match (
                pin_timestamps.get(&a.name).copied(),
                pin_timestamps.get(&b.name).copied(),
            ) {
                (Some(a_ts), Some(b_ts)) => b_ts.cmp(&a_ts),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            };
            by_time.then_with(|| compare_names(&a.name, &b.name))
        }
    }
}

fn recency_score(last_access: u64, now_secs: u64) -> f64 {
    let age_secs = now_secs.saturating_sub(last_access);
    let age_hours = age_secs as f64 / 3600.0;
    1.0 / (age_hours + 1.0)
}

pub(super) fn ranking_score(
    entry: Option<&FrecencyEntry>,
    ranking_mode: RankingMode,
    now_secs: u64,
) -> f64 {
    let Some(entry) = entry else {
        return 0.0;
    };

    match ranking_mode {
        RankingMode::Frecency => entry.frecency_at(now_secs),
        RankingMode::Frequency => entry.score as f64,
        RankingMode::Recency => recency_score(entry.last_access, now_secs),
    }
}

pub(super) fn ranking_boost(score: f64, ranking_mode: RankingMode) -> i64 {
    match ranking_mode {
        RankingMode::Frecency => (score * 10.0) as i64,
        RankingMode::Frequency => (score * 10.0) as i64,
        RankingMode::Recency => (score * 10_000.0) as i64,
    }
}

/// Sorts applications by the configured ranking mode and pinned policy.
pub fn sort_by_ranking(
    apps: &mut [App],
    frecency_data: &HashMap<String, FrecencyEntry>,
    ranking_mode: RankingMode,
    pinned_order_mode: PinnedOrderMode,
    pin_timestamps: &HashMap<String, u64>,
    now_secs: u64,
) {
    for app in apps.iter_mut() {
        if let Some(entry) = frecency_data.get(&app.name) {
            app.last_access = Some(entry.last_access);
        }
    }

    apps.sort_by(|a, b| {
        match (a.pinned, b.pinned) {
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            _ => {}
        }

        let pinned_order = compare_pinned_order(a, b, pinned_order_mode, pin_timestamps);
        if pinned_order != Ordering::Equal {
            return pinned_order;
        }

        let a_entry = frecency_data.get(&a.name);
        let b_entry = frecency_data.get(&b.name);

        let ranking_cmp = match ranking_mode {
            RankingMode::Frecency => {
                let a_score = ranking_score(a_entry, ranking_mode, now_secs);
                let b_score = ranking_score(b_entry, ranking_mode, now_secs);
                b_score.partial_cmp(&a_score).unwrap_or(Ordering::Equal)
            }
            RankingMode::Frequency => {
                let a_score = a_entry.map(|entry| entry.score).unwrap_or(0);
                let b_score = b_entry.map(|entry| entry.score).unwrap_or(0);
                b_score.cmp(&a_score)
            }
            RankingMode::Recency => {
                let a_score = a_entry.map(|entry| entry.last_access).unwrap_or(0);
                let b_score = b_entry.map(|entry| entry.last_access).unwrap_or(0);
                b_score.cmp(&a_score)
            }
        };

        ranking_cmp.then_with(|| compare_names(&a.name, &b.name))
    });
}

#[cfg(test)]
mod tests {
    use super::sort_by_ranking;
    use crate::cli::{PinnedOrderMode, RankingMode};
    use crate::core::ranking::FrecencyEntry;
    use crate::desktop::App;
    use std::collections::HashMap;

    fn test_app(name: &str, exec: &str) -> App {
        App::parse(
            format!(
                "[Desktop Entry]\nType=Application\nName={name}\nExec={exec}\nComment={name} description"
            ),
            false,
        )
        .expect("test desktop entry should parse")
    }

    #[test]
    fn sort_by_ranking_respects_oldest_pinned_order() {
        let mut alpha = test_app("Alpha", "/usr/bin/alpha");
        let mut beta = test_app("Beta", "/usr/bin/beta");
        alpha.pinned = true;
        beta.pinned = true;

        let mut apps = vec![beta, alpha];
        let pin_timestamps = HashMap::from([("Alpha".to_string(), 100), ("Beta".to_string(), 200)]);

        sort_by_ranking(
            &mut apps,
            &HashMap::new(),
            RankingMode::Frecency,
            PinnedOrderMode::OldestPinned,
            &pin_timestamps,
            10_000,
        );

        assert_eq!(apps[0].name, "Alpha");
        assert_eq!(apps[1].name, "Beta");
    }

    #[test]
    fn sort_by_ranking_uses_frequency_mode_and_sets_last_access() {
        let mut apps = vec![
            test_app("Alpha", "/usr/bin/alpha"),
            test_app("Beta", "/usr/bin/beta"),
        ];
        let frecency = HashMap::from([
            (
                "Alpha".to_string(),
                FrecencyEntry {
                    score: 2,
                    last_access: 100,
                },
            ),
            (
                "Beta".to_string(),
                FrecencyEntry {
                    score: 5,
                    last_access: 250,
                },
            ),
        ]);

        sort_by_ranking(
            &mut apps,
            &frecency,
            RankingMode::Frequency,
            PinnedOrderMode::Ranking,
            &HashMap::new(),
            10_000,
        );

        assert_eq!(apps[0].name, "Beta");
        assert_eq!(apps[0].last_access, Some(250));
        assert_eq!(apps[1].last_access, Some(100));
    }
}
