use super::FrecencyEntry;
use super::sort::{compare_names, compare_pinned_order, ranking_boost, ranking_score};
use crate::cli::{PinnedOrderMode, RankingMode};
use crate::desktop::App;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
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

#[derive(Copy, Clone)]
enum QueryBucket {
    PinnedAppNameExact,
    PinnedExecNameExact,
    PinnedAppNamePrefix,
    PinnedExecNamePrefix,
    PinnedAppNameWordStart,
    PinnedExecNameWordStart,
    PinnedMetadataMatch,
    PinnedFuzzyMatch,
    AppNameExact,
    ExecNameExact,
    AppNamePrefix,
    ExecNamePrefix,
    AppNameWordStart,
    ExecNameWordStart,
    MetadataMatch,
    FuzzyMatch,
}

impl QueryBucket {
    const fn score(self) -> i64 {
        match self {
            Self::PinnedAppNameExact => 120_000_000,
            Self::PinnedExecNameExact => 115_000_000,
            Self::PinnedAppNamePrefix => 110_000_000,
            Self::PinnedExecNamePrefix => 105_000_000,
            Self::PinnedAppNameWordStart => 100_000_000,
            Self::PinnedExecNameWordStart => 95_000_000,
            Self::PinnedMetadataMatch => 40_000_000,
            Self::PinnedFuzzyMatch => 20_000_000,
            Self::AppNameExact => 90_000_000,
            Self::ExecNameExact => 85_000_000,
            Self::AppNamePrefix => 80_000_000,
            Self::ExecNamePrefix => 75_000_000,
            Self::AppNameWordStart => 70_000_000,
            Self::ExecNameWordStart => 65_000_000,
            Self::MetadataMatch => 30_000_000,
            Self::FuzzyMatch => 0,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::PinnedAppNameExact => "Pinned App Name Exact",
            Self::PinnedExecNameExact => "Pinned Exec Name Exact",
            Self::PinnedAppNamePrefix => "Pinned App Name Prefix",
            Self::PinnedExecNamePrefix => "Pinned Exec Name Prefix",
            Self::PinnedAppNameWordStart => "Pinned App Name Word-Start",
            Self::PinnedExecNameWordStart => "Pinned Exec Name Word-Start",
            Self::PinnedMetadataMatch => "Pinned Metadata Match",
            Self::PinnedFuzzyMatch => "Pinned Fuzzy Match",
            Self::AppNameExact => "Normal App Name Exact",
            Self::ExecNameExact => "Normal Exec Name Exact",
            Self::AppNamePrefix => "Normal App Name Prefix",
            Self::ExecNamePrefix => "Normal Exec Name Prefix",
            Self::AppNameWordStart => "Normal App Name Word-Start",
            Self::ExecNameWordStart => "Normal Exec Name Word-Start",
            Self::MetadataMatch => "Normal Metadata Match",
            Self::FuzzyMatch => "Normal Fuzzy Match",
        }
    }
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

struct QueryContext<'a> {
    query_lower: String,
    pattern: Pattern,
    matcher: Matcher,
    options: FilterOptions<'a>,
}

impl<'a> QueryContext<'a> {
    fn new(options: FilterOptions<'a>) -> Self {
        Self {
            query_lower: options.query.to_lowercase(),
            pattern: Pattern::parse(options.query, CaseMatching::Ignore, Normalization::Smart),
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            options,
        }
    }
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
    let base_fuzzy_score = base_fuzzy_score(app, exec_name, context);
    let bucket = query_bucket(
        app,
        exec_name,
        &context.query_lower,
        base_fuzzy_score,
        context.options.prefix_depth,
    )?;

    let rank_score = ranking_score(
        context.options.frecency_data.get(&app.name),
        context.options.ranking_mode,
        context.options.now_secs,
    );
    let rank_boost = ranking_boost(rank_score, context.options.ranking_mode);
    let matcher_boost = base_fuzzy_score * 100;
    let final_score = bucket.score() + matcher_boost + rank_boost;

    let mut ranked_app = app.clone();
    ranked_app.score = final_score;
    ranked_app.breakdown = Some(ScoreBreakdown {
        tier: bucket.label().to_string(),
        bucket_score: bucket.score(),
        matcher_score: matcher_boost,
        frecency_boost: rank_boost,
        raw_frecency_milli: (rank_score * 1000.0) as i64,
        ranking_mode: context.options.ranking_mode.as_str().to_string(),
    });

