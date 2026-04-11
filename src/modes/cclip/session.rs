use eyre::{Result, WrapErr, eyre};
use std::fs;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};

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

struct CclipLockGuard(PathBuf);

impl Drop for CclipLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn ensure_single_cclip_instance(lock_path: &Path, replace: bool) -> Result<CclipLockGuard> {
    let lock_contents = read_lock_contents(lock_path)?;
    if lock_contents.is_empty() {
        write_current_pid_lockfile(lock_path)?;
        return Ok(CclipLockGuard(lock_path.to_path_buf()));
    }

    if let Ok(pid) = lock_contents.trim().parse::<i32>() {
        if !crate::platform::process::process_exists(pid) {
            remove_existing_lockfile(lock_path)?;
            write_current_pid_lockfile(lock_path)?;
            return Ok(CclipLockGuard(lock_path.to_path_buf()));
        }

        if !replace {
            return Err(eyre!("Fsel cclip mode is already running"));
        }

        match crate::platform::process::kill_process_sigterm_result(pid) {
            Ok(()) => remove_existing_lockfile(lock_path)?,
            Err(error) if error.raw_os_error() == Some(libc::ESRCH) => {
                remove_existing_lockfile(lock_path)?
            }
            Err(error) => {
                return Err(eyre!(
                    "Failed to kill existing fsel cclip process (pid {}): {}",
                    pid,
                    error
                ));
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
        write_current_pid_lockfile(lock_path)?;
        return Ok(CclipLockGuard(lock_path.to_path_buf()));
    }

    remove_existing_lockfile(lock_path)?;
    write_current_pid_lockfile(lock_path)?;
    Ok(CclipLockGuard(lock_path.to_path_buf()))
}

fn read_lock_contents(lock_path: &Path) -> Result<String> {
    match fs::read_to_string(lock_path) {
        Ok(contents) => Ok(contents),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error).wrap_err("Failed to read cclip lockfile"),
    }
}

fn remove_existing_lockfile(lock_path: &Path) -> Result<()> {
    if let Err(error) = fs::remove_file(lock_path)
        && error.kind() != io::ErrorKind::NotFound
    {
        return Err(error).wrap_err("Failed to remove cclip lockfile");
    }

    Ok(())
}

fn write_current_pid_lockfile(lock_path: &Path) -> Result<()> {
    let mut lock_file = fs::File::create(lock_path)?;
    let pid = crate::platform::process::get_current_pid();
    lock_file.write_all(pid.to_string().as_bytes())?;
    Ok(())
}
