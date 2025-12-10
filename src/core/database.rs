use directories::ProjectDirs;
use eyre::{eyre, Result, WrapErr};
use redb::{ReadableDatabase, ReadableTable};
use std::fs;
use std::path::PathBuf;

/// open the database, creating the directory if needed
/// returns the database and the data directory path
pub fn open_history_db() -> Result<(std::sync::Arc<redb::Database>, PathBuf)> {
    let project_dirs = ProjectDirs::from("ch", "forkbomb9", env!("CARGO_PKG_NAME"))
        .ok_or_else(|| eyre!("can't find data dir for {}", env!("CARGO_PKG_NAME")))?;

    let mut db_path = project_dirs.data_local_dir().to_path_buf();

    if !db_path.exists() {
        fs::create_dir_all(&db_path)?;
    }

    let data_dir = db_path.clone();
    db_path.push("hist_db.redb");

    let db = redb::Database::create(&db_path)
        .wrap_err_with(|| format!(
            "Failed to open database at {:?}. If you upgraded from an older version, delete the old database file: rm {:?}",
            db_path, db_path
        ))?;

    Ok((std::sync::Arc::new(db), data_dir))
}

/// load pinned apps from database
/// returns a set of app names that are pinned
pub fn load_pinned_apps(db: &std::sync::Arc<redb::Database>) -> std::collections::HashSet<String> {
    let mut pinned = std::collections::HashSet::new();

    match db.begin_read() {
        Ok(read_txn) => {
            match read_txn.open_table(crate::core::cache::PINNED_TABLE) {
                Ok(table) => {
                    match table.get("pinned_apps") {
                        Ok(Some(data)) => match bincode::deserialize::<Vec<String>>(data.value()) {
                            Ok(apps) => pinned.extend(apps),
                            Err(e) => {
                                eprintln!("Warning: Failed to deserialize pinned apps: {}", e)
                            }
                        },
                        Ok(None) => {} // No pinned apps yet
                        Err(e) => eprintln!("Warning: Failed to read pinned apps: {}", e),
                    }
                }
                Err(e) => eprintln!("Warning: Failed to open pinned table: {}", e),
            }
        }
        Err(e) => eprintln!("Warning: Failed to begin read transaction: {}", e),
    }

    pinned
}

/// save pinned apps to database
pub fn save_pinned_apps(
    db: &std::sync::Arc<redb::Database>,
    pinned: &std::collections::HashSet<String>,
) -> Result<()> {
    let apps: Vec<String> = pinned.iter().cloned().collect();
    let data = bincode::serialize(&apps)?;

    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(crate::core::cache::PINNED_TABLE)?;
        table.insert("pinned_apps", data.as_slice())?;
    }
    write_txn.commit()?;
    Ok(())
}

/// toggle pin status for an app
pub fn toggle_pin(db: &std::sync::Arc<redb::Database>, app_name: &str) -> Result<bool> {
    let mut pinned = load_pinned_apps(db);
    let is_pinned = if pinned.contains(app_name) {
        pinned.remove(app_name);
        false
    } else {
        pinned.insert(app_name.to_string());
        true
    };
    save_pinned_apps(db, &pinned)?;
    Ok(is_pinned)
}

// =============================================================================
// FRECENCY STORAGE
// =============================================================================

use crate::core::state::FrecencyEntry;
use std::collections::HashMap;

/// Load frecency data from database
pub fn load_frecency(db: &std::sync::Arc<redb::Database>) -> HashMap<String, FrecencyEntry> {
    let mut frecency = HashMap::new();

    match db.begin_read() {
        Ok(read_txn) => {
            if let Ok(table) = read_txn.open_table(crate::core::cache::FRECENCY_TABLE) {
                // Iterate over all entries
                if let Ok(iter) = table.iter() {
                    for (key, value) in iter.flatten() {
                        if let Ok(entry) = bincode::deserialize::<FrecencyEntry>(value.value()) {
                            frecency.insert(key.value().to_string(), entry);
                        }
                    }
                }
            }
        }
        Err(e) => eprintln!("Warning: Failed to begin read transaction for frecency: {}", e),
    }

    frecency
}

/// Save frecency data to database
pub fn save_frecency(
    db: &std::sync::Arc<redb::Database>,
    frecency: &HashMap<String, FrecencyEntry>,
) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(crate::core::cache::FRECENCY_TABLE)?;
        
        // Clear existing entries and write new ones
        // Note: In production, you might want to do incremental updates
        for (name, entry) in frecency {
            let data = bincode::serialize(entry)?;
            table.insert(name.as_str(), data.as_slice())?;
        }
    }
    write_txn.commit()?;
    Ok(())
}

/// Record an app access (updates frecency)
pub fn record_access(db: &std::sync::Arc<redb::Database>, app_name: &str) -> Result<()> {
    let mut frecency = load_frecency(db);
    
    // Update or create entry
    frecency
        .entry(app_name.to_string())
        .and_modify(|e| e.access())
        .or_default();
    
    // Age entries if total exceeds max (10000 by default, like zoxide)
    crate::core::state::age_entries(&mut frecency, 10000);
    
    save_frecency(db, &frecency)?;
    Ok(())
}

/// Get frecency score for an app
#[allow(dead_code)]
pub fn get_frecency_score(db: &std::sync::Arc<redb::Database>, app_name: &str) -> f64 {
    load_frecency(db)
        .get(app_name)
        .map(|e| e.frecency())
        .unwrap_or(0.0)
}
