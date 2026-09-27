//! Directory-driven name lookup with serial resolution of equally ranked candidates.

use super::listing::DirectoryListing;
use super::{ICON_EXTENSIONS, IconCandidate};
use crate::desktop::traversal::{self, Hidden};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub(super) fn matching_paths(
    directories: &[PathBuf],
    icon: &str,
    listing: &DirectoryListing,
) -> Vec<PathBuf> {
    directories
        .iter()
        .flat_map(|directory| {
            ICON_EXTENSIONS
                .iter()
                .map(move |extension| (directory, format!("{icon}.{extension}")))
        })
        .filter(|(directory, file_name)| listing.holds(directory, file_name))
        .map(|(directory, file_name)| directory.join(file_name))
        .collect()
}

pub(super) fn best_in_traversal_order(
    mut candidates: Vec<IconCandidate>,
    roots: &[PathBuf],
    theme: &str,
) -> Option<PathBuf> {
    candidates.sort_by_key(IconCandidate::score);
    let first = candidates.first()?;
    let score = first.score();
    let tied: HashSet<&Path> = candidates
        .iter()
        .take_while(|candidate| candidate.score() == score)
        .map(|candidate| candidate.path.as_path())
        .collect();
    if tied.len() > 1 {
        // Rank by depth-first filesystem order, which is not the order directories
        // are listed in.
        let root = roots[first.root_rank].join(theme);
        if let Some(entry) =
            traversal::entries(&root, Hidden::Exclude).find(|entry| tied.contains(entry.path()))
        {
            return Some(entry.into_path());
        }
    }
    Some(first.path.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn fallback_probes_undeclared_directories_and_preserves_tie_order() {
        let root = super::super::tests::temp_dir();
        let theme = root.join("Test");
        for directory in ["a/32x32/apps", "b/32x32/apps", ".hidden/32x32/apps"] {
            fs::create_dir_all(theme.join(directory)).unwrap();
            fs::write(theme.join(directory).join("editor.png"), "image").unwrap();
            fs::write(theme.join(directory).join("editor.txt"), "not an icon").unwrap();
        }
        fs::create_dir_all(theme.join("editor.png")).unwrap();

        let directories = traversal::directories(&theme, Hidden::Exclude);
        assert!(directories.contains(&theme));
        assert!(
            !directories
                .iter()
                .any(|path| path.starts_with(theme.join(".hidden")))
        );
        let paths = matching_paths(&directories, "editor", &DirectoryListing::default());
        assert_eq!(
            paths.iter().collect::<HashSet<_>>(),
            HashSet::from([
                &theme.join("a/32x32/apps/editor.png"),
                &theme.join("b/32x32/apps/editor.png"),
            ])
        );

        let first_in_traversal = traversal::entries(&theme, Hidden::Exclude)
            .map(|entry| entry.into_path())
            .find(|path| paths.contains(path))
            .expect("the icon files are reachable by traversal");
        for ordering in [paths.clone(), paths.iter().rev().cloned().collect()] {
            let candidates = ordering
                .into_iter()
                .map(|path| IconCandidate::from_fallback(path, 32, 0))
                .collect();
            assert_eq!(
                best_in_traversal_order(candidates, std::slice::from_ref(&root), "Test"),
                Some(first_in_traversal.clone())
            );
        }
        assert!(matching_paths(&directories, "missing", &DirectoryListing::default()).is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn listing_stops_above_the_directories_holding_icon_files() {
        let root = super::super::tests::temp_dir();
        let deepest = root.join("a/b/c/d");
        fs::create_dir_all(deepest.join("e")).unwrap();
        fs::write(deepest.join("editor.png"), "image").unwrap();

        let directories = traversal::directories(&root, Hidden::Exclude);

        assert!(directories.contains(&deepest));
        assert!(!directories.contains(&deepest.join("e")));
        assert_eq!(
            matching_paths(&directories, "editor", &DirectoryListing::default()),
            vec![deepest.join("editor.png")]
        );
        fs::remove_dir_all(root).unwrap();
    }
}
