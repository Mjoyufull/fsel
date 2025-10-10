/// Desktop file cache for fast startup
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use eyre::Result;
use serde::{Deserialize, Serialize};

use crate::xdg::App;

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
    tree: sled::Tree,
    name_index: sled::Tree,
    file_list: sled::Tree,
}

/// Cached directory listing
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileListCache {
    /// List of all desktop file paths
    paths: Vec<PathBuf>,
    /// Last time we scanned directories
    last_scan: SystemTime,
}

impl DesktopCache {
    /// Create a new cache with the given database
    /// Trees are opened once and reused
    pub fn new(db: sled::Db) -> Self {
        let tree = db.open_tree(b"desktop_cache").expect("Failed to open cache tree");
        let name_index = db.open_tree(b"app_name_index").expect("Failed to open name index tree");
        let file_list = db.open_tree(b"file_list_cache").expect("Failed to open file list tree");
        Self { tree, name_index, file_list }
    }
    

    
    /// Get cached file list (avoids directory walk)
    /// Returns None if cache is stale (older than 5 minutes)
    pub fn get_file_list(&self) -> Result<Option<Vec<PathBuf>>> {
        if let Some(data) = self.file_list.get(b"paths")? {
            if let Ok(cache) = bincode::deserialize::<FileListCache>(&data) {
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
    
    /// Store file list in cache
    pub fn set_file_list(&self, paths: Vec<PathBuf>) -> Result<()> {
        let cache = FileListCache {
            paths,
            last_scan: SystemTime::now(),
        };
        let data = bincode::serialize(&cache)?;
        self.file_list.insert(b"paths", data.as_slice())?;
        Ok(())
    }
    
    /// Get app by name (uses index for fast lookup)
    pub fn get_by_name(&self, app_name: &str) -> Result<Option<App>> {
        // Look up the path in the index
        if let Some(path_bytes) = self.name_index.get(app_name.as_bytes())? {
            let path_str = String::from_utf8_lossy(&path_bytes);
            let path = PathBuf::from(path_str.as_ref());
            
            // Get the cached app from the path
            return self.get(&path);
        }
        
        Ok(None)
    }
    
    /// Get cached app if file hasn't changed
    pub fn get(&self, path: &Path) -> Result<Option<App>> {
        let path_key = path.to_string_lossy().as_bytes().to_vec();
        
        if let Some(data) = self.tree.get(&path_key)? {
            if let Ok(entry) = bincode::deserialize::<CacheEntry>(&data) {
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
                
                let path_key = path.to_string_lossy().as_bytes().to_vec();
                let data = bincode::serialize(&entry)?;
                self.tree.insert(path_key, data.as_slice())?;
                
                // Update name index for fast lookup
                let path_str = path.to_string_lossy();
                self.name_index.insert(app.name.as_bytes(), path_str.as_bytes())?;
            }
        }
        
        Ok(())
    }
    
    /// Clear the entire cache, name index, and file list
    pub fn clear(&self) -> Result<()> {
        self.tree.clear()?;
        self.name_index.clear()?;
        self.file_list.clear()?;
        Ok(())
    }
    
    /// Clear only the file list (forces directory rescan but keeps parsed apps)
    pub fn clear_file_list(&self) -> Result<()> {
        self.file_list.clear()?;
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
    pub fn load(db: &sled::Db) -> Self {
        let mut history = HashMap::new();
        let mut pinned = std::collections::HashSet::new();
        
        // Load all history entries from the default tree only
        // Note: db.iter() only iterates the default tree, not named trees
        for item in db.iter() {
            if let Ok((key, value)) = item {
                let key_str = String::from_utf8_lossy(&key);
                
                // Skip special keys (pinned_apps is stored in default tree)
                if key_str == "pinned_apps" {
                    continue;
                }
                
                // Try to unpack as history count (8 bytes)
                if value.len() == 8 {
                    if let Ok(packed_bytes) = value.as_ref().try_into() {
                        let count = crate::bytes::unpack(packed_bytes);
                        history.insert(key_str.to_string(), count);
                    }
                }
            }
        }
        
        // Load pinned apps
        if let Ok(Some(data)) = db.get(b"pinned_apps") {
            if let Ok(apps) = bincode::deserialize::<Vec<String>>(&data) {
                pinned.extend(apps);
            }
        }
        
        Self { history, pinned }
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
