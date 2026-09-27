//! File names per directory, shared by declared and undeclared icon lookups.

use rayon::prelude::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Names of the files each searched directory holds, read once per directory.
///
/// Every icon a launcher resolves asks the same directories the same question. Reading
/// a directory once keeps a theme's directory count out of the per-icon cost, which
/// themes declaring hundreds of directories otherwise charge as four filesystem probes
/// each, for every icon.
#[derive(Default)]
pub(super) struct DirectoryListing {
    names: RefCell<HashMap<PathBuf, Arc<HashSet<Box<str>>>>>,
}

impl DirectoryListing {
    /// Whether `directory` holds a file named `file_name`.
    pub(super) fn holds(&self, directory: &Path, file_name: &str) -> bool {
        // A link is listed without resolving it, so confirm the few names that match.
        self.names(directory).contains(file_name) && directory.join(file_name).is_file()
    }

    /// Read the directories a lookup is about to ask about, together.
    ///
    /// The first icon of a session waits on a cold directory cache rather than on the
    /// reading itself, so the whole theme is read at once instead of one directory at
    /// a time. Later icons then answer from memory.
    pub(super) fn prefill<'a>(&self, directories: impl IntoIterator<Item = &'a Path>) {
        let missing = {
            let names = self.names.borrow();
            directories
                .into_iter()
                .filter(|directory| !names.contains_key(*directory))
                .map(Path::to_path_buf)
                .collect::<Vec<_>>()
        };
        let read = missing
            .into_par_iter()
            .map(|directory| {
                let names = Arc::new(read_names(&directory));
                (directory, names)
            })
            .collect::<Vec<_>>();
        self.names.borrow_mut().extend(read);
    }

    fn names(&self, directory: &Path) -> Arc<HashSet<Box<str>>> {
        if let Some(names) = self.names.borrow().get(directory) {
            return Arc::clone(names);
        }

        let names = Arc::new(read_names(directory));
        self.names
            .borrow_mut()
            .insert(directory.to_path_buf(), Arc::clone(&names));
        names
    }
}

fn read_names(directory: &Path) -> HashSet<Box<str>> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return HashSet::new();
    };
    entries
        .filter_map(Result::ok)
        // A directory named like an icon file is not a candidate for one.
        .filter(|entry| entry.file_type().is_ok_and(|kind| !kind.is_dir()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .map(String::into_boxed_str)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::DirectoryListing;
    use std::fs;

    #[test]
    fn a_directory_named_like_an_icon_is_not_a_file() {
        let root = super::super::tests::temp_dir();
        fs::create_dir_all(root.join("editor.png")).unwrap();
        fs::write(root.join("editor.svg"), "image").unwrap();

        let listing = DirectoryListing::default();

        assert!(!listing.holds(&root, "editor.png"));
        assert!(listing.holds(&root, "editor.svg"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_missing_directory_holds_nothing() {
        let listing = DirectoryListing::default();

        assert!(!listing.holds(std::path::Path::new("/nonexistent-icon-root"), "editor.png"));
    }
}
