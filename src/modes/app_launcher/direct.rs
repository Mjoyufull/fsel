use crate::cli;
use crate::core::{cache, database};
use crate::desktop;
use crate::strings;
use eyre::{Result, eyre};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use std::io::{self, Write};

/// Launches a program directly by name, bypassing the TUI.
pub(crate) fn launch_program_directly(cli: &cli::Opts, program_name: &str) -> Result<()> {
    let (db, _) = database::open_history_db()?;
    let history_cache = cache::HistoryCache::load(&db)?;

    if let Some(app) = find_history_exact_name_match(&db, &history_cache, program_name, cli)? {
        return launch_or_print(cli, &db, &app);
    }

    if matches!(cli.match_mode, cli::MatchMode::Fuzzy)
        && let Some(app) = find_history_best_match(&db, &history_cache, program_name, cli)?
    {
        return launch_or_print(cli, &db, &app);
    }

    let all_apps = load_available_apps(&db, cli);
    let app_to_run =
        select_match_for_mode(all_apps, program_name, cli.match_mode).ok_or_else(|| {
            if matches!(cli.match_mode, cli::MatchMode::Exact) {
                eyre!("No exact application match found for '{}'", program_name)
            } else {
                eyre!("No matching application found for '{}'", program_name)
            }
        })?;

    if cli.confirm_first_launch
        && app_to_run.history == 0
        && !confirm_first_launch(&app_to_run.name)?
    {
        eprintln!(
            "Cancelled. Use 'fsel -ss {}' to search in TUI.",
            program_name
        );
        return Ok(());
    }

    if cli.verbose.unwrap_or(0) > 0 {
        eprintln!("Launching: {} ({})", app_to_run.name, app_to_run.command);
    }

    launch_or_print(cli, &db, &app_to_run)
}

fn launch_or_print(
    cli: &cli::Opts,
    db: &std::sync::Arc<redb::Database>,
    app: &desktop::App,
) -> Result<()> {
    if cli.no_exec {
        println!("{}", app.command);
        return Ok(());
    }

    super::launch::launch_app(app, cli, db)
}

fn find_history_exact_name_match(
    db: &std::sync::Arc<redb::Database>,
    history_cache: &cache::HistoryCache,
    program_name: &str,
    cli: &cli::Opts,
) -> Result<Option<desktop::App>> {
    let program_name_lower = program_name.to_lowercase();

    for app_name in history_cache.history.keys() {
        if app_name.to_lowercase() == program_name_lower
            && let Some(app) = super::search::find_app_by_name_fast(db, app_name, cli)?
        {
            return Ok(Some(app));
        }
    }

    Ok(None)
}

fn find_history_best_match(
    db: &std::sync::Arc<redb::Database>,
    history_cache: &cache::HistoryCache,
    program_name: &str,
    cli: &cli::Opts,
) -> Result<Option<desktop::App>> {
    if let Some((app_name, _)) = history_cache.get_best_match(program_name)
        && let Some(app) = super::search::find_app_by_name_fast(db, app_name, cli)?
    {
        return Ok(Some(app));
    }

    Ok(None)
}

fn load_available_apps(db: &std::sync::Arc<redb::Database>, cli: &cli::Opts) -> Vec<desktop::App> {
    let apps_receiver = desktop::read_with_options(
        desktop::application_dirs(),
        db,
        desktop::DiscoverOptions {
            filter_desktop: cli.filter_desktop,
            filter_actions: cli.filter_actions,
            list_executables: cli.list_executables_in_path,
        },
    );

    let mut all_apps = Vec::new();
    while let Ok(app) = apps_receiver.recv() {
        all_apps.push(app);
    }
    all_apps
}

fn confirm_first_launch(app_name: &str) -> Result<bool> {
    eprint!("Launch {} [Y/n]? ", app_name);
    io::stderr().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;
    let response = response.trim().to_lowercase();
    Ok(response != "n" && response != "no")
}

fn select_match_for_mode(
    apps: Vec<desktop::App>,
    program_name: &str,
    match_mode: cli::MatchMode,
) -> Option<desktop::App> {
    match match_mode {
        cli::MatchMode::Exact => select_exact_match(apps, program_name),
        cli::MatchMode::Fuzzy => select_best_match(apps, program_name),
    }
}

fn select_best_match(apps: Vec<desktop::App>, program_name: &str) -> Option<desktop::App> {
    let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
    let program_name_lower = program_name.to_lowercase();
    let mut best_app: Option<(desktop::App, i64)> = None;

    for app in apps {
        let Some(score) = score_candidate(&app, program_name, &program_name_lower, &mut matcher)
        else {
            continue;
        };
        match &best_app {
            Some((_, current_best_score)) if score <= *current_best_score => {}
            _ => best_app = Some((app, score)),
        }
    }

    best_app.map(|(app, _)| app)
}

