//! Process-management helpers kept behind an explicit platform boundary.

use std::fs;
use std::io;
use std::path::Path;

/// Returns the current process ID.
#[allow(unsafe_code)]
pub fn get_current_pid() -> i32 {
    // SAFETY: `getpid` has no preconditions and does not dereference pointers.
    unsafe { libc::getpid() }
}

/// Sends `SIGTERM` to a process and ignores the result.
#[allow(unsafe_code, dead_code)]
pub fn kill_process_sigterm(pid: i32) {
    let _ = kill_process_sigterm_result(pid);
}

/// Sends `SIGTERM` to a process and returns any OS error to the caller.
#[allow(unsafe_code)]
pub fn kill_process_sigterm_result(pid: i32) -> io::Result<()> {
    // SAFETY: `kill` is called with a plain PID and a fixed signal value.
    let ret = unsafe { libc::kill(pid, libc::SIGTERM) };
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Sends `SIGKILL` to a process and returns any OS error to the caller.
#[allow(unsafe_code)]
pub fn kill_process_sigkill_result(pid: i32) -> io::Result<()> {
    // SAFETY: `kill` is called with a plain PID and a fixed signal value.
    let ret = unsafe { libc::kill(pid, libc::SIGKILL) };
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Returns whether a process exists for the given PID.
#[allow(unsafe_code)]
pub fn process_exists(pid: i32) -> bool {
    // SAFETY: `kill(pid, 0)` is the standard existence probe and has no extra preconditions.
    unsafe { libc::kill(pid, 0) == 0 }
}

pub(crate) fn find_processes_holding_file(path: &Path) -> io::Result<Vec<i32>> {
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
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let pid = match entry
            .file_name()
            .to_str()
            .and_then(|value| value.parse().ok())
        {
            Some(pid) => pid,
            None => continue,
        };

        let fd_entries = match fs::read_dir(entry.path().join("fd")) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for fd_entry in fd_entries {
            let fd_entry = match fd_entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };

            let target = match fs::read_link(fd_entry.path()) {
                Ok(target) => target,
                Err(_) => continue,
            };

            if target == canonical {
                holders.push(pid);
                break;
            }

            if let Some(target_str) = target.to_str()
                && target_str.starts_with(canonical_str.as_ref())
            {
                holders.push(pid);
                break;
            }
        }
    }

    Ok(holders)
}
