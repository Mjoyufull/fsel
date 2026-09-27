//! Parallel name lookup with serial resolution of equally ranked candidates.

use super::{IconCandidate, has_icon_name};
use crate::desktop::traversal::{self, Hidden};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

// The launcher owns one resolver on its lookup worker. Lookups finish before
// the next begins; only image decoding is fanned out separately.
const LOOKUP_THREADS: usize = 4;

pub(super) fn matching_paths(root: &Path, icon: &str) -> Vec<PathBuf> {
    let (sender, receiver) = mpsc::channel();
    traversal::builder(root, Hidden::Exclude)
        .threads(LOOKUP_THREADS)
        .build_parallel()
        .run(|| {
            Box::new(|result| {
                if let Ok(entry) = result {
                    let path = entry.path();
                    // Avoid metadata syscalls for the unrelated majority of filenames.
                    if entry.depth() > 0
                        && has_icon_name(path, icon)
                        && path.is_file()
                        && sender.send(path.to_path_buf()).is_err()
                    {
                        return ignore::WalkState::Quit;
                    }
                }
                ignore::WalkState::Continue
            })
        });
    drop(sender);
    receiver.into_iter().collect()
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
        // Preserve depth-first filesystem order, not worker completion order.
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
    fn fallback_matches_names_before_validating_files_and_preserves_tie_order() {
        let root = super::super::tests::temp_dir();
        let theme = root.join("Test");
        for directory in ["a/32x32/apps", "b/32x32/apps", ".hidden/32x32/apps"] {
            fs::create_dir_all(theme.join(directory)).unwrap();
            fs::write(theme.join(directory).join("editor.png"), "image").unwrap();
            fs::write(theme.join(directory).join("editor.txt"), "not an icon").unwrap();
        }
        fs::create_dir_all(theme.join("editor.png")).unwrap();
        let serial = traversal::entries(&theme, Hidden::Exclude)
            .map(|entry| entry.into_path())
            .filter(|path| has_icon_name(path, "editor") && path.is_file())
            .collect::<Vec<_>>();
        assert_eq!(serial.len(), 2);
        let paths = matching_paths(&theme, "editor");
        assert_eq!(
            paths.iter().collect::<HashSet<_>>(),
            serial.iter().collect::<HashSet<_>>()
        );
        for ordering in [serial.clone(), serial.iter().rev().cloned().collect()] {
            let candidates = ordering
                .into_iter()
                .map(|path| IconCandidate::from_fallback(path, 32, 0))
                .collect();
            assert_eq!(
                best_in_traversal_order(candidates, std::slice::from_ref(&root), "Test"),
                Some(serial[0].clone())
            );
        }
        assert!(matching_paths(&theme, "missing").is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
