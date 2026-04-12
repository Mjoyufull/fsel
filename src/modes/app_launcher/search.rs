use crate::cli;
use crate::core::cache;
use crate::desktop;
use eyre::Result;
use jwalk::WalkDir;
use std::fs;
use std::path::Path;

/// Find an app by name using the desktop cache first, then a targeted filesystem scan.
pub fn find_app_by_name_fast(
    db: &std::sync::Arc<redb::Database>,
    app_name: &str,
    cli: &cli::Opts,
) -> Result<Option<desktop::App>> {
    let desktop_cache = cache::DesktopCache::new(db.clone())?;
    let history_cache = cache::HistoryCache::load(db)?;

    if let Ok(Some(app)) = desktop_cache.get_by_name(app_name)
        && matches_current_desktop(&app, cli)
    {
        return Ok(Some(history_cache.apply_to_app(app)));
    }

    for dir in crate::desktop::application_dirs() {
        for entry in WalkDir::new(dir)
            .min_depth(1)
            .max_depth(5)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| {
                !entry.file_type().is_dir()
                    && entry.path().extension().and_then(|ext| ext.to_str()) == Some("desktop")
            })
        {
            let file_path = entry.path();

            if let Some(app) = load_app_from_path(&desktop_cache, &file_path, cli)?
                && app.name == app_name
                && matches_current_desktop(&app, cli)
            {
                return Ok(Some(history_cache.apply_to_app(app)));
            }
        }
    }

    Ok(None)
}

fn load_app_from_path(
    desktop_cache: &cache::DesktopCache,
    file_path: &Path,
    cli: &cli::Opts,
) -> Result<Option<desktop::App>> {
    if let Ok(Some(cached_app)) = desktop_cache.get(file_path) {
        return Ok(Some(cached_app));
    }

    let contents = match fs::read_to_string(file_path) {
        Ok(contents) => contents,
        Err(_) => return Ok(None),
    };
    if !contents.contains("[Desktop Entry]") {
        return Ok(None);
    }

    let mut app = match desktop::App::parse(&contents, cli.filter_desktop) {
        Ok(app) => app,
        Err(_) => return Ok(None),
    };

    if let Some(file_name) = file_path.file_name().and_then(|name| name.to_str()) {
        app.desktop_id = Some(file_name.to_string());
    }

    let _ = desktop_cache.set(file_path, app.clone());
    Ok(Some(app))
}

fn matches_current_desktop(app: &desktop::App, cli: &cli::Opts) -> bool {
    if !cli.filter_desktop {
        return true;
    }

    let Ok(current_desktop) = std::env::var("XDG_CURRENT_DESKTOP") else {
        return true;
    };
    let desktops: Vec<String> = current_desktop
        .split(':')
        .map(|entry| entry.to_string())
        .collect();

    if app.not_show_in.iter().any(|desktop| {
        desktops
            .iter()
            .any(|current| current.eq_ignore_ascii_case(desktop))
    }) {
        return false;
    }

    if app.only_show_in.is_empty() {
        return true;
    }

    app.only_show_in.iter().any(|desktop| {
        desktops
            .iter()
            .any(|current| current.eq_ignore_ascii_case(desktop))
    })
}
