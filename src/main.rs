#![deny(unsafe_code)]
#![deny(missing_docs)]

//! # Fsel
//!
//! > _Blazing fast_ TUI launcher for GNU/Linux and *BSD
//!
//! For more info, check the [README](https://github.com/Mjoyufull/fsel)

mod cli;
mod common;
mod core;
mod desktop;
mod modes;
mod process;
mod strings;
mod ui;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use directories::ProjectDirs;
use eyre::{eyre, WrapErr};
use std::{fs, io, path};

fn main() {
    if let Err(error) = run() {
        let _ = shutdown_terminal(false);
        eprintln!("{error:?}");
        std::process::exit(1);
    }
}

fn run() -> eyre::Result<()> {
    let cli = cli::parse()?;

    // Route to appropriate mode
    if cli.dmenu_mode {
        return modes::dmenu::run(&cli);
    }

    if cli.cclip_mode {
        return run_cclip_mode(&cli);
    }

    // Default: app launcher
    modes::app_launcher::run(cli)
}

fn run_cclip_mode(cli: &cli::Opts) -> eyre::Result<()> {
    // Check if cclip is available
    if !modes::cclip::check_cclip_available() {
        eprintln!("error: cclip is not installed or not in PATH");
        eprintln!("install cclip from: https://github.com/heather7283/cclip");
        std::process::exit(1);
    }

    // Check if cclipd is running and has data
    if let Err(e) = modes::cclip::check_cclip_database() {
        eprintln!("error: {}", e);
        eprintln!("\nto use cclip mode, you need to:");
        eprintln!("1. start cclipd daemon:");
        eprintln!("   cclipd -s 2 -t \"image/png\" -t \"image/*\" -t \"text/plain;charset=utf-8\" -t \"text/*\" -t \"*\"");
        eprintln!("2. copy some stuff to build up history");
        eprintln!("\nfor more info: https://github.com/heather7283/cclip");
        std::process::exit(1);
    }

    // Handle lock file for cclip mode
    let lock_path =
        if let Some(project_dirs) = ProjectDirs::from("ch", "forkbomb9", env!("CARGO_PKG_NAME")) {
            let mut cache_dir = project_dirs.cache_dir().to_path_buf();
            if !cache_dir.exists() {
                fs::create_dir_all(&cache_dir)?;
            }
            cache_dir.push("fsel-cclip.lock");
            cache_dir
        } else {
            return Err(eyre!("can't find cache dir for {}", env!("CARGO_PKG_NAME")));
        };

    let contents = match fs::read_to_string(&lock_path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Ok(c) => c,
        Err(e) => {
            return Err(e).wrap_err("Failed to read cclip lockfile");
        }
    };

    if !contents.is_empty() {
        if cli.replace {
            let pid: i32 = contents
                .parse()
                .wrap_err("Failed to parse cclip lockfile contents")?;
            process::kill_process_sigterm(pid);
            fs::remove_file(&lock_path)?;
            std::thread::sleep(std::time::Duration::from_millis(200));
        } else {
            return Err(eyre!("Fsel cclip mode is already running"));
        }
    }

    // Write current pid to lock file
    let mut lock_file = fs::File::create(&lock_path)?;
    let pid = process::get_current_pid();
    use std::io::Write;
    lock_file.write_all(pid.to_string().as_bytes())?;

    // Lock file cleanup guard
    struct CclipLockGuard(path::PathBuf);
    impl Drop for CclipLockGuard {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }
    let _cclip_lock_guard = CclipLockGuard(lock_path);

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(modes::cclip::run(cli))
}

/// Setup terminal for TUI mode
pub fn setup_terminal(disable_mouse: bool) -> eyre::Result<()> {
    enable_raw_mode().wrap_err("Failed to enable raw mode")?;
    io::stderr()
        .execute(EnterAlternateScreen)
        .wrap_err("Failed to enter alternate screen")?;
    if !disable_mouse {
        io::stderr()
            .execute(EnableMouseCapture)
            .wrap_err("Failed to enable mouse capture")?;
    }
    Ok(())
}

/// Shutdown terminal and restore normal mode
pub fn shutdown_terminal(disable_mouse: bool) -> eyre::Result<()> {
    if !disable_mouse {
        io::stderr().execute(DisableMouseCapture)?;
    }
    io::stderr().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

/// Find processes holding a file open (for database locking)
pub fn find_processes_holding_file(path: &path::Path) -> io::Result<Vec<i32>> {
    let mut holders = Vec::new();

    if !path.exists() {
        return Ok(holders);
    }

    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let canonical_str = canonical.to_string_lossy();

    let proc_entries = match fs::read_dir("/proc") {
        Ok(entries) => entries,
        Err(_) => return Ok(holders),
    };

    for entry in proc_entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let file_name = entry.file_name();
        let pid: i32 = match file_name.to_str().and_then(|s| s.parse().ok()) {
            Some(pid) => pid,
            None => continue,
        };

        let fd_dir = entry.path().join("fd");
        let fd_entries = match fs::read_dir(fd_dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for fd_entry in fd_entries {
            let fd_entry = match fd_entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let target = match fs::read_link(fd_entry.path()) {
                Ok(t) => t,
                Err(_) => continue,
            };

            if target == canonical {
                holders.push(pid);
                break;
            }

            if let Some(target_str) = target.to_str() {
                if target_str.starts_with(canonical_str.as_ref()) {
                    holders.push(pid);
                    break;
                }
            }
        }
    }

    Ok(holders)
}
