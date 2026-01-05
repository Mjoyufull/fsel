use eyre::{eyre, Result};
use jwalk::WalkDir;
use nucleo_matcher::{Config, Matcher, Utf32Str};
use std::{env, fs, path};

use crate::cli;
use crate::core::{cache, database};
use crate::desktop;
use crate::strings;

/// Find an app by name using fast cache lookup
pub fn find_app_by_name_fast(
    db: &std::sync::Arc<redb::Database>,
    app_name: &str,
    cli: &cli::Opts,
) -> Result<Option<desktop::App>> {
    let desktop_cache = cache::DesktopCache::new(db.clone())?;
    let history_cache = cache::HistoryCache::load(db)?;

    // Try the name index first - this is instant if the app is cached
    if let Ok(Some(app)) = desktop_cache.get_by_name(app_name) {
        // Apply filtering if needed
        if cli.filter_desktop {
            if let Ok(current_desktop) = env::var("XDG_CURRENT_DESKTOP") {
                let desktops: Vec<String> =
                    current_desktop.split(':').map(|s| s.to_string()).collect();

                if !app.not_show_in.is_empty() {
                    let should_hide = app
                        .not_show_in
                        .iter()
                        .any(|d| desktops.iter().any(|cd| cd.eq_ignore_ascii_case(d)));
                    if should_hide {
                        return Ok(None);
                    }
                }

                if !app.only_show_in.is_empty() {
                    let should_show = app
                        .only_show_in
                        .iter()
                        .any(|d| desktops.iter().any(|cd| cd.eq_ignore_ascii_case(d)));
                    if !should_show {
                        return Ok(None);
                    }
                }
            }
        }

        // Found in cache! Apply history and return
        return Ok(Some(history_cache.apply_to_app(app)));
    }

    // Not in cache - need to search for it
    let mut dirs: Vec<path::PathBuf> = vec![];

    // Add user's data directory
    if let Some(xdg_data_home) = env::var("XDG_DATA_HOME").ok().filter(|s| !s.is_empty()) {
        let mut dir = path::PathBuf::from(xdg_data_home);
        dir.push("applications");
        if dir.exists() {
            dirs.push(dir);
        }
    } else if let Some(home_dir) = dirs::home_dir() {
        let mut dir = home_dir;
        dir.push(".local/share/applications");
        if dir.exists() {
            dirs.push(dir);
        }
    }

    // Add system data directories
    if let Ok(res) = env::var("XDG_DATA_DIRS") {
        for data_dir in res.split(':').filter(|s| !s.is_empty()) {
            let mut dir = path::PathBuf::from(data_dir);
            dir.push("applications");
            if dir.exists() {
                dirs.push(dir);
            }
        }
    } else {
        let mut default_paths = vec![
            path::PathBuf::from("/usr/local/share"),
            path::PathBuf::from("/usr/share"),
        ];

        #[cfg(target_os = "openbsd")]
        {
            default_paths.push(path::PathBuf::from("/usr/X11R6/share"));
        }

        for data_dir in &mut default_paths {
            data_dir.push("applications");
            if data_dir.exists() {
                dirs.push(data_dir.clone());
            }
        }
    }

    let desktop_cache = cache::DesktopCache::new(db.clone())?;
    let history_cache = cache::HistoryCache::load(db)?;

    // Search for the specific app
    for dir in &dirs {
        for entry in WalkDir::new(dir)
            .min_depth(1)
            .max_depth(5)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| {
                !entry.file_type().is_dir()
                    && entry.path().extension().and_then(|s| s.to_str()) == Some("desktop")
            })
        {
            let file_path = entry.path();

            // Try cache first
            let app_result: Result<desktop::App, eyre::Report> =
                if let Ok(Some(cached_app)) = desktop_cache.get(&file_path) {
                    Ok(cached_app)
                } else {
                    // Parse the file
                    match fs::read_to_string(&file_path) {
                        Ok(contents) => {
                            if !contents.contains("[Desktop Entry]") {
                                continue;
                            }

                            match desktop::App::parse(&contents, None, cli.filter_desktop) {
                                Ok(mut app) => {
                                    if let Some(file_name) =
                                        file_path.file_name().and_then(|n| n.to_str())
                                    {
                                        app.desktop_id = Some(file_name.to_string());
                                    }
                                    let _ = desktop_cache.set(&file_path, app.clone());
                                    Ok(app)
                                }
                                Err(_) => continue,
                            }
                        }
                        Err(_) => continue,
                    }
                };

            if let Ok(app) = app_result {
                // Check if this is the app we're looking for
                if app.name == app_name {
                    // Apply filtering if needed
                    if cli.filter_desktop {
                        if let Ok(current_desktop) = env::var("XDG_CURRENT_DESKTOP") {
                            let desktops: Vec<String> =
                                current_desktop.split(':').map(|s| s.to_string()).collect();

                            if !app.not_show_in.is_empty() {
                                let should_hide = app
                                    .not_show_in
                                    .iter()
                                    .any(|d| desktops.iter().any(|cd| cd.eq_ignore_ascii_case(d)));
                                if should_hide {
                                    continue;
                                }
                            }

                            if !app.only_show_in.is_empty() {
                                let should_show = app
                                    .only_show_in
                                    .iter()
                                    .any(|d| desktops.iter().any(|cd| cd.eq_ignore_ascii_case(d)));
                                if !should_show {
                                    continue;
                                }
                            }
                        }
                    }

                    // Found it! Apply history and return
                    return Ok(Some(history_cache.apply_to_app(app)));
                }
            }
        }
    }

    Ok(None)
}

