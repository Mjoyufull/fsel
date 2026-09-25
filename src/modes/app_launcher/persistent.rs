//! Detached launches owned by one live launcher session.

use crate::cli::Opts;
use crate::core::state::State;
use redb::ReadableTable;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::{Child, Stdio};
use std::sync::Arc;

#[derive(Default)]
pub(super) struct PersistentSession {
    children: Vec<Child>,
    hooks: Vec<Child>,
}

impl PersistentSession {
    /// Collect the children that have finished, reporting a launch command that failed.
    ///
    /// The command keeps no terminal of its own and fsel never waits for it, so one that
    /// starts and then fails leaves no trace at all. A script the shell cannot execute is
    /// the common case, and it is silent exactly when it needs explaining.
    pub(super) fn reap(&mut self) -> Option<String> {
        self.children
            .retain_mut(|child| !matches!(child.try_wait(), Ok(Some(_))));

        let mut failure = None;
        self.hooks.retain_mut(|hook| match hook.try_wait() {
            Ok(None) => true,
            Ok(Some(status)) => {
                if !status.success() {
                    failure.get_or_insert_with(|| format!("Launch command {status}"));
                }
                false
            }
            Err(_) => false,
        });
        failure
    }

    pub(super) fn launch(&mut self, state: &mut State, cli: &Opts, db: &Arc<redb::Database>) {
        state.should_launch = false;
        let Some(app) = state.selected.and_then(|index| state.shown.get(index)) else {
            return;
        };
        let name = app.name.clone();
        let message = match super::launch::spawn_app(app, cli) {
            Ok(child) => {
                let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
                let hook = cli
                    .on_launch
                    .as_deref()
                    .map(|command| notify_launch(&shell, command, app, child.id()));
                self.children.push(child);
                let launched = match record_launch(db, &name) {
                    Ok(count) => {
                        let saved = match crate::core::database::record_access(db, &name) {
                            Ok(frecency) => {
                                state.frecency_data = frecency;
                                format!("Launched {name}")
                            }
                            Err(error) => format!(
                                "Launched {name}; history saved, \
                                 but could not update frecency: {error}"
                            ),
                        };
                        state.update_launch_metadata(&name, count);
                        saved
                    }
                    Err(error) => format!("Launched {name}; could not save history: {error}"),
                };
                match hook {
                    Some(Ok(hook)) => {
                        self.hooks.push(hook);
                        launched
                    }
                    Some(Err(error)) => {
                        format!("{launched}; launch command did not start: {error}")
                    }
                    None => launched,
                }
            }
            Err(error) => format!("Could not launch {}: {error}", app.name),
        };
        state.set_status_message(message);
        state.update_info(
            cli.highlight_color,
            cli.fancy_mode,
            cli.verbose.unwrap_or(0),
        );
    }
}

