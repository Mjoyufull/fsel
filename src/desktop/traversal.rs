//! Shared depth, hidden-file and symlink policy for desktop and icon discovery.

use std::path::Path;

pub(crate) enum Hidden {
    Include,
    Exclude,
}

pub(super) fn builder(root: &Path, hidden: Hidden) -> ignore::WalkBuilder {
    let mut walk = ignore::WalkBuilder::new(root);
    // XDG directories must not inherit source-control or global ignore rules.
    // Follow a root alias, but never recurse into descendant directory symlinks.
    walk.standard_filters(false)
        .hidden(matches!(hidden, Hidden::Exclude))
        .follow_links(false)
        .max_depth(Some(5));
    walk
}

pub(crate) fn entries(root: &Path, hidden: Hidden) -> impl Iterator<Item = ignore::DirEntry> {
    builder(root, hidden)
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.depth() > 0)
}

#[cfg(test)]
mod tests;
