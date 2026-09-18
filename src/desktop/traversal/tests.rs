use super::{Hidden, entries};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("fsel-traversal-{}-{stamp}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    root
}

fn write(root: &Path, relative: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        path,
        "[Desktop Entry]\nType=Application\nName=Fixture\nExec=true\n",
    )
    .unwrap();
}

fn desktops(root: &Path, hidden: Hidden) -> BTreeSet<PathBuf> {
    entries(root, hidden)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "desktop"))
        .map(|entry| entry.into_path())
        .collect()
}

#[test]
fn depth_and_hidden_policy_do_not_enable_ignore_rules() {
    let root = fixture();
    for path in [
        "visible.desktop",
        ".hidden.desktop",
        ".hidden/inside.desktop",
        "a/b/c/d/last.desktop",
        "a/b/c/d/e/too-deep.desktop",
    ] {
        write(&root, path);
    }
    fs::write(root.join(".ignore"), "*.desktop\n").unwrap();
    fs::write(root.join(".gitignore"), "*.desktop\n").unwrap();
    let visible = BTreeSet::from([
        root.join("visible.desktop"),
        root.join("a/b/c/d/last.desktop"),
    ]);
    assert_eq!(desktops(&root, Hidden::Exclude), visible);
    let mut all = visible;
    all.extend([
        root.join(".hidden.desktop"),
        root.join(".hidden/inside.desktop"),
    ]);
    assert_eq!(desktops(&root, Hidden::Include), all);
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn root_alias_native_bytes_and_file_links_are_preserved_without_following_nested_links() {
    use std::ffi::OsStr;
    use std::os::unix::{ffi::OsStrExt, fs::symlink};
    let root = fixture();
    let real = root.join("real");
    write(&real, "visible.desktop");
    let raw_name = OsStr::from_bytes(b"raw-\xff.desktop");
    fs::write(real.join(raw_name), "test").unwrap();
    symlink("visible.desktop", real.join("file-link.desktop")).unwrap();
    symlink("missing", real.join("broken.desktop")).unwrap();
    symlink(".", real.join("loop")).unwrap();
    symlink("real", root.join("alias")).unwrap();
    let alias = root.join("alias");
    let found = desktops(&alias, Hidden::Include);
    assert_eq!(
        found,
        BTreeSet::from([
            alias.join("visible.desktop"),
            alias.join(raw_name),
            alias.join("file-link.desktop"),
            alias.join("broken.desktop")
        ])
    );
    assert!(found.iter().all(|path| path.starts_with(&alias)));
    assert_ne!(
        crate::desktop::desktop_file_id(std::slice::from_ref(&alias), &alias.join(raw_name)),
        crate::desktop::desktop_file_id(std::slice::from_ref(&alias), &alias.join("raw-�.desktop"))
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn early_return_drops_the_iterator_and_missing_roots_are_empty() {
    let root = fixture();
    for index in 0..20 {
        write(&root, &format!("dir{index}/app.desktop"));
    }
    assert!(entries(&root, Hidden::Exclude).next().is_some());
    assert_eq!(entries(&root.join("missing"), Hidden::Include).count(), 0);
    fs::remove_dir_all(root).unwrap();
}
