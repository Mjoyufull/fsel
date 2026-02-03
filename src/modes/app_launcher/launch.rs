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
    if commands.is_empty() {
        return Err(eyre::eyre!("Empty command for app '{}'", app.name));
    }
    if crate::cli::DEBUG_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        crate::core::debug_logger::log_event(&format!(
            "Launch requested for '{}' with command: {}",
            app.name, app.command
        ));
    }

    if let Some(path) = &app.path {
        env::set_current_dir(std::path::PathBuf::from(path))?;
    }

    if cli.tty && app.is_terminal {
        use std::os::unix::process::CommandExt;

        if crate::cli::DEBUG_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            crate::core::debug_logger::log_event("TTY mode: Replacing fsel with target app");
            crate::core::debug_logger::log_launch(app, &app.command);
        }

        // Validate the target executable before recording history / committing.
        // Resolve commands[0] on PATH (if it has no slash) or check the given path,
        // and ensure it exists and is executable. This prevents failed execs from
        // being recorded as successful launches.
        let exe_path_opt: Option<std::path::PathBuf> = {
            let cmd0: &str = &commands[0];
            let cmd_path = std::path::Path::new(cmd0);
            if cmd0.contains('/') {
                // Path provided directly
                if cmd_path.exists() && cmd_path.is_file() {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        match cmd_path.metadata() {
                            Ok(md) => {
                                if (md.permissions().mode() & 0o111) != 0 {
                                    Some(cmd_path.to_path_buf())
                                } else {
                                    None
                                }
                            }
                            Err(_) => None,
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        Some(cmd_path.to_path_buf())
                    }
                } else {
                    None
                }
            } else {
                // Search PATH
                if let Ok(pathvar) = env::var("PATH") {
                    let mut found: Option<std::path::PathBuf> = None;
                    for dir in pathvar.split(':') {
                        let candidate = std::path::Path::new(dir).join(cmd0);
                        if candidate.exists() && candidate.is_file() {
                            #[cfg(unix)]
                            {
                                use std::os::unix::fs::PermissionsExt;
                                if let Ok(md) = candidate.metadata() {
                                    if (md.permissions().mode() & 0o111) != 0 {
                                        found = Some(candidate);
                                        break;
                                    }
                                }
                            }
                            #[cfg(not(unix))]
                            {
                                found = Some(candidate);
                                break;
                            }
                        }
                    }
                    found
                } else {
                    None
                }
            }
        };

        let exe_path = match exe_path_opt {
            Some(p) => p,
            None => {
                return Err(eyre::eyre!(
                    "Executable not found or not executable: {}",
                    commands[0]
                ))
            }
        };

        // Record history and frecency BEFORE exec since we disappear after
        let value = app.history + 1;
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(crate::core::cache::HISTORY_TABLE)?;
            table.insert(app.name.as_str(), value)?;
        }
        write_txn.commit()?;

        if let Err(e) = crate::core::database::record_access(db, &app.name) {
            eprintln!("Warning: Failed to update frecency: {}", e);
        }

        let mut exec = process::Command::new(exe_path);
        exec.args(&commands[1..]);

        let err = exec.exec();
        // If we're here, exec failed
        return Err(err.into());
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

    if crate::cli::DEBUG_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        let cmd_str = format!(
            "{} {}",
            exec.get_program().to_string_lossy(),
            exec.get_args()
                .map(|a| a.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ")
        );
        crate::core::debug_logger::log_launch(app, &cmd_str);
    }

    exec.spawn()?;

    // log it for history
    let value = app.history + 1;
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(crate::core::cache::HISTORY_TABLE)?;
        table.insert(app.name.as_str(), value)?;
    }
    write_txn.commit()?;

    // Update frecency (modern usage tracking)
    if let Err(e) = crate::core::database::record_access(db, &app.name) {
        eprintln!("Warning: Failed to update frecency: {}", e);
    }

    Ok(())
}
