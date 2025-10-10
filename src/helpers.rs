// helper functions used across the codebase

use directories::ProjectDirs;
use eyre::{eyre, Result};
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;

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

/// open the history database, creating the directory if needed
/// returns the database and the data directory path
pub fn open_history_db() -> Result<(sled::Db, PathBuf)> {
    let project_dirs = ProjectDirs::from("ch", "forkbomb9", env!("CARGO_PKG_NAME"))
        .ok_or_else(|| eyre!("can't find data dir for {}", env!("CARGO_PKG_NAME")))?;
    
    let mut hist_db = project_dirs.data_local_dir().to_path_buf();
    
    if !hist_db.exists() {
        fs::create_dir_all(&hist_db)?;
    }
    
    let data_dir = hist_db.clone();
    hist_db.push("hist_db");
    
    let db = sled::open(&hist_db)?;
    
    Ok((db, data_dir))
}

/// load pinned apps from database
/// returns a set of app names that are pinned
pub fn load_pinned_apps(db: &sled::Db) -> std::collections::HashSet<String> {
    let mut pinned = std::collections::HashSet::new();
    
    if let Ok(Some(data)) = db.get(b"pinned_apps") {
        if let Ok(apps) = bincode::deserialize::<Vec<String>>(&data) {
            pinned.extend(apps);
        }
    }
    
    pinned
}

/// save pinned apps to database
pub fn save_pinned_apps(db: &sled::Db, pinned: &std::collections::HashSet<String>) -> Result<()> {
    let apps: Vec<String> = pinned.iter().cloned().collect();
    let data = bincode::serialize(&apps)?;
    db.insert(b"pinned_apps", data.as_slice())?;
    Ok(())
}

/// toggle pin status for an app
pub fn toggle_pin(db: &sled::Db, app_name: &str) -> Result<bool> {
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
    db: &sled::Db,
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
    let packed = crate::bytes::pack(value);
    db.insert(app.name.as_bytes(), &packed)?;
    
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
