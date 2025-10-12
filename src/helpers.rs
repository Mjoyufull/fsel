// helper functions used across the codebase

use directories::ProjectDirs;
use eyre::{eyre, Result, WrapErr};
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use redb::ReadableDatabase;

/// Extract executable name from a command string
/// Takes the first word and strips any path components
/// 
/// Examples:
/// - "/usr/bin/firefox" -> "firefox"
/// - "firefox --new-window" -> "firefox"
/// - "env FOO=bar firefox" -> "env"
/// 
/// Optimized to avoid unnecessary allocations
#[inline]
pub fn extract_exec_name(command: &str) -> &str {
    command
        .split_whitespace()
        .next()
        .and_then(|cmd| cmd.rsplit('/').next())
        .unwrap_or("")
}

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
            match read_txn.open_table(crate::cache::PINNED_TABLE) {
                Ok(table) => {
                    match table.get("pinned_apps") {
                        Ok(Some(data)) => {
                            match bincode::deserialize::<Vec<String>>(data.value()) {
                                Ok(apps) => pinned.extend(apps),
                                Err(e) => eprintln!("Warning: Failed to deserialize pinned apps: {}", e),
                            }
                        }
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
pub fn save_pinned_apps(db: &std::sync::Arc<redb::Database>, pinned: &std::collections::HashSet<String>) -> Result<()> {
    let apps: Vec<String> = pinned.iter().cloned().collect();
    let data = bincode::serialize(&apps)?;
    
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(crate::cache::PINNED_TABLE)?;
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

/// launch an app with the specified configuration
pub fn launch_app(
    app: &crate::xdg::App,
    cli: &crate::cli::Opts,
    db: &std::sync::Arc<redb::Database>,
) -> Result<()> {
    use std::env;
    use std::process;
    
    let commands = shell_words::split(&app.command)?;
    
    if let Some(path) = &app.path {
        env::set_current_dir(std::path::PathBuf::from(path))?;
    }
    
    let mut runner: Vec<&str> = vec![];
    
    if cli.uwsm {
        runner.extend_from_slice(&["uwsm", "app", "--"]);
    } else if cli.systemd_run {
        runner.extend_from_slice(&["systemd-run", "--user", "--scope", "--"]);
    } else if cli.sway {
        runner.extend_from_slice(&["swaymsg", "exec", "--"]);
    }
    
    if app.is_terminal {
        runner.extend_from_slice(&cli.terminal_launcher.split(' ').collect::<Vec<&str>>());
    }
    
    runner.extend_from_slice(&commands.iter().map(AsRef::as_ref).collect::<Vec<&str>>());
    
    let mut exec = process::Command::new(runner[0]);
    exec.args(&runner[1..]);
    
    #[allow(unsafe_code)]
    unsafe {
        exec.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    
    if cli.verbose.unwrap_or(0) > 0 {
        exec.stdin(process::Stdio::null())
            .stdout(process::Stdio::null())
            .stderr(process::Stdio::null())
            .spawn()?;
    } else {
        exec.spawn()?;
    }
    
    // update history
    let value = app.history + 1;
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(crate::cache::HISTORY_TABLE)?;
        table.insert(app.name.as_str(), value)?;
    }
    write_txn.commit()?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_exec_name() {
        assert_eq!(extract_exec_name("/usr/bin/firefox"), "firefox");
        assert_eq!(extract_exec_name("firefox --new-window"), "firefox");
        assert_eq!(extract_exec_name("env FOO=bar firefox"), "env");
        assert_eq!(extract_exec_name(""), "");
        assert_eq!(extract_exec_name("   "), "");
    }
}
