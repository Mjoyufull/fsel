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
            println!("ID\tHIDDEN_MS\tNAME\tSOURCE");
            for entry in entries {
                println!(
                    "{}\t{}\t{}\t{}",
                    entry.id().value(),
                    entry.hidden_at_unix_ms(),
                    sanitize_table_field(entry.display_name()),
                    sanitize_table_field(entry.source_display())
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

pub(crate) fn log_startup_if_enabled(cli: &Opts, app_count: usize, frecency_entries: usize) {
    if crate::cli::DEBUG_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        crate::core::debug_logger::log_startup_info(cli, app_count, frecency_entries);
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
    use super::sanitize_table_field;

    #[test]
    fn hidden_entry_table_fields_do_not_emit_terminal_controls() {
        assert_eq!(
            sanitize_table_field("Unsafe\n\u{1b}[31mName"),
            "Unsafe  [31mName"
        );
    }
}
