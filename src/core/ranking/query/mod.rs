mod bucket;
mod matcher;

use super::FrecencyEntry;
use super::sort::{compare_names, compare_pinned_order, ranking_boost, ranking_score};
use crate::cli::{PinnedOrderMode, RankingMode};
use crate::desktop::App;
use bucket::query_bucket;
use matcher::{QueryContext, base_fuzzy_score};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Detailed score breakdown for debug output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ScoreBreakdown {
    /// Human-readable tier name for the winning match bucket.
    pub tier: String,
    /// Coarse bucket score before fuzzy and frecency boosts.
    pub bucket_score: i64,
    /// Additional score derived from fuzzy matching.
    pub matcher_score: i64,
    /// Additional score derived from the selected ranking mode.
    pub frecency_boost: i64,
    /// Raw frecency score expressed in milli-units for debug output.
    pub raw_frecency_milli: i64,
    /// Name of the ranking mode that produced the frecency boost.
    #[serde(default = "default_ranking_mode_label")]
    pub ranking_mode: String,
}

fn default_ranking_mode_label() -> String {
    "frecency".to_string()
}

/// Inputs required to rank apps for a search query.
#[derive(Clone, Copy)]
pub struct FilterOptions<'a> {
    /// Query text entered by the user.
    pub query: &'a str,
    /// Frecency metadata keyed by app name.
    pub frecency_data: &'a HashMap<String, FrecencyEntry>,
    /// Prefix depth that still enables word-start tiering.
    pub prefix_depth: usize,
    /// Ranking mode used for frecency boosts.
    pub ranking_mode: RankingMode,
    /// Pinned-app tie-break policy.
    pub pinned_order_mode: PinnedOrderMode,
    /// First-pin timestamps for deterministic pinned ordering.
    pub pin_timestamps: &'a HashMap<String, u64>,
    /// Current Unix timestamp in seconds.
    pub now_secs: u64,
}

/// Filters applications for `options.query` and returns them in ranked order.
pub fn filter_apps(apps: &[App], options: FilterOptions<'_>) -> Vec<App> {
    if options.query.is_empty() {
        return apps.to_vec();
    }

    let mut context = QueryContext::new(options);
    let mut scored: Vec<(i64, App)> = apps
        .iter()
        .filter_map(|app| score_app_for_query(app, &mut context))
        .collect();

    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| {
                compare_pinned_order(
                    &a.1,
                    &b.1,
                    context.options.pinned_order_mode,
                    context.options.pin_timestamps,
                )
            })
            .then_with(|| compare_names(&a.1.name, &b.1.name))
    });

    scored.into_iter().map(|(_, app)| app).collect()
}

fn score_app_for_query(app: &App, context: &mut QueryContext<'_>) -> Option<(i64, App)> {
    let exec_name = crate::strings::extract_exec_name(&app.command);
    let matcher_score = base_fuzzy_score(app, exec_name, context);
    let bucket = query_bucket(
        app,
        exec_name,
        &context.query_lower,
        matcher_score,
        context.options.prefix_depth,
    )?;

    let frecency_score = ranking_score(
        context.options.frecency_data.get(&app.name),
        context.options.ranking_mode,
        context.options.now_secs,
    );
    let frecency_boost = ranking_boost(frecency_score, context.options.ranking_mode);
    let matcher_boost = matcher_score * 100;
    let final_score = bucket.score() + matcher_boost + frecency_boost;

    let mut ranked_app = app.clone();
    ranked_app.score = final_score;
    ranked_app.breakdown = Some(ScoreBreakdown {
        tier: bucket.label().to_string(),
        bucket_score: bucket.score(),
        matcher_score: matcher_boost,
        frecency_boost,
        raw_frecency_milli: (frecency_score * 1000.0) as i64,
        ranking_mode: context.options.ranking_mode.as_str().to_string(),
    });

    Some((final_score, ranked_app))
}

#[cfg(test)]
mod tests {
    use super::{FilterOptions, filter_apps};
    use crate::cli::{PinnedOrderMode, RankingMode};
    use crate::desktop::App;
    use std::collections::HashMap;

    fn test_app(
        name: &str,
        exec: &str,
        generic_name: Option<&str>,
        keywords: &[&str],
        categories: &[&str],
    ) -> App {
        let mut contents = vec![
            "[Desktop Entry]".to_string(),
            "Type=Application".to_string(),
            format!("Name={name}"),
            format!("Exec={exec}"),
            format!("Comment={name} description"),
        ];

        if let Some(generic_name) = generic_name {
            contents.push(format!("GenericName={generic_name}"));
        }
        if !keywords.is_empty() {
            contents.push(format!("Keywords={};", keywords.join(";")));
        }
        if !categories.is_empty() {
            contents.push(format!("Categories={};", categories.join(";")));
        }

        App::parse(contents.join("\n"), false).expect("test desktop entry should parse")
    }

    #[test]
    fn filter_apps_prefers_exact_matches_and_populates_breakdown() {
        let apps = vec![
            test_app("Alpha", "/usr/bin/alpha", None, &[], &[]),
            test_app("Alpha Beta", "/usr/bin/alpha-beta", None, &[], &[]),
        ];

        let ranked = filter_apps(
            &apps,
            FilterOptions {
                query: "alpha",
                frecency_data: &HashMap::new(),
                prefix_depth: 5,
                ranking_mode: RankingMode::Frecency,
                pinned_order_mode: PinnedOrderMode::Ranking,
                pin_timestamps: &HashMap::new(),
                now_secs: 10_000,
            },
        );

        assert_eq!(ranked[0].name, "Alpha");
        assert_eq!(ranked[1].name, "Alpha Beta");
        assert_eq!(
            ranked[0]
                .breakdown
                .as_ref()
                .map(|breakdown| breakdown.tier.as_str()),
            Some("Normal App Name Exact")
        );
    }

    #[test]
    fn filter_apps_matches_metadata_within_prefix_depth() {
        let apps = vec![test_app(
            "Archive Tool",
            "/usr/bin/archive-tool",
            Some("Extractor"),
            &["compress", "zip"],
            &["Utility"],
        )];

        let ranked = filter_apps(
            &apps,
            FilterOptions {
                query: "zip",
                frecency_data: &HashMap::new(),
                prefix_depth: 5,
                ranking_mode: RankingMode::Frecency,
                pinned_order_mode: PinnedOrderMode::Ranking,
                pin_timestamps: &HashMap::new(),
                now_secs: 10_000,
            },
        );

        assert_eq!(ranked.len(), 1);
        assert_eq!(
            ranked[0]
                .breakdown
                .as_ref()
                .map(|breakdown| breakdown.tier.as_str()),
            Some("Normal Metadata Match")
        );
    }
}
