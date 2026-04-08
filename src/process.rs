// Process management utilities

use std::fs;
use std::io;
use std::path;

/// Get current process ID
#[allow(unsafe_code)]
pub fn get_current_pid() -> i32 {
    unsafe { libc::getpid() }
}

/// Wrapper
/// Send SIGTERM to a process, ignore result
#[allow(unsafe_code, dead_code)]
pub fn kill_process_sigterm(pid: i32) {
    let _ = kill_process_sigterm_result(pid);
}

/// Send SIGTERM to a process
/// Lets SIGTERM fail with error code
/// Allows caller to handle error codes
#[allow(unsafe_code)]
pub fn kill_process_sigterm_result(pid: i32) -> io::Result<()> {
    let ret = unsafe { libc::kill(pid, libc::SIGTERM) };
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Send SIGKILL to a process and return any OS error to the caller.
#[allow(unsafe_code)]
pub fn kill_process_sigkill_result(pid: i32) -> io::Result<()> {
    let ret = unsafe { libc::kill(pid, libc::SIGKILL) };
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Check if a process exists
#[allow(unsafe_code)]
pub fn process_exists(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

pub(crate) fn find_processes_holding_file(path: &path::Path) -> io::Result<Vec<i32>> {
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
