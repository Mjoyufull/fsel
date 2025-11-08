// Process management utilities

/// Get current process ID
#[allow(unsafe_code)]
pub fn get_current_pid() -> i32 {
    unsafe { libc::getpid() }
}

/// Wrapper
/// Send SIGTERM to a process, ignore result
#[allow(unsafe_code)]
pub fn kill_process_sigterm(pid: i32) {
    let _ = kill_process_sigterm_result(pid);
}

/// Send SIGTERM to a process
/// Lets SIGTERM fail with error code
/// Allows caller to handle error codes

#[allow(unsafe_code)]
pub fn kill_process_sigterm_result(pid: i32) -> Result<(), i32> {
    let ret = unsafe { libc::kill(pid, libc::SIGTERM) };
    if ret == 0 {
        Ok(())
    } else {
        Err(unsafe { *libc::__errno_location()})
    }
}