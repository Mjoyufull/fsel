//! Detached launches owned by one live launcher session.

use crate::cli::Opts;
use crate::core::state::State;
use redb::ReadableTable;
use std::process::Child;
use std::sync::Arc;

#[derive(Default)]
pub(super) struct PersistentSession {
    children: Vec<Child>,
}

impl PersistentSession {
    pub(super) fn reap(&mut self) {
        self.children
            .retain_mut(|child| !matches!(child.try_wait(), Ok(Some(_))));
    }

    pub(super) fn launch(&mut self, state: &mut State, cli: &Opts, db: &Arc<redb::Database>) {
        state.should_launch = false;
        let Some(app) = state.selected.and_then(|index| state.shown.get(index)) else {
            return;
        };
        let name = app.name.clone();
        let message = match super::launch::spawn_app(app, cli) {
            Ok(child) => {
                self.children.push(child);
                match record_launch(db, &name) {
                    Ok(count) => {
                        let message = match crate::core::database::record_access(db, &name) {
                            Ok(frecency) => {
                                state.frecency_data = frecency;
                                format!("Launched {name}")
                            }
                            Err(error) => format!(
                                "Launched {name}; history saved, but could not update frecency: {error}"
                            ),
                        };
                        state.update_launch_metadata(&name, count);
                        message
                    }
                    Err(error) => format!("Launched {name}; could not save history: {error}"),
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

fn record_launch(db: &Arc<redb::Database>, name: &str) -> eyre::Result<u64> {
    let transaction = db.begin_write()?;
    let count = {
        let mut table = transaction.open_table(crate::core::cache::HISTORY_TABLE)?;
        let count = table.get(name)?.map_or(0, |value| value.value());
        let count = count.saturating_add(1);
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
        session.reap();
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
        session.reap();
        assert!(session.children.is_empty());
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
