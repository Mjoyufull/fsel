use super::tables::{DESKTOP_CACHE_TABLE, FILE_LIST_TABLE, NAME_INDEX_TABLE};
use crate::desktop::App;
use eyre::Result;
use redb::{Database, ReadableDatabase, ReadableTable};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

const FILE_LIST_CACHE_KEY: &str = "paths";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    app: App,
    mtime: SystemTime,
    path: PathBuf,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileListCache {
    paths: Vec<PathBuf>,
    dir_mtimes: HashMap<PathBuf, SystemTime>,
}

pub struct DesktopCache {
    db: Arc<Database>,
}

impl DesktopCache {
    pub fn new(db: Arc<Database>) -> Result<Self> {
        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(DESKTOP_CACHE_TABLE)?;
            let _ = write_txn.open_table(NAME_INDEX_TABLE)?;
            let _ = write_txn.open_table(FILE_LIST_TABLE)?;
        }
        write_txn.commit()?;

        Ok(Self { db })
    }

    #[allow(dead_code)]
    pub fn get_file_list(&self, dirs: &[PathBuf]) -> Result<Option<Vec<PathBuf>>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(FILE_LIST_TABLE)?;

        if let Some(data) = table.get(FILE_LIST_CACHE_KEY)?
            && let Ok(cache) = postcard::from_bytes::<FileListCache>(data.value())
            && file_list_cache_is_fresh(&cache, dirs)
        {
            return Ok(Some(cache.paths));
        }

        Ok(None)
    }

    #[allow(dead_code)]
    pub fn batch_get(&self, paths: &[PathBuf]) -> Result<HashMap<PathBuf, App>> {
        let mut result = HashMap::new();
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(DESKTOP_CACHE_TABLE)?;

        for path in paths {
            if let Some(app) = read_cached_app(&table, path)? {
                result.insert(path.clone(), app);
            }
        }

        Ok(result)
    }

    #[allow(dead_code)]
    pub fn batch_set(&self, apps: Vec<(PathBuf, App)>) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut cache_table = write_txn.open_table(DESKTOP_CACHE_TABLE)?;
            let mut index_table = write_txn.open_table(NAME_INDEX_TABLE)?;

            for (path, app) in apps {
                write_cached_app(&mut cache_table, &mut index_table, &path, &app)?;
            }
        }
        write_txn.commit()?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn set_file_list(&self, paths: Vec<PathBuf>, scanned_dirs: &[PathBuf]) -> Result<()> {
        let cache = FileListCache {
            paths,
            dir_mtimes: collect_dir_mtimes(scanned_dirs),
        };
        let data = postcard::to_allocvec(&cache)?;

        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(FILE_LIST_TABLE)?;
            table.insert(FILE_LIST_CACHE_KEY, data.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn get_by_name(&self, app_name: &str) -> Result<Option<App>> {
        let read_txn = self.db.begin_read()?;
        let index_table = read_txn.open_table(NAME_INDEX_TABLE)?;

        if let Some(path_bytes) = index_table.get(app_name)? {
            let path = PathBuf::from(String::from_utf8_lossy(path_bytes.value()).as_ref());
            return self.get(&path);
        }

        Ok(None)
    }

    pub fn get(&self, path: &Path) -> Result<Option<App>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(DESKTOP_CACHE_TABLE)?;
        read_cached_app(&table, path)
    }

    pub fn set(&self, path: &Path, app: App) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut cache_table = write_txn.open_table(DESKTOP_CACHE_TABLE)?;
            let mut index_table = write_txn.open_table(NAME_INDEX_TABLE)?;
            write_cached_app(&mut cache_table, &mut index_table, path, &app)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn clear(&self) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut cache_table = write_txn.open_table(DESKTOP_CACHE_TABLE)?;
            let mut index_table = write_txn.open_table(NAME_INDEX_TABLE)?;
            let mut file_list_table = write_txn.open_table(FILE_LIST_TABLE)?;

            remove_all_rows(&mut cache_table)?;
            remove_all_rows(&mut index_table)?;
            remove_all_rows(&mut file_list_table)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn clear_file_list(&self) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(FILE_LIST_TABLE)?;
            remove_all_rows(&mut table)?;
        }
        write_txn.commit()?;
        Ok(())
    }
}

fn collect_dir_mtimes(scanned_dirs: &[PathBuf]) -> HashMap<PathBuf, SystemTime> {
    let mut dir_mtimes = HashMap::new();

    for dir in scanned_dirs {
        if let Ok(metadata) = fs::metadata(dir)
            && let Ok(mtime) = metadata.modified()
        {
            dir_mtimes.insert(dir.clone(), mtime);
        }
    }

    dir_mtimes
}

fn file_list_cache_is_fresh(cache: &FileListCache, dirs: &[PathBuf]) -> bool {
    for dir in dirs {
        let Some(cached_mtime) = cache.dir_mtimes.get(dir) else {
            return false;
        };

        let Ok(metadata) = fs::metadata(dir) else {
            return false;
        };
        let Ok(current_mtime) = metadata.modified() else {
            return false;
        };

        if current_mtime != *cached_mtime {
            return false;
        }
    }

    cache.dir_mtimes.len() == dirs.len()
}

fn read_cached_app(table: &redb::ReadOnlyTable<&str, &[u8]>, path: &Path) -> Result<Option<App>> {
    let path_key = path.to_string_lossy();

    if let Some(data) = table.get(path_key.as_ref())?
        && let Ok(entry) = postcard::from_bytes::<CacheEntry>(data.value())
        && cache_entry_is_fresh(path, &entry)
    {
        return Ok(Some(entry.app));
    }

    Ok(None)
}

fn cache_entry_is_fresh(path: &Path, entry: &CacheEntry) -> bool {
    if let Ok(metadata) = fs::metadata(path)
        && let Ok(mtime) = metadata.modified()
    {
        return mtime == entry.mtime;
    }

    false
}

fn write_cached_app(
    cache_table: &mut redb::Table<&str, &[u8]>,
    index_table: &mut redb::Table<&str, &[u8]>,
    path: &Path,
    app: &App,
) -> Result<()> {
    let Ok(metadata) = fs::metadata(path) else {
        return Ok(());
    };
    let Ok(mtime) = metadata.modified() else {
        return Ok(());
    };

    let entry = CacheEntry {
        app: app.clone(),
        mtime,
        path: path.to_path_buf(),
    };
    let data = postcard::to_allocvec(&entry)?;
    let path_key = path.to_string_lossy();

    cache_table.insert(path_key.as_ref(), data.as_slice())?;
    index_table.insert(app.name.as_str(), path_key.as_bytes())?;
    Ok(())
}

fn remove_all_rows<V>(table: &mut redb::Table<&str, V>) -> Result<()>
where
    V: redb::Value + 'static,
{
    let keys: Vec<String> = table
        .iter()?
        .filter_map(|result| result.ok().map(|(key, _)| key.value().to_string()))
        .collect();

    for key in keys {
        table.remove(key.as_str())?;
    }

    Ok(())
}
