use crate::cli::Opts;
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
        println!("Database cleared successfully!");
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

    Ok(false)
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
