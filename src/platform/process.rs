//! Process-management helpers kept behind an explicit platform boundary.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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

pub(crate) fn process_matches_current_exe(pid: i32) -> io::Result<bool> {
    let current_exe = canonical_path(std::env::current_exe()?);
    let process_exe = canonical_path(fs::read_link(format!("/proc/{pid}/exe"))?);
    Ok(process_exe == current_exe)
}

pub(crate) fn process_has_argument(pid: i32, expected: &str) -> io::Result<bool> {
    let raw = fs::read(format!("/proc/{pid}/cmdline"))?;
    Ok(raw
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
        .filter_map(|part| std::str::from_utf8(part).ok())
        .any(|argument| argument == expected))
}

pub(crate) fn find_processes_holding_file(path: &Path) -> io::Result<Vec<i32>> {
    let mut holders = Vec::new();

    if !path.exists() {
        return Ok(holders);
    }

    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
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

            if target_matches_path(&target, &canonical) {
                holders.push(pid);
                break;
            }
        }
    }

    Ok(holders)
}

fn canonical_path(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

fn target_matches_path(target: &Path, canonical: &Path) -> bool {
    if target == canonical {
        return true;
    }

    let Some(target_str) = target.to_str() else {
        return false;
    };
    let Some(canonical_str) = canonical.to_str() else {
        return false;
    };

    target_str
        .strip_suffix(" (deleted)")
        .is_some_and(|trimmed| trimmed == canonical_str)
}