fn select_exact_match(apps: Vec<desktop::App>, program_name: &str) -> Option<desktop::App> {
    let program_name_lower = program_name.to_lowercase();
    let mut best_app: Option<(desktop::App, i64)> = None;

    for app in apps {
        let Some(score) = score_exact_candidate(&app, &program_name_lower) else {
            continue;
        };
        match &best_app {
            Some((_, current_best_score)) if score <= *current_best_score => {}
            _ => best_app = Some((app, score)),
        }
    }

    best_app.map(|(app, _)| app)
}

fn score_candidate(
    app: &desktop::App,
    program_name: &str,
    program_name_lower: &str,
    matcher: &mut Matcher,
) -> Option<i64> {
    let app_name_lower = app.name.to_lowercase();
    let exec_name = strings::extract_exec_name(&app.command);
    let exec_name_lower = exec_name.to_lowercase();

    let final_score = if app_name_lower == program_name_lower {
        1_000_000
    } else if exec_name_lower == program_name_lower {
        900_000
    } else if exec_name_lower.starts_with(program_name_lower) {
        800_000
    } else if app_name_lower.starts_with(program_name_lower) {
        700_000
    } else {
        let mut program_chars = Vec::new();
        let program_utf32 = Utf32Str::new(program_name, &mut program_chars);
        let mut name_chars = Vec::new();
        let name_utf32 = Utf32Str::new(app.name.as_str(), &mut name_chars);
        let name_score = matcher.fuzzy_match(name_utf32, program_utf32).unwrap_or(0) as i64;
        let mut exec_chars = Vec::new();
        let exec_utf32 = Utf32Str::new(exec_name, &mut exec_chars);
        let exec_score = matcher.fuzzy_match(exec_utf32, program_utf32).unwrap_or(0) as i64;
        let best_score = std::cmp::max(name_score, exec_score * 2);

        if best_score == 0 {
            return None;
        }

        best_score
    };

    Some(apply_rank_boosts(final_score, app))
}

fn score_exact_candidate(app: &desktop::App, program_name_lower: &str) -> Option<i64> {
    let app_name_lower = app.name.to_lowercase();
    let exec_name = strings::extract_exec_name(&app.command);
    let exec_name_lower = exec_name.to_lowercase();

    let final_score = if app_name_lower == program_name_lower {
        1_000_000
    } else if exec_name_lower == program_name_lower {
        900_000
    } else {
        return None;
    };

    Some(apply_rank_boosts(final_score, app))
}

fn apply_rank_boosts(mut final_score: i64, app: &desktop::App) -> i64 {
    if app.pinned {
        if final_score < 700_000 {
            final_score = final_score.saturating_add(500_000);
        } else {
            final_score = final_score.saturating_add(50_000);
        }
    }

    if app.history > 0 {
        let history = i64::try_from(app.history).unwrap_or(i64::MAX);
        final_score = if final_score >= 700_000 {
            final_score.saturating_add(history)
        } else {
            final_score.saturating_mul(history)
        };
    }

    final_score
}

#[cfg(test)]
mod tests {
    use super::{score_candidate, select_match_for_mode};
    use crate::cli::MatchMode;
    use crate::desktop::App;
    use nucleo_matcher::{Config, Matcher};

    fn app(name: &str, command: &str) -> App {
        App::parse(
            format!(
                "[Desktop Entry]\nType=Application\nName={name}\nExec={command}\nComment=Test app"
            ),
            false,
        )
        .expect("test desktop entry should parse")
    }

    #[test]
    fn exact_name_match_beats_other_candidates() {
        let selected = select_match_for_mode(
            vec![app("Foot Terminal", "foot"), app("Firefox", "firefox")],
            "Firefox",
            MatchMode::Fuzzy,
        )
        .expect("a match should be selected");

        assert_eq!(selected.name, "Firefox");
    }

    #[test]
    fn executable_prefix_match_scores_above_fuzzy_name_match() {
        let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
        let exec_prefix = score_candidate(&app("Console", "fx-run"), "fx", "fx", &mut matcher)
            .expect("prefix candidate should score");
        let fuzzy = score_candidate(&app("Firefox", "browser"), "fx", "fx", &mut matcher)
            .expect("fuzzy candidate should score");

        assert!(exec_prefix > fuzzy);
    }

    #[test]
    fn exact_mode_accepts_exact_executable_name() {
        let selected = select_match_for_mode(
            vec![app("Steam Store", "steam steam://store")],
            "steam",
            MatchMode::Exact,
        )
        .expect("exact executable match should be selected");

        assert_eq!(selected.name, "Steam Store");
    }

    #[test]
    fn exact_mode_rejects_prefix_only_matches() {
        let selected = select_match_for_mode(
            vec![app("Steam Store", "steam steam://store")],
            "test",
            MatchMode::Exact,
        );

        assert!(selected.is_none());
    }

    #[test]
    fn fuzzy_mode_keeps_best_effort_matching_for_program_launch() {
        let selected = select_match_for_mode(
            vec![app("Steam Store", "steam steam://store")],
            "test",
            MatchMode::Fuzzy,
        )
        .expect("fuzzy mode should still return the best match");

        assert_eq!(selected.name, "Steam Store");
    }
}
