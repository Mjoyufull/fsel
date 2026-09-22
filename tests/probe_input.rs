//! Controlling-TTY tests: real pipeline stdin/stdout plus simulated terminal replies.
#![cfg(target_os = "linux")]
#![allow(unsafe_code)]

use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct Session {
    child: Child,
    master: File,
    slave: File,
    original: libc::termios,
    screen: Vec<u8>,
}

impl Session {
    fn start() -> Self {
        let size = libc::winsize {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 720,
            ws_ypixel: 432,
        };
        let (mut master, mut slave) = (-1, -1);
        // SAFETY: output descriptors and size point to live storage; null requests default termios.
        assert_eq!(
            unsafe {
                libc::openpty(
                    &mut master,
                    &mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    &size,
                )
            },
            0
        );
        // SAFETY: openpty returned new descriptors, each transferred to exactly one owner.
        let (master, slave) = unsafe { (File::from_raw_fd(master), File::from_raw_fd(slave)) };
        // SAFETY: master is open and only this fixture changes its flags.
        assert_ne!(
            unsafe { libc::fcntl(master.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) },
            -1
        );
        let original = attributes(&slave);
        let mut command = Command::new(env!("CARGO_BIN_EXE_fsel"));
        command
            .args(["--config", "/dev/null", "--preview", "printf preview"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(slave.try_clone().unwrap())
            .env("TERM", "alacritty")
            .env("XDG_CONFIG_HOME", "/nonexistent/fsel-probe-config");
        for key in [
            "KITTY_WINDOW_ID",
            "WEZTERM_EXECUTABLE",
            "KONSOLE_VERSION",
            "TMUX",
        ] {
            command.env_remove(key);
        }
        // SAFETY: only async-signal-safe syscalls run between fork and exec. Stderr is the PTY slave.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 || libc::ioctl(2, libc::TIOCSCTTY, 0) == -1 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command.spawn().unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all("alpha\néclair\nfirefox\n".as_bytes())
            .unwrap();
        let mut session = Self {
            child,
            master,
            slave,
            original,
            screen: Vec::new(),
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        while !session.screen.windows(4).any(|bytes| bytes == b"\x1b[5n") {
            assert!(Instant::now() < deadline, "terminal probe never appeared");
            assert!(
                session.child.try_wait().unwrap().is_none(),
                "launcher exited before probing"
            );
            session.drain();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            session.screen.windows(6).any(|bytes| bytes == b"Filter"),
            "initial frame precedes query"
        );
        session
    }

    fn drain(&mut self) {
        let mut bytes = [0; 16384];
        if let Ok(count) = self.master.read(&mut bytes) {
            self.screen.extend_from_slice(&bytes[..count]);
        }
    }

    fn finish(mut self, expected: &[u8]) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            self.drain();
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            assert!(Instant::now() < deadline, "launcher failed to exit");
            std::thread::sleep(Duration::from_millis(5));
        }
        let mut output = Vec::new();
        self.child
            .stdout
            .take()
            .unwrap()
            .read_to_end(&mut output)
            .unwrap();
        assert_eq!(output, expected);
        let restored = attributes(&self.slave);
        assert_eq!(
            restored.c_lflag, self.original.c_lflag,
            "raw mode was restored"
        );
        assert_eq!(restored.c_iflag, self.original.c_iflag);
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn attributes(file: &File) -> libc::termios {
    let mut value = std::mem::MaybeUninit::uninit();
    // SAFETY: tcgetattr initializes value on success, and file remains open.
    assert_eq!(
        unsafe { libc::tcgetattr(file.as_raw_fd(), value.as_mut_ptr()) },
        0
    );
    // SAFETY: successful tcgetattr initialized the structure.
    unsafe { value.assume_init() }
}

const RESPONSE: &[u8] = b"\x1b[?62;4c\x1b[6;18;9t\x1b_Gi=31;ENOENT\x1b\\\x1b[0n";

#[test]
fn early_text_backspace_and_trailing_enter_survive_probe() {
    let mut session = Session::start();
    session
        .master
        .write_all(&[b"xx\x7f\x7ff".as_slice(), RESPONSE, b"\r"].concat())
        .unwrap();
    session.finish(b"firefox\n");
}

#[test]
fn early_arrow_keys_are_decoded_by_crossterm() {
    let mut session = Session::start();
    session
        .master
        .write_all(&[b"\x1b[B".as_slice(), RESPONSE, b"\r"].concat())
        .unwrap();
    session.finish("éclair\n".as_bytes());
}

#[test]
fn split_utf8_across_probe_handoff_is_preserved() {
    let mut session = Session::start();
    session
        .master
        .write_all(&[b"\xc3".as_slice(), RESPONSE].concat())
        .unwrap();
    std::thread::sleep(Duration::from_millis(100));
    session.master.write_all(b"\xa9\r").unwrap();
    session.finish("é\n".as_bytes());
}

#[test]
fn resizing_after_probe_keeps_input_and_shutdown_working() {
    let mut session = Session::start();
    session.master.write_all(RESPONSE).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    let size = libc::winsize {
        ws_row: 32,
        ws_col: 100,
        ws_xpixel: 900,
        ws_ypixel: 576,
    };
    // SAFETY: the descriptor is live and size is initialized for this ioctl.
    assert_eq!(
        unsafe { libc::ioctl(session.slave.as_raw_fd(), libc::TIOCSWINSZ, &size) },
        0
    );
    std::thread::sleep(Duration::from_millis(100));
    session.master.write_all(b"f\r").unwrap();
    session.finish(b"firefox\n");
}

#[test]
fn silent_terminal_preserves_input_and_escape_restores_tty() {
    let mut session = Session::start();
    session.master.write_all(b"f\r").unwrap();
    session.finish(b"firefox\n");
    let mut session = Session::start();
    session
        .master
        .write_all(&[RESPONSE, b"\x1b"].concat())
        .unwrap();
    session.finish(b"");
}