/// Announce a launch to the session's hook command.
///
/// The launcher stays open, so nothing downstream of fsel can react to a launch on its
/// own. The hook runs in its own process group with no terminal of its own: fsel owns
/// the screen, and a hook that closes the window fsel runs in outlives that window.
///
/// `FSEL_PID` is passed because the hook cannot find fsel by walking up from itself:
/// a shell that forks rather than execs the command sits between the two.
fn notify_launch(
    shell: &str,
    command: &str,
    app: &crate::desktop::App,
    pid: u32,
) -> std::io::Result<Child> {
    let mut hook = std::process::Command::new(shell);
    hook.args(["-c", command])
        .env("FSEL_PID", std::process::id().to_string())
        .env("FSEL_LAUNCHED_APP", &app.name)
        .env("FSEL_LAUNCHED_COMMAND", &app.command)
        .env("FSEL_LAUNCHED_PID", pid.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    hook.process_group(0);
    hook.spawn()
}

fn record_launch(db: &Arc<redb::Database>, name: &str) -> eyre::Result<u64> {
    let transaction = db.begin_write()?;
    let count = {
        let mut table = transaction.open_table(crate::core::cache::HISTORY_TABLE)?;
        let previous: u64 = table.get(name)?.map_or(0, |value| value.value());
        let count = previous.saturating_add(1);
        table.insert(name, count)?;
        count
    };
    transaction.commit()?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use redb::ReadableDatabase;
    use std::fs;

    fn database() -> Arc<redb::Database> {
        Arc::new(
            redb::Database::builder()
                .create_with_backend(redb::backends::InMemoryBackend::new())
                .unwrap(),
        )
    }

    fn state(command: &str) -> (State, Opts) {
        let app = crate::desktop::App::parse(
            format!("[Desktop Entry]\nType=Application\nName=Fixture\nExec={command}\n"),
            false,
        )
        .unwrap();
        let cli = Opts {
            persistent: true,
            detach: true,
            ..Default::default()
        };
        let mut state = State::new(
            vec![app],
            cli.match_mode,
            Default::default(),
            cli.prefix_depth,
            cli.ranking_mode,
            cli.pinned_order_mode,
            Default::default(),
        );
        state.query = "Fixture".into();
        state.selected = Some(0);
        (state, cli)
    }

    #[test]
    fn repeated_launches_preserve_selection_and_increment_live_history() {
        let db = database();
        let (mut state, cli) = state("/bin/true");
        let mut session = PersistentSession::default();
        for _ in 0..2 {
            state.should_launch = true;
            session.launch(&mut state, &cli, &db);
            assert!(!state.should_launch);
            assert_eq!(state.query, "Fixture");
            assert_eq!(state.selected, Some(0));
            assert_eq!(state.scroll_offset, 0);
        }
        let read = db.begin_read().unwrap();
        let table = read.open_table(crate::core::cache::HISTORY_TABLE).unwrap();
        assert_eq!(table.get("Fixture").unwrap().unwrap().value(), 2);
        assert_eq!(state.apps[0].history, 2);
        assert_eq!(state.shown[0].history, 2);
        assert!(state.shown[0].last_access.is_some());
        assert!(state.frecency_data.contains_key("Fixture"));
        state.filter();
        assert_eq!(state.shown[0].history, 2);
        for child in &mut session.children {
            child.wait().unwrap();
        }
        assert!(session.reap().is_none());
        assert!(session.children.is_empty());
    }

    #[test]
    fn frecency_failure_reports_that_history_was_saved() {
        let db = database();
        let transaction = db.begin_write().unwrap();
        transaction
            .open_table(redb::TableDefinition::<&str, u64>::new("frecency"))
            .unwrap();
        transaction.commit().unwrap();
        let (mut state, cli) = state("/bin/true");
        let mut session = PersistentSession::default();
        session.launch(&mut state, &cli, &db);
        assert!(
            state
                .text
                .contains("history saved, but could not update frecency")
        );
        assert_eq!(state.shown[0].history, 1);
        assert_eq!(
            db.begin_read()
                .unwrap()
                .open_table(crate::core::cache::HISTORY_TABLE)
                .unwrap()
                .get("Fixture")
                .unwrap()
                .unwrap()
                .value(),
            1
        );
        session.children[0].wait().unwrap();
    }

    #[test]
    fn spawn_failure_keeps_session_without_recording_history() {
        let db = database();
        let (mut state, cli) = state("/nonexistent/fsel-persistent-test");
        let mut session = PersistentSession::default();
        state.should_launch = true;
        session.launch(&mut state, &cli, &db);
        assert!(!state.should_launch);
        assert!(!state.should_exit);
        assert!(state.text.contains("Could not launch Fixture"));
        assert!(session.children.is_empty());
        assert!(
            db.begin_read()
                .unwrap()
                .open_table(crate::core::cache::HISTORY_TABLE)
                .is_err()
        );
    }

    #[test]
    fn history_failure_does_not_lose_the_spawned_child() {
        let db = database();
        let transaction = db.begin_write().unwrap();
        transaction
            .open_table(redb::TableDefinition::<&str, &str>::new("history"))
            .unwrap();
        transaction.commit().unwrap();
        let (mut state, cli) = state("/bin/true");
        let mut session = PersistentSession::default();
        session.launch(&mut state, &cli, &db);
        assert_eq!(session.children.len(), 1);
        assert!(
            state
                .text
                .contains("Launched Fixture; could not save history")
        );
        session.children[0].wait().unwrap();
        assert!(session.reap().is_none());
        assert!(session.children.is_empty());
    }

    fn report_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("fsel-on-launch-{label}-{}", std::process::id()))
    }

    #[test]
    fn a_launch_reports_the_application_to_the_hook_command() {
        let db = database();
        let report = report_path("reported");
        let _ = fs::remove_file(&report);
        let (mut state, mut cli) = state("/bin/true");
        cli.on_launch = Some(format!(
            "printf '%s|%s|%s|%s' \"$FSEL_PID\" \"$FSEL_LAUNCHED_APP\" \
             \"$FSEL_LAUNCHED_COMMAND\" \"$FSEL_LAUNCHED_PID\" > {}",
            report.display()
        ));
        let mut session = PersistentSession::default();

        session.launch(&mut state, &cli, &db);

        assert_eq!(session.children.len(), 1);
        assert_eq!(session.hooks.len(), 1);
        let launched = session.children[0].id();
        for child in session.children.iter_mut().chain(&mut session.hooks) {
            child.wait().unwrap();
        }
        assert_eq!(
            fs::read_to_string(&report).unwrap(),
            format!("{}|Fixture|/bin/true|{launched}", std::process::id())
        );
        assert!(session.reap().is_none());
        let _ = fs::remove_file(&report);
    }

    #[test]
    fn a_launch_command_that_fails_is_reported_rather_than_swallowed() {
        let db = database();
        let (mut state, mut cli) = state("/bin/true");
        cli.on_launch = Some("exit 126".to_string());
        let mut session = PersistentSession::default();

        session.launch(&mut state, &cli, &db);

        assert_eq!(session.hooks.len(), 1);
        session.hooks[0].wait().unwrap();
        let failure = session.reap().expect("a failed command should be reported");
        assert!(failure.contains("126"), "{failure}");
        assert!(session.hooks.is_empty());
    }

    #[test]
    fn an_unusable_shell_reports_instead_of_running_the_hook() {
        let (state, _) = state("/bin/true");

        let failure = notify_launch("/nonexistent/fsel-shell", "true", &state.shown[0], 1);

        assert!(failure.is_err());
    }

    #[test]
    fn a_failed_launch_leaves_the_hook_command_unrun() {
        let db = database();
        let report = report_path("unrun");
        let _ = fs::remove_file(&report);
        let (mut state, mut cli) = state("/nonexistent/fsel-persistent-test");
        cli.on_launch = Some(format!("printf 'ran' > {}", report.display()));
        let mut session = PersistentSession::default();

        session.launch(&mut state, &cli, &db);

        assert!(session.children.is_empty());
        assert!(!report.exists());
    }

    #[test]
    fn desktop_working_directory_is_child_local() {
        let before = std::env::current_dir().unwrap();
        let (mut state, cli) = state("/bin/sh -c 'test \"$PWD\" = /tmp'");
        state.shown[0].path = Some("/tmp".into());
        let mut child = super::super::launch::spawn_app(&state.shown[0], &cli).unwrap();
        assert!(child.wait().unwrap().success());
        assert_eq!(std::env::current_dir().unwrap(), before);
    }
}
