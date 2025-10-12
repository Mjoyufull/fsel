/// Desktop file cache for fast startup
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use eyre::Result;
use serde::{Deserialize, Serialize};
use redb::{Database, ReadableTable, ReadableDatabase, TableDefinition};

use crate::xdg::App;

// Table definitions - centralized for consistency
pub const DESKTOP_CACHE_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("desktop_cache");
pub const NAME_INDEX_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("app_name_index");
pub const FILE_LIST_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("file_list_cache");
pub const HISTORY_TABLE: TableDefinition<&str, u64> = TableDefinition::new("history");
pub const PINNED_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("pinned_apps");

/// Cache entry for a desktop file
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    /// Parsed app
    app: App,
    /// Last modified time of the desktop file
    mtime: SystemTime,
    /// Path to the desktop file
    path: PathBuf,
}

/// Desktop file cache
pub struct DesktopCache {
    db: std::sync::Arc<Database>,
}

/// Cached directory listing
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileListCache {
    /// List of all desktop file paths
    paths: Vec<PathBuf>,
    /// Last time we scanned directories
    last_scan: SystemTime,
}

impl DesktopCache {
    /// Create a new cache with the given database
    pub fn new(db: std::sync::Arc<Database>) -> Result<Self> {
        // Create all tables if they don't exist
        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(DESKTOP_CACHE_TABLE)?;
            let _ = write_txn.open_table(NAME_INDEX_TABLE)?;
            let _ = write_txn.open_table(FILE_LIST_TABLE)?;
            let _ = write_txn.open_table(HISTORY_TABLE)?;
            let _ = write_txn.open_table(PINNED_TABLE)?;
        }
        write_txn.commit()?;
        