    Some((final_score, ranked_app))
}

fn base_fuzzy_score(app: &App, exec_name: &str, context: &mut QueryContext<'_>) -> i64 {
    let mut name_buf = Vec::new();
    let name_haystack = Utf32Str::new(&app.name, &mut name_buf);
    let name_score = context
        .pattern
        .score(name_haystack, &mut context.matcher)
        .unwrap_or(0) as i64;

    let mut exec_buf = Vec::new();
    let exec_haystack = Utf32Str::new(exec_name, &mut exec_buf);
    let exec_score = context
        .pattern
        .score(exec_haystack, &mut context.matcher)
        .unwrap_or(0) as i64;

    let mut meta_score = 0;
    let mut check_meta = |haystack: &str| {
        let mut buf = Vec::new();
        let haystack = Utf32Str::new(haystack, &mut buf);
        let score = context
            .pattern
            .score(haystack, &mut context.matcher)
            .unwrap_or(0) as i64;
        if score > meta_score {
            meta_score = score;
        }
    };

    for keyword in &app.keywords {
        check_meta(keyword);
    }
    for category in &app.categories {
        check_meta(category);
    }
    if let Some(generic_name) = &app.generic_name {
        check_meta(generic_name);
    }

    name_score.max(exec_score * 2).max(meta_score)
}

fn query_bucket(
    app: &App,
    exec_name: &str,
    query_lower: &str,
    base_fuzzy_score: i64,
    prefix_depth: usize,
) -> Option<QueryBucket> {
    let app_name_lower = app.name.to_lowercase();
    let exec_name_lower = exec_name.to_lowercase();
    let generic_name_lower = app.generic_name.as_ref().map(|value| value.to_lowercase());

    let name_exact = app_name_lower == query_lower;
    let exec_exact = exec_name_lower == query_lower;
    let name_prefix = app_name_lower.starts_with(query_lower);
    let exec_prefix = exec_name_lower.starts_with(query_lower);
    let within_depth = query_lower.len() <= prefix_depth;

    let name_word = matches_word_start(&app_name_lower, query_lower);
    let exec_word = matches_word_start(&exec_name_lower, query_lower);
    let meta_match = generic_name_lower
        .as_ref()
        .map(|value| matches_word_start(value, query_lower))
        .unwrap_or(false)
        || app
            .keywords
            .iter()
            .any(|keyword| matches_word_start(&keyword.to_lowercase(), query_lower))
        || app
            .categories
            .iter()
            .any(|category| matches_word_start(&category.to_lowercase(), query_lower));

    let bucket = if app.pinned {
        if name_exact {
            QueryBucket::PinnedAppNameExact
        } else if exec_exact {
            QueryBucket::PinnedExecNameExact
        } else if name_prefix {
            QueryBucket::PinnedAppNamePrefix
        } else if exec_prefix {
            QueryBucket::PinnedExecNamePrefix
        } else if within_depth && name_word {
            QueryBucket::PinnedAppNameWordStart
        } else if within_depth && exec_word {
            QueryBucket::PinnedExecNameWordStart
        } else if within_depth && meta_match {
            QueryBucket::PinnedMetadataMatch
        } else if base_fuzzy_score > 0 {
            QueryBucket::PinnedFuzzyMatch
        } else {
            return None;
        }
    } else if name_exact {
        QueryBucket::AppNameExact
    } else if exec_exact {
        QueryBucket::ExecNameExact
    } else if name_prefix {
        QueryBucket::AppNamePrefix
    } else if exec_prefix {
        QueryBucket::ExecNamePrefix
    } else if within_depth && name_word {
        QueryBucket::AppNameWordStart
    } else if within_depth && exec_word {
        QueryBucket::ExecNameWordStart
    } else if within_depth && meta_match {
        QueryBucket::MetadataMatch
    } else if base_fuzzy_score > 0 {
        QueryBucket::FuzzyMatch
    } else {
        return None;
    };

    Some(bucket)
}

fn matches_word_start(haystack: &str, query_lower: &str) -> bool {
    haystack.starts_with(query_lower) || haystack.contains(&format!(" {}", query_lower))
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
