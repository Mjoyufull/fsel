//! Shared depth, hidden-file and symlink policy for desktop and icon discovery.

use rayon::prelude::*;
use std::path::{Path, PathBuf};

pub(crate) enum Hidden {
    Include,
    Exclude,
}

/// Deepest entry any desktop or icon lookup reads.
const MAX_DEPTH: usize = 5;

pub(super) fn builder(root: &Path, hidden: Hidden) -> ignore::WalkBuilder {
    let mut walk = ignore::WalkBuilder::new(root);
    // XDG directories must not inherit source-control or global ignore rules.
    // Follow a root alias, but never recurse into descendant directory symlinks.
    walk.standard_filters(false)
        .hidden(matches!(hidden, Hidden::Exclude))
        .follow_links(false)
        .max_depth(Some(MAX_DEPTH));
    walk
}

pub(crate) fn entries(root: &Path, hidden: Hidden) -> impl Iterator<Item = ignore::DirEntry> {
    builder(root, hidden)
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.depth() > 0)
}

/// `root` and every directory below it that an entry of [`entries`] can sit in.
///
/// Reading directory entries directly keeps the listing proportional to the tree: the
/// walker builds a matcher and an owned entry for every file it passes, which a theme
/// holding tens of thousands of icons pays for on every lookup.
pub(crate) fn directories(root: &Path, hidden: Hidden) -> Vec<PathBuf> {
    let mut directories = vec![root.to_path_buf()];
    let mut level = 0..1;
    for _ in 1..MAX_DEPTH {
        // Each level is read in parallel because a cold directory cache, not the
        // per-entry work, is what a first lookup waits on.
        let children = directories[level]
            .par_iter()
            .flat_map_iter(|parent| child_directories(parent, &hidden))
            .collect::<Vec<_>>();
        if children.is_empty() {
            break;
        }
        level = directories.len()..directories.len() + children.len();
        directories.extend(children);
    }
    directories
}

fn child_directories(parent: &Path, hidden: &Hidden) -> impl Iterator<Item = PathBuf> {
    let exclude_hidden = matches!(hidden, Hidden::Exclude);
    std::fs::read_dir(parent)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        // Never descend into a directory symlink, matching the walker's link policy.
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter(move |entry| {
            !exclude_hidden || !entry.file_name().as_encoded_bytes().starts_with(b".")
        })
        .map(|entry| entry.path())
}

#[cfg(test)]
mod tests;
