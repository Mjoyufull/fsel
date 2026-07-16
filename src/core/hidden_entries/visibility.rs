use super::EntryKey;
use crate::desktop::App;
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct HiddenSummary {
    pub(crate) manual: usize,
    pub(crate) automatic: usize,
    pub(crate) unavailable: usize,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct VisibilityOptions {
    pub(crate) auto_hide_duplicates: bool,
    pub(crate) application_dirs: Vec<PathBuf>,
}

pub(crate) fn eligible_apps(
    apps: &[App],
    manually_hidden: &HashSet<EntryKey>,
    options: &VisibilityOptions,
) -> (Vec<App>, HiddenSummary) {
    let discovered_keys = apps
        .iter()
        .filter_map(App::entry_key)
        .collect::<HashSet<_>>();
    let manual = manually_hidden.intersection(&discovered_keys).count();
    let unavailable = manually_hidden
        .difference(&discovered_keys)
        .filter(|key| key.source_path().is_some_and(|path| !path.exists()))
        .count();
    let manually_eligible = apps
        .iter()
        .filter(|app| {
            !app.entry_key()
                .is_some_and(|key| manually_hidden.contains(&key))
        })
        .collect::<Vec<_>>();

    if !options.auto_hide_duplicates {
        let visible = manually_eligible
            .into_iter()
            .filter(|app| !app.hidden)
            .cloned()
            .collect();
        return (
            visible,
            HiddenSummary {
                manual,
                automatic: 0,
                unavailable,
            },
        );
    }

    let visible_candidate_count = manually_eligible.iter().filter(|app| !app.hidden).count();
    let mut priority_order = manually_eligible
        .iter()
        .enumerate()
        .map(|(index, app)| (index, priority_key(app, &options.application_dirs)))
        .collect::<Vec<_>>();
    priority_order.sort_by(|(_, left), (_, right)| left.cmp(right));

    let mut desktop_ids = HashSet::new();
    let mut desktop_winners = Vec::new();
    for (index, _) in priority_order {
        let app = manually_eligible[index];
        if let Some(desktop_id) = app.desktop_id.as_deref()
            && !desktop_ids.insert(desktop_id.to_string())
        {
            continue;
        }
        if !app.hidden {
            desktop_winners.push(index);
        }
    }

    let mut names = HashSet::new();
    let visible_indexes = desktop_winners
        .into_iter()
        .filter(|index| names.insert(normalized_name(&manually_eligible[*index].name)))
        .collect::<HashSet<_>>();

    let visible = manually_eligible
        .into_iter()
        .enumerate()
        .filter(|(index, _)| visible_indexes.contains(index))
        .map(|(_, app)| app.clone())
        .collect::<Vec<_>>();
    let automatic = visible_candidate_count.saturating_sub(visible.len());

    (
        visible,
        HiddenSummary {
            manual,
            automatic,
            unavailable,
        },
    )
}

fn priority_key(app: &App, application_dirs: &[PathBuf]) -> (usize, String, String) {
    let source_path = app.source_path();
    let root_rank = source_path
        .and_then(|path| {
            application_dirs
                .iter()
                .position(|root| path.starts_with(root))
        })
        .unwrap_or(application_dirs.len());
    let relative_path = source_path
        .and_then(|path| {
            application_dirs
                .get(root_rank)
                .and_then(|root| path.strip_prefix(root).ok())
        })
        .or(source_path)
        .map(crate::core::path_key::encode)
        .unwrap_or_default();
    let entry_key = app
        .entry_key()
        .map(|key| key.as_str().to_string())
        .unwrap_or_default();
    (root_rank, relative_path, entry_key)
}

fn normalized_name(name: &str) -> String {
    name.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{VisibilityOptions, eligible_apps};
    use crate::core::hidden_entries::EntryKey;
    use crate::desktop::App;
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};

    fn app(name: &str, path: &str, desktop_id: &str) -> App {
        let mut app = App::parse(
            format!("[Desktop Entry]\nType=Application\nName={name}\nExec=/bin/true"),
            false,
        )
        .expect("fixture should parse");
        app.desktop_id = Some(desktop_id.to_string());
        app.set_source_path(Path::new(path));
        app
    }

    fn options() -> VisibilityOptions {
        VisibilityOptions {
            auto_hide_duplicates: true,
            application_dirs: vec![
                PathBuf::from("/home/me/.local/share/applications"),
                PathBuf::from("/usr/share/applications"),
            ],
        }
    }

    #[test]
    fn automatic_hiding_is_disabled_by_default() {
        let apps = vec![
            app("Editor", "/one/editor.desktop", "editor.desktop"),
            app("Editor", "/two/editor.desktop", "editor.desktop"),
        ];

        let (visible, summary) =
            eligible_apps(&apps, &HashSet::new(), &VisibilityOptions::default());

        assert_eq!(visible.len(), 2);
        assert_eq!(summary.automatic, 0);
    }

    #[test]
    fn automatic_hiding_uses_root_precedence_not_input_order() {
        let system = app(
            "Vivaldi",
            "/usr/share/applications/vivaldi.desktop",
            "vivaldi.desktop",
        );
        let user = app(
            "Vivaldi",
            "/home/me/.local/share/applications/vivaldi.desktop",
            "vivaldi.desktop",
        );

        let (visible, summary) = eligible_apps(&[system, user], &HashSet::new(), &options());

        assert_eq!(visible.len(), 1);
        assert_eq!(
            visible[0].source_path(),
            Some(Path::new(
                "/home/me/.local/share/applications/vivaldi.desktop"
            ))
        );
        assert_eq!(summary.automatic, 1);
    }

    #[test]
    fn hiding_the_automatic_winner_reveals_the_next_source() {
        let user = app(
            "Vivaldi",
            "/home/me/.local/share/applications/vivaldi.desktop",
            "vivaldi.desktop",
        );
        let system = app(
            "Vivaldi",
            "/usr/share/applications/vivaldi.desktop",
            "vivaldi.desktop",
        );
        let user_key = user.entry_key().expect("user app should have an entry key");

        let (visible, summary) =
            eligible_apps(&[system, user], &HashSet::from([user_key]), &options());

        assert_eq!(visible.len(), 1);
        assert_eq!(
            visible[0].source_path(),
            Some(Path::new("/usr/share/applications/vivaldi.desktop"))
        );
        assert_eq!(summary.manual, 1);
        assert_eq!(summary.automatic, 0);
    }

    #[test]
    fn equal_names_with_different_ids_are_suppressed_conservatively() {
        let first = app(
            " Browser ",
            "/home/me/.local/share/applications/first.desktop",
            "first.desktop",
        );
        let second = app(
            "browser",
            "/usr/share/applications/second.desktop",
            "second.desktop",
        );

        let (visible, _) = eligible_apps(&[second, first], &HashSet::new(), &options());

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].desktop_id.as_deref(), Some("first.desktop"));
    }

    #[test]
    fn equal_names_in_one_root_use_lexical_source_order() {
        let later = app(
            "Browser",
            "/usr/share/applications/z-browser.desktop",
            "z-browser.desktop",
        );
        let earlier = app(
            "Browser",
            "/usr/share/applications/a-browser.desktop",
            "a-browser.desktop",
        );

        let (visible, _) = eligible_apps(&[later, earlier], &HashSet::new(), &options());

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].desktop_id.as_deref(), Some("a-browser.desktop"));
    }

    #[test]
    fn tombstone_reserves_the_desktop_id() {
        let mut tombstone = app(
            "Vivaldi",
            "/home/me/.local/share/applications/vivaldi.desktop",
            "vivaldi.desktop",
        );
        tombstone.hidden = true;
        let system = app(
            "Vivaldi",
            "/usr/share/applications/vivaldi.desktop",
            "vivaldi.desktop",
        );

        let (visible, summary) = eligible_apps(&[system, tombstone], &HashSet::new(), &options());

        assert!(visible.is_empty());
        assert_eq!(summary.automatic, 1);
    }

    #[test]
    fn manual_hides_are_source_specific() {
        let first = app("Editor", "/one/editor.desktop", "editor.desktop");
        let second = app("Editor", "/two/editor.desktop", "editor.desktop");
        let hidden = EntryKey::desktop(Path::new("/one/editor.desktop"), "editor.desktop");

        let (visible, summary) = eligible_apps(
            &[first, second],
            &HashSet::from([hidden]),
            &VisibilityOptions::default(),
        );

        assert_eq!(visible.len(), 1);
        assert_eq!(
            visible[0].source_path(),
            Some(Path::new("/two/editor.desktop"))
        );
        assert_eq!(summary.manual, 1);
    }

    #[test]
    fn missing_manual_sources_are_reported_as_unavailable() {
        let app = app("Editor", "/one/editor.desktop", "editor.desktop");
        let missing = EntryKey::desktop(Path::new("/missing/editor.desktop"), "editor.desktop");

        let (visible, summary) = eligible_apps(
            &[app],
            &HashSet::from([missing]),
            &VisibilityOptions::default(),
        );

        assert_eq!(visible.len(), 1);
        assert_eq!(summary.manual, 0);
        assert_eq!(summary.unavailable, 1);
    }
}
