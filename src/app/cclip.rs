use directories::ProjectDirs;
use eyre::{WrapErr, eyre};
use std::{fs, io, path};

use crate::cli;
use crate::modes;
use crate::process;

pub(crate) fn run(cli: &cli::Opts) -> eyre::Result<()> {
    if !modes::cclip::check_cclip_available() {
        eprintln!("error: cclip is not installed or not in PATH");
        eprintln!("install cclip from: https://github.com/heather7283/cclip");
        std::process::exit(1);
    }

    if let Err(e) = modes::cclip::check_cclip_database() {
        eprintln!("error: {}", e);
        eprintln!("\nto use cclip mode, you need to:");
        eprintln!("1. start cclipd daemon:");
        eprintln!(
            "   cclipd -s 2 -t \"image/png\" -t \"image/*\" -t \"text/plain;charset=utf-8\" -t \"text/*\" -t \"*\""
        );
        eprintln!("2. copy some stuff to build up history");
        eprintln!("\nfor more info: https://github.com/heather7283/cclip");
        std::process::exit(1);
    }

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

    let is_non_interactive = cli.cclip_clear_tags || cli.cclip_tag_list || cli.cclip_wipe_tags;

    if !contents.is_empty() && !is_non_interactive {
        if let Ok(pid) = contents.trim().parse::<i32>() {
            if !process::process_exists(pid) {
                if let Err(e) = fs::remove_file(&lock_path) {
                    eprintln!("Warning: Failed to remove stale lock file: {}", e);
                }
            } else if cli.replace {
                match process::kill_process_sigterm_result(pid) {
                    Ok(()) => {
                        fs::remove_file(&lock_path)?;
                    }
                    Err(e) if e.raw_os_error() == Some(libc::ESRCH) => {
                        fs::remove_file(&lock_path)?;
                    }
                    Err(e) => {
                        return Err(eyre!(
                            "Failed to kill existing fsel cclip process (pid {}): {}",
                            pid,
                            e
                        ));
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            } else {
                return Err(eyre!("Fsel cclip mode is already running"));
            }
        } else if let Err(e) = fs::remove_file(&lock_path) {
            eprintln!("Warning: Failed to remove corrupted lock file: {}", e);
        }
    }

    let _cclip_lock_guard = if !is_non_interactive {
        let mut lock_file = fs::File::create(&lock_path)?;
        let pid = process::get_current_pid();
        use std::io::Write;
        lock_file.write_all(pid.to_string().as_bytes())?;

        struct CclipLockGuard(path::PathBuf);
        impl Drop for CclipLockGuard {
            fn drop(&mut self) {
                let _ = fs::remove_file(&self.0);
            }
        }
        Some(CclipLockGuard(lock_path))
    } else {
        None
    };

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(modes::cclip::run(cli))
}