        Ok(Self { db })
    }
    
    /// Get cached file list (avoids directory walk)
    /// Returns None if cache is stale (older than 5 minutes)
    #[allow(dead_code)]
    pub fn get_file_list(&self) -> Result<Option<Vec<PathBuf>>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(FILE_LIST_TABLE)?;
        
        if let Some(data) = table.get("paths")? {
            if let Ok(cache) = bincode::deserialize::<FileListCache>(data.value()) {
                // Check if cache is fresh (less than 5 minutes old)
                if let Ok(elapsed) = SystemTime::now().duration_since(cache.last_scan) {
                    if elapsed.as_secs() < 300 {  // 5 minutes
                        return Ok(Some(cache.paths));
                    }
                }
            }
        }
        Ok(None)
    }
    
    /// Batch get multiple apps from cache
    /// Returns HashMap of path -> app for cache hits
    #[allow(dead_code)]
    pub fn batch_get(&self, paths: &[PathBuf]) -> Result<std::collections::HashMap<PathBuf, App>> {
        let mut result = std::collections::HashMap::new();
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(DESKTOP_CACHE_TABLE)?;
        
        for path in paths {
            let path_key = path.to_string_lossy();
            if let Some(data) = table.get(path_key.as_ref())? {
                if let Ok(entry) = bincode::deserialize::<CacheEntry>(data.value()) {
                    // Check if file has been modified
                    if let Ok(metadata) = fs::metadata(path) {
                        if let Ok(mtime) = metadata.modified() {
                            if mtime == entry.mtime {
                                result.insert(path.clone(), entry.app);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(result)
    }
    
    /// Batch set multiple apps in cache
    /// Much faster than individual sets - uses single transaction
    #[allow(dead_code)]
    pub fn batch_set(&self, apps: Vec<(PathBuf, App)>) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut cache_table = write_txn.open_table(DESKTOP_CACHE_TABLE)?;
            let mut index_table = write_txn.open_table(NAME_INDEX_TABLE)?;
            
            for (path, app) in apps {
                if let Ok(metadata) = fs::metadata(&path) {
                    if let Ok(mtime) = metadata.modified() {
                        let entry = CacheEntry {
                            app: app.clone(),
                            mtime,
                            path: path.clone(),
                        };
                        
                        let path_key = path.to_string_lossy();
                        let data = bincode::serialize(&entry)?;
                        cache_table.insert(path_key.as_ref(), data.as_slice())?;
                        
                        // Update name index
                        let path_str = path.to_string_lossy();
                        index_table.insert(app.name.as_str(), path_str.as_bytes())?;
                    }
                }
            }
        }
        write_txn.commit()?;
        Ok(())
    }
    
    /// Store file list in cache
    #[allow(dead_code)]
    pub fn set_file_list(&self, paths: Vec<PathBuf>) -> Result<()> {
        let cache = FileListCache {
            paths,
            last_scan: SystemTime::now(),
        };
        let data = bincode::serialize(&cache)?;
        
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(FILE_LIST_TABLE)?;
            table.insert("paths", data.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }
    
    /// Get app by name (uses index for fast lookup)
    pub fn get_by_name(&self, app_name: &str) -> Result<Option<App>> {
        let read_txn = self.db.begin_read()?;
        let index_table = read_txn.open_table(NAME_INDEX_TABLE)?;
        
        // Look up the path in the index
        if let Some(path_bytes) = index_table.get(app_name)? {
            let path_str = String::from_utf8_lossy(path_bytes.value());
            let path = PathBuf::from(path_str.as_ref());
            
            // Get the cached app from the path
            return self.get(&path);
        }
        
        Ok(None)
    }
    
    /// Get cached app if file hasn't changed
    pub fn get(&self, path: &Path) -> Result<Option<App>> {
        let path_key = path.to_string_lossy();
        
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(DESKTOP_CACHE_TABLE)?;
        
        if let Some(data) = table.get(path_key.as_ref())? {
            if let Ok(entry) = bincode::deserialize::<CacheEntry>(data.value()) {
                // Check if file has been modified
                if let Ok(metadata) = fs::metadata(path) {
                    if let Ok(mtime) = metadata.modified() {
                        if mtime == entry.mtime {
                            return Ok(Some(entry.app));
                        }
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// Store app in cache with current mtime and update name index
    pub fn set(&self, path: &Path, app: App) -> Result<()> {
        if let Ok(metadata) = fs::metadata(path) {
            if let Ok(mtime) = metadata.modified() {
                let entry = CacheEntry {
                    app: app.clone(),
                    mtime,
                    path: path.to_path_buf(),
                };
                
                let path_key = path.to_string_lossy();
                let data = bincode::serialize(&entry)?;
                
                let write_txn = self.db.begin_write()?;
                {
                    let mut cache_table = write_txn.open_table(DESKTOP_CACHE_TABLE)?;
                    let mut index_table = write_txn.open_table(NAME_INDEX_TABLE)?;
                    
                    cache_table.insert(path_key.as_ref(), data.as_slice())?;
                    
                    // Update name index for fast lookup
                    let path_str = path.to_string_lossy();
                    index_table.insert(app.name.as_str(), path_str.as_bytes())?;
                }
                write_txn.commit()?;
            }
        }
        
        Ok(())
    }
    
    /// Clear the entire cache, name index, and file list
    pub fn clear(&self) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut cache_table = write_txn.open_table(DESKTOP_CACHE_TABLE)?;
            let mut index_table = write_txn.open_table(NAME_INDEX_TABLE)?;
            let mut file_list_table = write_txn.open_table(FILE_LIST_TABLE)?;
            
            // Collect keys first, then delete (redb doesn't have drain)
            let cache_keys: Vec<String> = cache_table.iter()?.filter_map(|r| r.ok().map(|(k, _)| k.value().to_string())).collect();
            let index_keys: Vec<String> = index_table.iter()?.filter_map(|r| r.ok().map(|(k, _)| k.value().to_string())).collect();
            let file_keys: Vec<String> = file_list_table.iter()?.filter_map(|r| r.ok().map(|(k, _)| k.value().to_string())).collect();
            
            for key in cache_keys {
                cache_table.remove(key.as_str())?;
            }
            for key in index_keys {
                index_table.remove(key.as_str())?;
            }
            for key in file_keys {
                file_list_table.remove(key.as_str())?;
            }
        }
        write_txn.commit()?;
        Ok(())
    }
    
    /// Clear only the file list (forces directory rescan but keeps parsed apps)
    pub fn clear_file_list(&self) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(FILE_LIST_TABLE)?;
            
            // Collect keys first, then delete
            let keys: Vec<String> = table.iter()?.filter_map(|r| r.ok().map(|(k, _)| k.value().to_string())).collect();
            for key in keys {
                table.remove(key.as_str())?;
            }
        }
        write_txn.commit()?;
        Ok(())
    }
}

/// Batch history and pinned data loader
pub struct HistoryCache {
    pub history: HashMap<String, u64>,
    pub pinned: std::collections::HashSet<String>,
}

impl HistoryCache {
    /// Load all history and pinned data at once
    pub fn load(db: &std::sync::Arc<Database>) -> Result<Self> {
        let mut history = HashMap::new();
        let mut pinned = std::collections::HashSet::new();
        
        let read_txn = db.begin_read()?;
        
        // Load all history entries (table might not exist yet)
        if let Ok(history_table) = read_txn.open_table(HISTORY_TABLE) {
            for item in history_table.iter()? {
                if let Ok((key, value)) = item {
                    history.insert(key.value().to_string(), value.value());
                }
            }
        }
        
        // Load pinned apps (table might not exist yet)
        if let Ok(pinned_table) = read_txn.open_table(PINNED_TABLE) {
            if let Some(data) = pinned_table.get("pinned_apps")? {
                if let Ok(apps) = bincode::deserialize::<Vec<String>>(data.value()) {
                    pinned.extend(apps);
                }
            }
        }
        
        Ok(Self { history, pinned })
    }
    
    /// Get history count for an app
    pub fn get_history(&self, app_name: &str) -> u64 {
        self.history.get(app_name).copied().unwrap_or(0)
    }
    
    /// Check if app is pinned
    pub fn is_pinned(&self, app_name: &str) -> bool {
        self.pinned.contains(app_name)
    }
    
    /// Apply history and pinned status to an app
    pub fn apply_to_app(&self, mut app: App) -> App {
        app.history = self.get_history(&app.name);
        app.pinned = self.is_pinned(&app.name);
        app
    }
    
    /// Get the most frequently used app matching a prefix
    /// Returns (app_name, count) for the best match
    pub fn get_best_match(&self, prefix: &str) -> Option<(&String, u64)> {
        let prefix_lower = prefix.to_lowercase();
        
        self.history
            .iter()
            .filter(|(name, _)| name.to_lowercase().starts_with(&prefix_lower))
            .max_by_key(|(_, count)| *count)
            .map(|(name, count)| (name, *count))
    }
}
