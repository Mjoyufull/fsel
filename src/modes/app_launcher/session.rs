use eyre::{Result, WrapErr, eyre};
use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const CHECK_INTERVAL_MS: u64 = 5;
const TOTAL_WAIT_MS: u64 = 30;
const DB_OPEN_RETRY_DELAY_MS: u64 = 15;

/// Active launcher session resources that must live for the duration of app launcher mode.
pub(crate) struct LauncherSession {
    db: Arc<redb::Database>,
    _lock: LauncherLockGuard,
}

impl LauncherSession {
    /// Starts a launcher session by enforcing singleton policy and opening the database.
    pub(crate) fn start(history_db_path: &Path, lock_path: &Path, replace: bool) -> Result<Self> {
        let lock = ensure_single_launcher_instance(history_db_path, lock_path, replace)?;
        let db = Arc::new(open_database(history_db_path, replace)?);
        Ok(Self { db, _lock: lock })
    }

    /// Returns the shared history database handle for the session.
    pub(crate) fn db(&self) -> &Arc<redb::Database> {
        &self.db
    }
}

struct LauncherLockGuard(PathBuf);

impl Drop for LauncherLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn ensure_single_launcher_instance(
    history_db_path: &Path,
    lock_path: &Path,
    replace: bool,
) -> Result<LauncherLockGuard> {
    let lock_contents = read_lock_contents(lock_path)?;

    if !lock_contents.is_empty() {
        if !replace {
            return Err(eyre!("Fsel is already running"));
        }

        let holder_pids = crate::platform::process::find_processes_holding_file(history_db_path)
            .unwrap_or_default();
        let target_pids = collect_target_pids(&lock_contents, &holder_pids);
        terminate_target_pids(&target_pids)?;
        ensure_no_remaining_holders(history_db_path, &target_pids)?;
    } else if replace {
        let target_pids: BTreeSet<i32> =
            crate::platform::process::find_processes_holding_file(history_db_path)
                .unwrap_or_default()
                .into_iter()
                .collect();

        if !target_pids.is_empty() {
            terminate_target_pids(&target_pids)?;
            ensure_no_remaining_holders(history_db_path, &target_pids)?;
        }
    }

    remove_existing_lockfile(lock_path)?;
    write_current_pid_lockfile(lock_path)?;
    Ok(LauncherLockGuard(lock_path.to_path_buf()))
}

fn read_lock_contents(lock_path: &Path) -> Result<String> {
    match fs::read_to_string(lock_path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Ok(contents) => Ok(contents),
        Err(error) => Err(error).wrap_err("Failed to read lockfile"),
    }
}

fn collect_target_pids(lock_contents: &str, holder_pids: &[i32]) -> BTreeSet<i32> {
    let mut target_pids = BTreeSet::new();

    if let Ok(pid) = lock_contents.trim().parse::<i32>() {
        target_pids.insert(pid);
    }

    target_pids.extend(holder_pids.iter().copied());
    target_pids
}

fn terminate_target_pids(target_pids: &BTreeSet<i32>) -> Result<()> {
    for pid in target_pids.iter().copied() {
        if let Err(error) = crate::platform::process::kill_process_sigterm_result(pid)
            && error.raw_os_error() != Some(libc::ESRCH)
        {
            return Err(eyre!("Failed to kill process {}: {}", pid, error));
        }

        wait_for_process_exit(pid)?;
    }

    Ok(())
}

fn wait_for_process_exit(pid: i32) -> Result<()> {
    let mut waited_ms = 0u64;
    let mut escalated = false;

    loop {
        if !crate::platform::process::process_exists(pid) {
            return Ok(());
        }

        if !escalated {
            if let Err(error) = crate::platform::process::kill_process_sigkill_result(pid)
                && error.raw_os_error() != Some(libc::ESRCH)
            {
                return Err(eyre!("Failed to kill process {}: {}", pid, error));
            }
            escalated = true;
        }

        if waited_ms >= TOTAL_WAIT_MS {
            return Err(eyre!("Existing fsel instance (pid {pid}) refused to exit"));
        }

        thread::sleep(Duration::from_millis(CHECK_INTERVAL_MS));
        waited_ms += CHECK_INTERVAL_MS;
    }
}

