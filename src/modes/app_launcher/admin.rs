use crate::cli::Opts;
use crate::core::hidden_entries::{HiddenEntryId, HiddenEntryStore};
use eyre::{Result, WrapErr};
use redb::ReadableTable;
use std::path::Path;
use std::sync::Arc;

pub(crate) fn handle_maintenance_command(
    cli: &Opts,
    db: &Arc<redb::Database>,
    data_dir: &Path,
) -> Result<bool> {
    if cli.clear_history {
        clear_history(db)?;
        println!("Launch history and pins cleared successfully!");
        println!(
            "To fully remove the database, delete {}",
            data_dir.display()
        );
        return Ok(true);
    }

    if cli.clear_cache {
        let cache = crate::core::cache::DesktopCache::new(Arc::clone(db))?;
        cache.clear().wrap_err("Error clearing cache")?;
        println!("Desktop file cache cleared successfully!");
        return Ok(true);
    }

    if cli.refresh_cache {
        let cache = crate::core::cache::DesktopCache::new(Arc::clone(db))?;
        cache.clear_file_list().wrap_err("Error refreshing cache")?;
        println!("Desktop file list refreshed - will rescan on next launch!");
        return Ok(true);
    }

    if cli.list_hidden {
        let store = HiddenEntryStore::new(Arc::clone(db))?;
        let entries = store.list()?;
        if entries.is_empty() {
            println!("No manually hidden entries.");
        } else {
            println!("ID\tHIDDEN\tNAME\tSOURCE");
            for entry in entries {
                let availability = if entry.source_is_available() == Some(false) {
                    " [unavailable]"
                } else {
                    ""
                };
                println!(
                    "{}\t{}\t{}\t{}{}",
                    entry.id().value(),
                    format_hidden_timestamp(entry.hidden_at_unix_ms()),
                    sanitize_table_field(entry.display_name()),
                    sanitize_table_field(entry.source_display()),
                    availability,
                );
            }
        }
        return Ok(true);
    }

    if let Some(id) = cli.unhide {
        let store = HiddenEntryStore::new(Arc::clone(db))?;
        let removed = store.remove(HiddenEntryId::new(id))?;
        let Some(entry) = removed else {
            return Err(eyre::eyre!("No manually hidden entry has ID {id}"));
        };
        println!("Restored {}", sanitize_table_field(entry.display_name()));
        return Ok(true);
    }

    if cli.unhide_all {
        let store = HiddenEntryStore::new(Arc::clone(db))?;
        let removed_count = store.remove_all()?;
        println!("Restored {removed_count} manually hidden entries.");
        return Ok(true);
    }

    Ok(false)
}

fn sanitize_table_field(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect()
}

fn format_hidden_timestamp(unix_ms: u64) -> String {
    let unix_nanos = i128::from(unix_ms) * 1_000_000;
    time::OffsetDateTime::from_unix_timestamp_nanos(unix_nanos)
        .ok()
        .and_then(|timestamp| {
            timestamp
                .format(&time::format_description::well_known::Rfc3339)
                .ok()
        })
        .unwrap_or_else(|| unix_ms.to_string())
}

pub(crate) fn initialize_test_mode(cli: &Opts) {
    if !cli.test_mode {
        return;
    }

    crate::cli::DEBUG_ENABLED.store(true, std::sync::atomic::Ordering::Relaxed);
    if let Err(error) = crate::core::debug_logger::init_test_log() {
        eprintln!("Warning: Failed to initialize debug logging: {}", error);
    } else {
        crate::core::debug_logger::log_event("App launcher started in test mode");
    }
}

pub(crate) fn log_startup_if_enabled(
    cli: &Opts,
    app_count: usize,
    frecency_entries: usize,
    hidden_summary: &crate::core::hidden_entries::HiddenSummary,
) {
    if crate::cli::DEBUG_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        crate::core::debug_logger::log_startup_info(
            cli,
            app_count,
            frecency_entries,
            hidden_summary,
        );
    }
}

fn clear_history(db: &redb::Database) -> Result<()> {
    let write_txn = db.begin_write().wrap_err("Error starting transaction")?;
    {
        let mut history_table = write_txn.open_table(crate::core::cache::HISTORY_TABLE)?;
        let mut pinned_table = write_txn.open_table(crate::core::cache::PINNED_TABLE)?;

        let history_keys: Vec<String> = history_table
            .iter()?
            .map(|result| result.map(|(key, _)| key.value().to_string()))
            .collect::<Result<_, _>>()?;
        let pinned_keys: Vec<String> = pinned_table
            .iter()?
            .map(|result| result.map(|(key, _)| key.value().to_string()))
            .collect::<Result<_, _>>()?;

        for key in history_keys {
            history_table.remove(key.as_str())?;
        }
        for key in pinned_keys {
            pinned_table.remove(key.as_str())?;
        }
    }
    write_txn.commit().wrap_err("Error clearing database")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{clear_history, format_hidden_timestamp, sanitize_table_field};
    use crate::core::hidden_entries::{EntryKey, HiddenEntryStore, NewHiddenEntry};
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;

    #[test]
    fn hidden_entry_table_fields_do_not_emit_terminal_controls() {
        assert_eq!(
            sanitize_table_field("Unsafe\n\u{1b}[31mName"),
            "Unsafe  [31mName"
        );
    }

    #[test]
    fn hidden_timestamps_are_human_readable() {
        assert_eq!(format_hidden_timestamp(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn clearing_history_and_cache_preserves_manual_hides() {
        let dir =
            std::env::temp_dir().join(format!("fsel-hidden-maintenance-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("test directory should be created");
        let db = Arc::new(
            redb::Database::create(dir.join("history.redb")).expect("database should be created"),
        );
        let store = HiddenEntryStore::new(Arc::clone(&db)).expect("store should initialize");
        store
            .insert(NewHiddenEntry::new(
                EntryKey::desktop(Path::new("/example.desktop"), "example.desktop"),
                "Example",
                "/example.desktop",
                0,
            ))
            .expect("record should be inserted");

        clear_history(&db).expect("history should clear");
        crate::core::cache::DesktopCache::new(Arc::clone(&db))
            .expect("cache should initialize")
            .clear()
            .expect("cache should clear");

        assert_eq!(store.list().expect("records should load").len(), 1);
        drop(store);
        drop(db);
        fs::remove_dir_all(dir).expect("test directory should be removed");
    }
}
