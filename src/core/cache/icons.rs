use super::tables::ICON_PATH_CACHE_TABLE;
use eyre::Result;
use redb::{Database, ReadableDatabase};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    path: Option<PathBuf>,
}

/// Result of a persistent icon-path lookup.
pub(crate) enum IconPathLookup {
    Missing,
    Hit(Option<PathBuf>),
}

/// Persistent icon resolution metadata. Image contents are never stored here.
pub(crate) struct IconPathCache {
    db: Arc<Database>,
}

impl IconPathCache {
    pub(crate) fn new(db: Arc<Database>) -> Result<Self> {
        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(ICON_PATH_CACHE_TABLE)?;
        }
        write_txn.commit()?;
        Ok(Self { db })
    }

    pub(crate) fn get(&self, key: &str) -> Result<IconPathLookup> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(ICON_PATH_CACHE_TABLE)?;
        let Some(data) = table.get(key)? else {
            return Ok(IconPathLookup::Missing);
        };
        let Ok(entry) = postcard::from_bytes::<CacheEntry>(data.value()) else {
            return Ok(IconPathLookup::Missing);
        };
        if entry.path.as_ref().is_some_and(|path| !path.is_file()) {
            return Ok(IconPathLookup::Missing);
        }
        Ok(IconPathLookup::Hit(entry.path))
    }

    pub(crate) fn set(&self, key: &str, path: Option<PathBuf>) -> Result<()> {
        let data = postcard::to_allocvec(&CacheEntry { path })?;
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(ICON_PATH_CACHE_TABLE)?;
            table.insert(key, data.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{IconPathCache, IconPathLookup};
    use std::fs;
    use std::sync::Arc;

    #[test]
    fn path_and_missing_results_round_trip() {
        let root = std::env::temp_dir().join(format!(
            "fsel-icon-path-cache-{}-{}",
            crate::platform::process::get_current_pid(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should follow the Unix epoch")
                .as_nanos(),
        ));
        fs::create_dir_all(&root).expect("cache test directory should be created");
        let icon = root.join("icon.png");
        fs::write(&icon, b"icon").expect("icon should be written");
        let db = Arc::new(
            redb::Database::create(root.join("cache.redb")).expect("database should be created"),
        );
        let cache = IconPathCache::new(db).expect("cache should initialize");

        cache
            .set("present", Some(icon.clone()))
            .expect("path should be cached");
        cache.set("missing", None).expect("miss should be cached");

        assert!(matches!(
            cache.get("present").expect("path should load"),
            IconPathLookup::Hit(Some(path)) if path == icon
        ));
        assert!(matches!(
            cache.get("missing").expect("miss should load"),
            IconPathLookup::Hit(None)
        ));
        assert!(matches!(
            cache.get("unknown").expect("unknown key should load"),
            IconPathLookup::Missing
        ));
        drop(cache);
        let _ = fs::remove_dir_all(root);
    }
}