fn ensure_no_remaining_holders(
    history_db_path: &Path,
    excluded_pids: &BTreeSet<i32>,
) -> Result<()> {
    if let Ok(mut remaining) =
        crate::platform::process::find_processes_holding_file(history_db_path)
    {
        remaining.retain(|pid| !excluded_pids.contains(pid));
        if !remaining.is_empty() {
            return Err(eyre!(
                "Existing fsel instance (pid(s) {:?}) refused to exit",
                remaining
            ));
        }
    }

    Ok(())
}

fn remove_existing_lockfile(lock_path: &Path) -> Result<()> {
    if let Err(error) = fs::remove_file(lock_path)
        && error.kind() != io::ErrorKind::NotFound
    {
        return Err(error).wrap_err("Failed to remove existing lockfile");
    }

    Ok(())
}

fn write_current_pid_lockfile(lock_path: &Path) -> Result<()> {
    let mut lock_file = fs::File::create(lock_path)?;
    lock_file.write_all(
        crate::platform::process::get_current_pid()
            .to_string()
            .as_bytes(),
    )?;
    Ok(())
}

fn open_database(history_db_path: &Path, replace: bool) -> Result<redb::Database> {
    let mut database = redb::Database::create(history_db_path);
    if let Err(error) = &database
        && should_retry_database_open(replace, &error.to_string())
    {
        thread::sleep(Duration::from_millis(DB_OPEN_RETRY_DELAY_MS));
        database = redb::Database::create(history_db_path);
    }

    database.wrap_err_with(|| format!("Failed to open database at {:?}", history_db_path))
}

fn should_retry_database_open(replace: bool, error_message: &str) -> bool {
    replace && error_message.contains("Cannot acquire lock")
}

#[cfg(test)]
mod tests {
    use super::{LauncherSession, collect_target_pids, should_retry_database_open};
    use redb::ReadableDatabase;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "fsel-session-{label}-{}-{unique}",
            crate::platform::process::get_current_pid()
        ));
        fs::create_dir_all(&dir).expect("test temp dir should be created");
        dir
    }

    #[test]
    fn collect_target_pids_merges_and_deduplicates_sources() {
        let targets = collect_target_pids("123", &[123, 456, 456]);
        assert_eq!(targets.into_iter().collect::<Vec<_>>(), vec![123, 456]);
    }

    #[test]
    fn collect_target_pids_ignores_invalid_lockfile_contents() {
        let targets = collect_target_pids("not-a-pid", &[456]);
        assert_eq!(targets.into_iter().collect::<Vec<_>>(), vec![456]);
    }

    #[test]
    fn should_retry_database_open_only_for_replace_lock_errors() {
        assert!(should_retry_database_open(true, "Cannot acquire lock"));
        assert!(!should_retry_database_open(false, "Cannot acquire lock"));
        assert!(!should_retry_database_open(
            true,
            "Some other database error"
        ));
    }

    #[test]
    fn start_without_replace_rejects_existing_lockfile() {
        let dir = test_temp_dir("reject");
        let history_db_path = dir.join("history.db");
        let lock_path = dir.join("launcher.lock");
        fs::write(&lock_path, "123").expect("lockfile should be written");

        let error = match LauncherSession::start(&history_db_path, &lock_path, false) {
            Ok(_) => panic!("existing lockfile should block startup without --replace"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("Fsel is already running"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn start_writes_and_removes_lockfile_on_drop() {
        let dir = test_temp_dir("guard");
        let history_db_path = dir.join("history.db");
        let lock_path = dir.join("launcher.lock");

        {
            let session = LauncherSession::start(&history_db_path, &lock_path, false)
                .expect("session should start");
            assert!(lock_path.exists());
            assert_eq!(
                fs::read_to_string(&lock_path).expect("lockfile should be readable"),
                crate::platform::process::get_current_pid().to_string()
            );
            assert!(session.db().begin_read().is_ok());
        }

        assert!(!lock_path.exists());
        let _ = fs::remove_dir_all(dir);
    }
}
