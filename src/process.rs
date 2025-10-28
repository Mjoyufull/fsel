// Process management utilities

/// Get current process ID
#[allow(unsafe_code)]
pub fn get_current_pid() -> i32 {
    unsafe { libc::getpid() }
}

/// Send SIGTERM to a process
#[allow(unsafe_code)]
pub fn kill_process_sigterm(pid: i32) {
    unsafe {
        libc::kill(pid, libc::SIGTERM);
    }
}
