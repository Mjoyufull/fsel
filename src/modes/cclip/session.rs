use eyre::{Result, WrapErr, eyre};
use std::fs;
use std::io;
use std::io::Write;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const LOCK_ACQUIRE_ATTEMPTS: usize = 8;
const CHECK_INTERVAL_MS: u64 = 5;
const TOTAL_WAIT_MS: u64 = 500;

/// Active cclip session lock guard.
///
/// Keeps the lockfile alive for interactive cclip mode and removes it on drop.
pub(crate) struct CclipSession {
    _lock: CclipLockGuard,
}

impl CclipSession {
    /// Starts a cclip interactive session and acquires singleton lock.
    pub(crate) fn start(lock_path: &Path, replace: bool) -> Result<Self> {
        let lock = ensure_single_cclip_instance(lock_path, replace)?;
        Ok(Self { _lock: lock })
    }
}

struct CclipLockGuard {
    path: PathBuf,
    expected_contents: String,
}

impl Drop for CclipLockGuard {
    fn drop(&mut self) {
        let _ = remove_lockfile_if_unchanged(&self.path, &self.expected_contents);
    }
}

fn ensure_single_cclip_instance(lock_path: &Path, replace: bool) -> Result<CclipLockGuard> {
    let expected_contents = build_lock_contents("cclip")?;

    for _ in 0..LOCK_ACQUIRE_ATTEMPTS {
        if write_lockfile_if_absent(lock_path, &expected_contents)? {
            return Ok(CclipLockGuard {
                path: lock_path.to_path_buf(),
                expected_contents,
            });
        }

        let lock_contents = read_lock_contents(lock_path)?;
        if lock_contents.is_empty() {
            continue;
        }

        let Some(pid) = parse_lock_pid(&lock_contents) else {
            remove_lockfile_if_unchanged(lock_path, &lock_contents)?;
            continue;
        };

        if !is_active_cclip_owner(pid) {
            remove_lockfile_if_unchanged(lock_path, &lock_contents)?;
            continue;
        }

        if !replace {
            return Err(eyre!("Fsel cclip mode is already running"));
        }

        terminate_existing_cclip(pid)?;
        remove_lockfile_if_unchanged(lock_path, &lock_contents)?;
    }

    Err(eyre!(
        "Failed to acquire cclip lockfile due to concurrent startup activity"
    ))
}

fn read_lock_contents(lock_path: &Path) -> Result<String> {
    match fs::read_to_string(lock_path) {
        Ok(contents) => Ok(contents),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error).wrap_err("Failed to read cclip lockfile"),
    }
}

fn build_lock_contents(mode: &str) -> Result<String> {
    let pid = crate::platform::process::get_current_pid();
    let exe_path = std::env::current_exe()
        .wrap_err("Failed to resolve current executable for cclip lockfile")?
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_PKG_NAME")));
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    Ok(format!(
        "pid={pid}\nmode={mode}\nexe={}\nnonce={nonce}\n",
        exe_path.display()
    ))
}

fn write_lockfile_if_absent(lock_path: &Path, contents: &str) -> Result<bool> {
    let mut lock_file = match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(lock_path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => return Ok(false),
        Err(error) => return Err(error).wrap_err("Failed to create cclip lockfile"),
    };

    lock_file
        .write_all(contents.as_bytes())
        .wrap_err("Failed to write cclip lockfile")?;
    Ok(true)
}

fn parse_lock_pid(lock_contents: &str) -> Option<i32> {
    lock_contents
        .lines()
        .find_map(|line| line.strip_prefix("pid="))
        .unwrap_or(lock_contents.trim())
        .parse::<i32>()
        .ok()
}

fn is_active_cclip_owner(pid: i32) -> bool {
    crate::platform::process::process_exists(pid)
        && crate::platform::process::process_matches_current_exe(pid).unwrap_or(false)
        && crate::platform::process::process_has_argument(pid, "--cclip").unwrap_or(false)
}

fn terminate_existing_cclip(pid: i32) -> Result<()> {
    match crate::platform::process::kill_process_sigterm_result(pid) {
        Ok(()) => wait_for_process_exit(pid),
        Err(error) if error.raw_os_error() == Some(libc::ESRCH) => Ok(()),
        Err(error) => Err(eyre!(
            "Failed to kill existing fsel cclip process (pid {}): {}",
            pid,
            error
        )),
    }
}

fn wait_for_process_exit(pid: i32) -> Result<()> {
    let mut waited_ms = 0u64;

    while crate::platform::process::process_exists(pid) {
        if waited_ms >= TOTAL_WAIT_MS {
            return Err(eyre!("Existing fsel cclip process (pid {pid}) refused to exit"));
        }

        thread::sleep(Duration::from_millis(CHECK_INTERVAL_MS));
        waited_ms += CHECK_INTERVAL_MS;
    }

    Ok(())
}

fn remove_lockfile_if_unchanged(lock_path: &Path, expected_contents: &str) -> Result<bool> {
    match fs::read_to_string(lock_path) {
        Ok(current_contents) if current_contents == expected_contents => {
            fs::remove_file(lock_path).wrap_err("Failed to remove cclip lockfile")?;
            Ok(true)
        }
        Ok(_) => Ok(false),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).wrap_err("Failed to validate cclip lockfile ownership"),
    }
}
