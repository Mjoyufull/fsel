use crate::cli;
use crate::core::cache;
use crate::core::hidden_entries::EntryKey;
use crate::desktop;
use eyre::Result;
use jwalk::WalkDir;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Find an app by name using the desktop cache first, then a targeted filesystem scan.
pub fn find_app_by_name_fast(
    db: &std::sync::Arc<redb::Database>,
    app_name: &str,
    cli: &cli::Opts,
    hidden_entry_keys: &HashSet<EntryKey>,
) -> Result<Option<desktop::App>> {
    let desktop_cache = cache::DesktopCache::new(db.clone())?;
    let history_cache = cache::HistoryCache::load(db)?;

    if let Ok(Some(app)) = desktop_cache.get_by_name(app_name)
        && !app.hidden
        && matches_current_desktop(&app, cli)
        && !is_hidden(&app, hidden_entry_keys)
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
                && !is_hidden(&app, hidden_entry_keys)
            {
                return Ok(Some(history_cache.apply_to_app(app)));
            }
        }
    }

    Ok(None)
}

fn is_hidden(app: &desktop::App, hidden_entry_keys: &HashSet<EntryKey>) -> bool {
    app.entry_key()
        .is_some_and(|entry_key| hidden_entry_keys.contains(&entry_key))
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
    app.set_source_path(file_path);

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

#[cfg(test)]
mod tests {
    use super::find_app_by_name_fast;
    use crate::cli::Opts;
    use crate::core::hidden_entries::EntryKey;
    use crate::desktop::App;
    use std::collections::HashSet;
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;

    #[test]
    fn cached_name_lookup_does_not_bypass_manual_hides() {
        let dir = std::env::temp_dir().join(format!("fsel-hidden-fast-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("test directory should be created");
        let desktop_path = dir.join("fixture.desktop");
        fs::write(
            &desktop_path,
            "[Desktop Entry]\nType=Application\nName=FselHiddenFixtureExact\nExec=/bin/true\n",
        )
        .expect("desktop entry should be written");
        let db = Arc::new(
            redb::Database::create(dir.join("history.redb")).expect("database should be created"),
        );
        let mut app = App::parse(
            fs::read_to_string(&desktop_path).expect("fixture should be readable"),
            false,
        )
        .expect("fixture should parse");
        app.desktop_id = Some("fixture.desktop".to_string());
        app.set_source_path(&desktop_path);
        crate::core::cache::DesktopCache::new(Arc::clone(&db))
            .expect("cache should initialize")
            .set(&desktop_path, app)
            .expect("cache should store fixture");
        let hidden = HashSet::from([EntryKey::desktop(
            Path::new(&desktop_path),
            "fixture.desktop",
        )]);

        let found = find_app_by_name_fast(&db, "FselHiddenFixtureExact", &Opts::default(), &hidden)
            .expect("lookup should succeed");

        assert!(found.is_none());
        drop(db);
        fs::remove_dir_all(dir).expect("test directory should be removed");
    }
}
