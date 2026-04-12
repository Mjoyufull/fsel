use crate::desktop::App;

#[derive(Copy, Clone)]
pub(super) enum QueryBucket {
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
    pub(super) const fn score(self) -> i64 {
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

    pub(super) const fn label(self) -> &'static str {
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

pub(super) fn query_bucket(
    app: &App,
    exec_name: &str,
    query_lower: &str,
    matcher_score: i64,
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
        } else if matcher_score > 0 {
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
    } else if matcher_score > 0 {
        QueryBucket::FuzzyMatch
    } else {
        return None;
    };

    Some(bucket)
}

fn matches_word_start(haystack: &str, query_lower: &str) -> bool {
    haystack.match_indices(query_lower).any(|(index, _)| {
        index == 0
            || haystack[..index]
                .chars()
                .next_back()
                .is_some_and(|ch| !ch.is_alphanumeric())
    })
}
