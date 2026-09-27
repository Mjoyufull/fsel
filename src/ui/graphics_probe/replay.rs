//! Replay probe-time input through a raw PTY so Crossterm remains the only input decoder.
//! Stdin's pipeline has already been consumed. Stdout/stderr and the controlling TTY are unchanged.

use super::filter::ReplyFilter;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::OpenOptionsExt;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread::JoinHandle;

pub(crate) struct InputReplay {
    original: OwnedFd,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl InputReplay {
    pub(super) fn start(input: Vec<u8>, filter: ReplyFilter) -> io::Result<Self> {
        let tty = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open("/dev/tty")?;
        let (master, slave, size) = raw_pty(&tty)?;
        let original = duplicate_stdin()?;
        replace_stdin(slave.as_raw_fd())?;
        let mut replay = Self {
            original,
            stop: Arc::new(AtomicBool::new(false)),
            worker: None,
        };
        let stop = Arc::clone(&replay.stop);
        replay.worker = Some(
            std::thread::Builder::new()
                .name("fsel-input-replay".into())
                .spawn(move || pump(tty, master, input, filter, size, stop))?,
        );
        Ok(replay)
    }
}

impl Drop for InputReplay {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let _ = replace_stdin(self.original.as_raw_fd());
    }
}

fn pump(
    mut tty: File,
    mut master: File,
    mut input: Vec<u8>,
    mut filter: ReplyFilter,
    mut size: libc::winsize,
    stop: Arc<AtomicBool>,
) {
    let mut consumed = 0;
    while !stop.load(Ordering::Acquire) {
        if let Ok(current) = window_size(&tty)
            && (
                current.ws_row,
                current.ws_col,
                current.ws_xpixel,
                current.ws_ypixel,
            ) != (size.ws_row, size.ws_col, size.ws_xpixel, size.ws_ypixel)
            && set_window_size(&master, &current).is_ok()
        {
            size = current;
            // SAFETY: SIGWINCH has no payload and is sent only to this live process.
            #[allow(unsafe_code)]
            unsafe {
                libc::kill(libc::getpid(), libc::SIGWINCH);
            }
        }
        if consumed < input.len() {
            match master.write(&input[consumed..]) {
                Ok(0) => break,
                Ok(count) => {
                    consumed += count;
                    continue;
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) => {}
                Err(_) => break,
            }
        } else {
            input.clear();
            consumed = 0;
        }
        let mut fds = [
            libc::pollfd {
                fd: tty.as_raw_fd(),
                events: if input.is_empty() { libc::POLLIN } else { 0 },
                revents: 0,
            },
            libc::pollfd {
                fd: master.as_raw_fd(),
                events: if input.is_empty() { 0 } else { libc::POLLOUT },
                revents: 0,
            },
        ];
        // SAFETY: fds is a valid two-element array for the duration of this bounded poll.
        #[allow(unsafe_code)]
        let ready = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as _, 50) };
        if ready < 0 && io::Error::last_os_error().kind() != io::ErrorKind::Interrupted {
            break;
        }
        if fds
            .iter()
            .any(|fd| fd.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0)
        {
            break;
        }
        if fds[0].revents & libc::POLLIN != 0 {
            let mut bytes = [0u8; 4096];
            match tty.read(&mut bytes) {
                Ok(0) => break,
                Ok(count) => {
                    for byte in &bytes[..count] {
                        filter.push(*byte, &mut input);
                    }
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) => {}
                Err(_) => break,
            }
        } else if input.is_empty() {
            filter.release_ambiguous_escape(&mut input);
        }
    }
}

#[allow(unsafe_code)]
fn raw_pty(tty: &File) -> io::Result<(File, OwnedFd, libc::winsize)> {
    let size = window_size(tty)?;
    let mut attrs = std::mem::MaybeUninit::<libc::termios>::uninit();
    // SAFETY: tcgetattr initializes attrs on success, and both pointers passed to openpty
    // refer to live initialized values. Successful descriptors are immediately owned.
    unsafe {
        if libc::tcgetattr(tty.as_raw_fd(), attrs.as_mut_ptr()) == -1 {
            return Err(io::Error::last_os_error());
        }
        let mut attrs = attrs.assume_init();
        libc::cfmakeraw(&mut attrs);
        let (mut master, mut slave) = (-1, -1);
        if libc::openpty(&mut master, &mut slave, std::ptr::null_mut(), &attrs, &size) == -1 {
            return Err(io::Error::last_os_error());
        }
        let master = File::from_raw_fd(master);
        let slave = OwnedFd::from_raw_fd(slave);
        if libc::fcntl(master.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) == -1 {
            return Err(io::Error::last_os_error());
        }
        for fd in [master.as_raw_fd(), slave.as_raw_fd()] {
            if libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) == -1 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok((master, slave, size))
    }
}

#[allow(unsafe_code)]
fn duplicate_stdin() -> io::Result<OwnedFd> {
    // SAFETY: F_DUPFD_CLOEXEC duplicates a valid standard descriptor into a new owned descriptor.
    let fd = unsafe { libc::fcntl(libc::STDIN_FILENO, libc::F_DUPFD_CLOEXEC, 3) };
    if fd == -1 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the descriptor was just created and has no other owner.
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

#[allow(unsafe_code)]
fn replace_stdin(fd: libc::c_int) -> io::Result<()> {
    // SAFETY: fd is borrowed from a live owned descriptor; only stdin is replaced.
    if unsafe { libc::dup2(fd, libc::STDIN_FILENO) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[allow(unsafe_code)]
fn window_size(fd: &File) -> io::Result<libc::winsize> {
    let mut size = std::mem::MaybeUninit::uninit();
    // SAFETY: the ioctl initializes size on success and fd remains open.
    if unsafe { libc::ioctl(fd.as_raw_fd(), libc::TIOCGWINSZ, size.as_mut_ptr()) } == -1 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the successful ioctl initialized every field.
    Ok(unsafe { size.assume_init() })
}

#[allow(unsafe_code)]
fn set_window_size(fd: &File, size: &libc::winsize) -> io::Result<()> {
    // SAFETY: size is initialized, and the ioctl borrows it only for this call.
    if unsafe { libc::ioctl(fd.as_raw_fd(), libc::TIOCSWINSZ, size) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
