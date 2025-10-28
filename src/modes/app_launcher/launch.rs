// Application launching utilities

use eyre::Result;
use std::env;
use std::ffi::CString;
use std::io;
use std::process;

/// launch an app with the specified configuration
pub fn launch_app(
    app: &crate::desktop::App,
    cli: &crate::cli::Opts,
    db: &std::sync::Arc<redb::Database>,
) -> Result<()> {
    let commands = shell_words::split(&app.command)?;

    if let Some(path) = &app.path {
        env::set_current_dir(std::path::PathBuf::from(path))?;
    }

    let mut runner: Vec<&str> = vec![];

    if cli.uwsm {
        runner.insert(0, "uwsm");
        runner.insert(1, "app");
        runner.insert(2, "--");
    } else if cli.systemd_run {
        runner.insert(0, "systemd-run");
        runner.insert(1, "--user");
        runner.insert(2, "--scope");
    } else if cli.sway {
        runner.extend_from_slice(&["swaymsg", "exec", "--"]);
    }

    if app.is_terminal {
        runner.extend_from_slice(&cli.terminal_launcher.split(' ').collect::<Vec<&str>>());
    }

    runner.extend_from_slice(&commands.iter().map(AsRef::as_ref).collect::<Vec<&str>>());

    let mut exec = process::Command::new(runner[0]);
    exec.args(&runner[1..]);

    // Ensure detached launches always get their own session and null stdio
    if cli.detach {
        #[allow(unsafe_code)]
        unsafe {
            use std::os::unix::process::CommandExt;
            exec.pre_exec(move || {
                if libc::setsid() == -1 {
                    return Err(io::Error::last_os_error());
                }

                libc::signal(libc::SIGHUP, libc::SIG_IGN);

                let c_path = CString::new("/dev/null")
                    .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
                let null_fd = libc::open(c_path.as_ptr(), libc::O_RDWR);
                if null_fd == -1 {
                    return Err(io::Error::last_os_error());
                }

                let dup = |fd: libc::c_int| -> io::Result<()> {
                    if libc::dup2(null_fd, fd) == -1 {
                        Err(io::Error::last_os_error())
                    } else {
                        Ok(())
                    }
                };

                dup(libc::STDIN_FILENO)?;
                dup(libc::STDOUT_FILENO)?;
                dup(libc::STDERR_FILENO)?;

                libc::close(null_fd);

                Ok(())
            });
        }
    }

    // Redirect stdio when detach is requested to avoid leaking output to parent
    if cli.detach {
        exec.stdin(process::Stdio::null());
        exec.stdout(process::Stdio::null());
        exec.stderr(process::Stdio::null());
    }

    exec.spawn()?;

    // update history
    let value = app.history + 1;
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(crate::core::cache::HISTORY_TABLE)?;
        table.insert(app.name.as_str(), value)?;
    }
    write_txn.commit()?;

    Ok(())
}