/// Launch a program directly by name (bypass TUI)
pub fn launch_program_directly(cli: &cli::Opts, program_name: &str) -> Result<()> {
    // Open database for history
    let (db, _data_dir) = database::open_history_db()?;

    let program_name_lower = program_name.to_lowercase();

    // FAST PATH: Check history for exact or prefix match
    // This avoids loading any desktop files for common cases
    let history_cache = cache::HistoryCache::load(&db)?;

    // First try exact match in history
    for (app_name, _count) in history_cache.history.iter() {
        if app_name.to_lowercase() == program_name_lower {
            // Found exact match in history - try to find and launch it quickly
            if let Some(app) = find_app_by_name_fast(&db, app_name, cli)? {
                if cli.no_exec {
                    println!("{}", app.command);
                    return Ok(());
                }
                return super::launch::launch_app(&app, cli, &db);
            }
        }
    }

    // Try prefix match in history (e.g., "fire" -> "Firefox")
    if let Some((app_name, _)) = history_cache.get_best_match(program_name) {
        if let Some(app) = find_app_by_name_fast(&db, app_name, cli)? {
            if cli.no_exec {
                println!("{}", app.command);
                return Ok(());
            }
            return super::launch::launch_app(&app, cli, &db);
        }
    }

    // SLOW PATH: No exact match in history, need to load all apps
    let mut dirs: Vec<path::PathBuf> = vec![];

    // Add user's data directory
    if let Some(xdg_data_home) = env::var("XDG_DATA_HOME").ok().filter(|s| !s.is_empty()) {
        let mut dir = path::PathBuf::from(xdg_data_home);
        dir.push("applications");
        if dir.exists() {
            dirs.push(dir);
        }
    } else if let Some(home_dir) = dirs::home_dir() {
        let mut dir = home_dir;
        dir.push(".local/share/applications");
        if dir.exists() {
            dirs.push(dir);
        }
    }

    // Add system data directories
    if let Ok(res) = env::var("XDG_DATA_DIRS") {
        for data_dir in res.split(':').filter(|s| !s.is_empty()) {
            let mut dir = path::PathBuf::from(data_dir);
            dir.push("applications");
            if dir.exists() {
                dirs.push(dir);
            }
        }
    } else {
        // Default paths for Linux and BSD
        let mut default_paths = vec![
            path::PathBuf::from("/usr/local/share"),
            path::PathBuf::from("/usr/share"),
        ];

        // Add BSD-specific paths
        #[cfg(target_os = "openbsd")]
        {
            default_paths.push(path::PathBuf::from("/usr/X11R6/share"));
        }

        for data_dir in &mut default_paths {
            data_dir.push("applications");
            if data_dir.exists() {
                dirs.push(data_dir.clone());
            }
        }
    }

    // Read applications with filtering options
    let apps_receiver =
        desktop::read_with_options(dirs, &db, cli.filter_desktop, cli.list_executables_in_path);

    // Collect all apps
    let mut all_apps = Vec::new();
    while let Ok(app) = apps_receiver.recv() {
        all_apps.push(app);
    }

    if all_apps.is_empty() {
        return Err(eyre!("No applications found"));
    }

    // Find the best match using improved matching logic for -p
    let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
    let mut best_app: Option<(desktop::App, i64)> = None;

    for app in all_apps {
        let app_name_lower = app.name.to_lowercase();

        // extract executable name from command
        let exec_name = strings::extract_exec_name(&app.command);
        let exec_name_lower = exec_name.to_lowercase();

        // Prioritized matching: exact > prefix > fuzzy
        let mut final_score = if app_name_lower == program_name_lower {
            1_000_000 // Exact app name match
        } else if exec_name_lower == program_name_lower {
            900_000 // Exact executable name match
        } else if exec_name_lower.starts_with(&program_name_lower) {
            800_000 // Executable prefix match (e.g., "fo" matches "foot")
        } else if app_name_lower.starts_with(&program_name_lower) {
            700_000 // App name prefix match
        } else {
            // Fuzzy matching with priority for executable name (SIMD-accelerated)
            let name_score = matcher
                .fuzzy_match(
                    Utf32Str::Ascii(app.name.as_bytes()),
                    Utf32Str::Ascii(program_name.as_bytes()),
                )
                .unwrap_or(0) as i64;
            let exec_score = matcher
                .fuzzy_match(
                    Utf32Str::Ascii(exec_name.as_bytes()),
                    Utf32Str::Ascii(program_name.as_bytes()),
                )
                .unwrap_or(0) as i64;

            // Prioritize executable name matches (2x weight)
            let best_score = std::cmp::max(name_score, exec_score * 2);

            if best_score == 0 {
                continue; // No match at all
            }

            best_score
        };

        // apply pin boost (highest priority after exact matches)
        if app.pinned {
            if final_score < 700_000 {
                final_score += 500_000; // boost fuzzy matches significantly
            } else {
                final_score += 50_000; // boost exact matches slightly
            }
        }

        // include history in scoring (but don't let it dominate exact/prefix matches)
        if app.history > 0 {
            final_score = if final_score >= 700_000 {
                // for exact/prefix matches, history is just a tiebreaker
                final_score + app.history as i64
            } else {
                // for fuzzy matches, history multiplies the score
                final_score * app.history as i64
            };
        }

        if let Some((_, current_best_score)) = &best_app {
            if final_score > *current_best_score {
                best_app = Some((app, final_score));
            }
        } else {
            best_app = Some((app, final_score));
        }
    }

    let app_to_run = match best_app {
        Some((app, _)) => app,
        None => {
            return Err(eyre!(
                "No matching application found for '{}'",
                program_name
            ));
        }
    };

    // confirm first launch if enabled and app has no history
    if cli.confirm_first_launch && app_to_run.history == 0 {
        use std::io::{self, Write};
        eprint!("Launch {} [Y/n]? ", app_to_run.name);
        io::stderr().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;
        let response = response.trim().to_lowercase();

        if response == "n" || response == "no" {
            // user said no, drop into TUI with search pre-filled
            // we need to return an error that signals to continue to TUI
            // but we can't easily do that from here, so just exit
            eprintln!(
                "Cancelled. Use 'fsel -ss {}' to search in TUI.",
                program_name
            );
            std::process::exit(0);
        }
    }

    // print what we're launching if verbose
    if cli.verbose.unwrap_or(0) > 0 {
        eprintln!("Launching: {} ({})", app_to_run.name, app_to_run.command);
    }

    // handle --no-exec: print command and exit cleanly
    if cli.no_exec {
        println!("{}", app_to_run.command);
        return Ok(());
    }

    // launch the app
    super::launch::launch_app(&app_to_run, cli, &db)?;

    Ok(())
}
