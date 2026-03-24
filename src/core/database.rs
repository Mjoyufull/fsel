use eyre::{Result, WrapErr};
use redb::{ReadableDatabase, ReadableTable};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::SystemTime;

/// open the database, creating the directory if needed
/// returns the database and the data directory path
pub fn open_history_db() -> Result<(std::sync::Arc<redb::Database>, PathBuf)> {
    let data_dir = crate::app::paths::runtime_data_dir()?;
    let db_path = crate::app::paths::history_db_path()?;

    let db = redb::Database::create(&db_path)
        .wrap_err_with(|| format!(
            "Failed to open database at {:?}. If you upgraded from an older version, delete the old database file: rm {:?}",
            db_path, db_path
        ))?;

    Ok((std::sync::Arc::new(db), data_dir))
}

/// load pinned apps from database
/// returns a set of app names that are pinned
const PINNED_APPS_KEY: &str = "pinned_apps";
const PINNED_TIMESTAMPS_KEY: &str = "pin_timestamps";

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn save_pinned_state(
    db: &std::sync::Arc<redb::Database>,
    pinned: &HashSet<String>,
    pin_timestamps: &HashMap<String, u64>,
) -> Result<()> {
    let mut apps: Vec<String> = pinned.iter().cloned().collect();
    apps.sort_unstable_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()).then(a.cmp(b)));

    let apps_data = postcard::to_allocvec(&apps)?;
    let timestamps_data = postcard::to_allocvec(pin_timestamps)?;

    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(crate::core::cache::PINNED_TABLE)?;
        table.insert(PINNED_APPS_KEY, apps_data.as_slice())?;
        table.insert(PINNED_TIMESTAMPS_KEY, timestamps_data.as_slice())?;
    }
    write_txn.commit()?;
    Ok(())
}

fn load_pinned_apps_internal(db: &std::sync::Arc<redb::Database>) -> (HashSet<String>, bool) {
    let mut pinned = HashSet::new();
    let mut loaded_ok = true;

    match db.begin_read() {
        Ok(read_txn) => match read_txn.open_table(crate::core::cache::PINNED_TABLE) {
            Ok(table) => match table.get(PINNED_APPS_KEY) {
                Ok(Some(data)) => match postcard::from_bytes::<Vec<String>>(data.value()) {
                    Ok(apps) => pinned.extend(apps),
                    Err(e) => {
                        eprintln!("Warning: Failed to deserialize pinned apps: {}", e);
                        loaded_ok = false;
                    }
                },
                Ok(None) => {} // No pinned apps yet
                Err(e) => {
                    eprintln!("Warning: Failed to read pinned apps: {}", e);
                    loaded_ok = false;
                }
            },
            Err(e) => {
                eprintln!("Warning: Failed to open pinned table: {}", e);
                loaded_ok = false;
            }
        },
        Err(e) => {
            eprintln!("Warning: Failed to begin read transaction: {}", e);
            loaded_ok = false;
        }
    }

    (pinned, loaded_ok)
}

pub fn load_pinned_apps(db: &std::sync::Arc<redb::Database>) -> HashSet<String> {
    let (pinned, _loaded_ok) = load_pinned_apps_internal(db);
    pinned
}

/// load pin timestamps from database
/// returns app_name -> first pinned unix timestamp
pub fn load_pin_timestamps(db: &std::sync::Arc<redb::Database>) -> HashMap<String, u64> {
    let (pinned, pinned_loaded_ok) = load_pinned_apps_internal(db);
    let mut pin_timestamps = HashMap::new();
    let mut timestamps_loaded_ok = true;

    match db.begin_read() {
        Ok(read_txn) => {
            match read_txn.open_table(crate::core::cache::PINNED_TABLE) {
                Ok(table) => match table.get(PINNED_TIMESTAMPS_KEY) {
                    Ok(Some(data)) => {
                        if let Ok(map) = postcard::from_bytes::<HashMap<String, u64>>(data.value())
                        {
                            pin_timestamps = map;
                        } else {
                            eprintln!("Warning: Failed to deserialize pin timestamps");
                            timestamps_loaded_ok = false;
                        }
                    }
                    Ok(None) => {} // No timestamps yet
                    Err(e) => {
                        eprintln!("Warning: Failed to read pin timestamps: {}", e);
                        timestamps_loaded_ok = false;
                    }
                },
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to open pinned table for pin timestamps: {}",
                        e
                    );
                    timestamps_loaded_ok = false;
                }
            }
        }
        Err(e) => {
            eprintln!(
                "Warning: Failed to begin read transaction for pin timestamps: {}",
                e
            );
            timestamps_loaded_ok = false;
        }
    }

    // Avoid writing any reconciled state when reads were not reliable.
    if !(pinned_loaded_ok && timestamps_loaded_ok) {
        return pin_timestamps;
    }

    let mut changed = false;

    let before_len = pin_timestamps.len();
    pin_timestamps.retain(|name, _| pinned.contains(name));
    if pin_timestamps.len() != before_len {
        changed = true;
    }

    let mut missing: Vec<String> = pinned
        .iter()
        .filter(|name| !pin_timestamps.contains_key(*name))
        .cloned()
        .collect();

    if !missing.is_empty() {
        missing.sort_unstable_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()).then(a.cmp(b)));

        let mut next_ts = pin_timestamps
            .values()
            .copied()
            .max()
            .unwrap_or_else(now_unix_seconds);

        for app_name in missing {
            next_ts = next_ts.saturating_add(1);
            pin_timestamps.insert(app_name, next_ts);
            changed = true;
        }
    }

    if changed && let Err(e) = save_pinned_state(db, &pinned, &pin_timestamps) {
        eprintln!("Warning: Failed to persist pin timestamps: {}", e);
    }

    pin_timestamps
}

/// toggle pin status for an app
pub fn toggle_pin(db: &std::sync::Arc<redb::Database>, app_name: &str) -> Result<bool> {
    let mut pinned = load_pinned_apps(db);
    let mut pin_timestamps = load_pin_timestamps(db);
    let is_pinned = if pinned.contains(app_name) {
        pinned.remove(app_name);
        pin_timestamps.remove(app_name);
        false
    } else {
        pinned.insert(app_name.to_string());
        pin_timestamps.insert(app_name.to_string(), now_unix_seconds());
        true
    };
    save_pinned_state(db, &pinned, &pin_timestamps)?;
    Ok(is_pinned)
}

// =============================================================================
// FRECENCY STORAGE
// =============================================================================

use crate::core::state::FrecencyEntry;

/// Load frecency data from database
pub fn load_frecency(db: &std::sync::Arc<redb::Database>) -> HashMap<String, FrecencyEntry> {
    let mut frecency = HashMap::new();

    match db.begin_read() {
        Ok(read_txn) => {
            if let Ok(table) = read_txn.open_table(crate::core::cache::FRECENCY_TABLE) {
                // Iterate over all entries
                if let Ok(iter) = table.iter() {
                    for (key, value) in iter.flatten() {
                        if let Ok(entry) = postcard::from_bytes::<FrecencyEntry>(value.value()) {
                            frecency.insert(key.value().to_string(), entry);
                        }
                    }
                }
            }
        }
        Err(e) => eprintln!(
            "Warning: Failed to begin read transaction for frecency: {}",
            e
        ),
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
            let data = postcard::to_allocvec(entry)?;
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
