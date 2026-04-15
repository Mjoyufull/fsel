use super::FilterOptions;
use crate::desktop::App;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

pub(super) struct QueryContext<'a> {
    pub(super) query_lower: String,
    pattern: Pattern,
    matcher: Matcher,
    pub(super) options: FilterOptions<'a>,
}

impl<'a> QueryContext<'a> {
    pub(super) fn new(options: FilterOptions<'a>) -> Self {
        Self {
            query_lower: options.query.to_lowercase(),
            pattern: Pattern::parse(options.query, CaseMatching::Ignore, Normalization::Smart),
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            options,
        }
    }
}

pub(super) fn base_fuzzy_score(app: &App, exec_name: &str, context: &mut QueryContext<'_>) -> i64 {
    let name_score = fuzzy_score(&context.pattern, &mut context.matcher, &app.name);
    let exec_score = fuzzy_score(&context.pattern, &mut context.matcher, exec_name);

    let mut metadata_score = 0;
    for keyword in &app.keywords {
        metadata_score =
            metadata_score.max(fuzzy_score(&context.pattern, &mut context.matcher, keyword));
    }
    for category in &app.categories {
        metadata_score = metadata_score.max(fuzzy_score(
            &context.pattern,
            &mut context.matcher,
            category,
        ));
    }
    if let Some(generic_name) = &app.generic_name {
        metadata_score = metadata_score.max(fuzzy_score(
            &context.pattern,
            &mut context.matcher,
            generic_name,
        ));
    }

    name_score.max(exec_score * 2).max(metadata_score)
}

fn fuzzy_score(pattern: &Pattern, matcher: &mut Matcher, haystack: &str) -> i64 {
    let mut buffer = Vec::new();
    let haystack = Utf32Str::new(haystack, &mut buffer);
    pattern.score(haystack, matcher).unwrap_or(0) as i64
}
