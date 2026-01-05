// Process management utilities

use std::io;

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
/// Check if a process exists
#[allow(unsafe_code)]
pub fn process_exists(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}
