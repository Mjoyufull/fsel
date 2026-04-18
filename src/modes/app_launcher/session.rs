use eyre::{Result, WrapErr, eyre};
use std::collections::BTreeSet;
use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CHECK_INTERVAL_MS: u64 = 5;
const TERM_GRACE_MS: u64 = 200;
const TOTAL_WAIT_MS: u64 = 750;
const DB_OPEN_RETRY_DELAY_MS: u64 = 15;
const LOCK_ACQUIRE_ATTEMPTS: usize = 8;

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

struct LauncherLockGuard(PathBuf, String);

impl Drop for LauncherLockGuard {
    fn drop(&mut self) {
        let _ = remove_lockfile_if_unchanged(&self.0, self.contents());
    }
}

impl LauncherLockGuard {
    fn new(path: PathBuf, contents: String) -> Self {
        Self(path, contents)
    }

    fn contents(&self) -> &str {
        &self.1
    }
}

fn ensure_single_launcher_instance(
    history_db_path: &Path,
    lock_path: &Path,
    replace: bool,
) -> Result<LauncherLockGuard> {
    let expected_contents = build_lock_contents("launcher")?;

    for _ in 0..LOCK_ACQUIRE_ATTEMPTS {
        if write_lockfile_if_absent(lock_path, &expected_contents)? {
            return Ok(LauncherLockGuard::new(
                lock_path.to_path_buf(),
                expected_contents,
            ));
        }

        let lock_contents = read_lock_contents(lock_path)?;
        let holder_pids = holder_pid_set(history_db_path);
        let owner_state = classify_launcher_owner(&lock_contents, &holder_pids);

        if holder_pids.is_empty() {
            match owner_state {
                LauncherOwner::Stale => {
                    if remove_lockfile_if_unchanged(lock_path, &lock_contents)? {
                        continue;
                    }
                }
                LauncherOwner::Starting(pid) => {
                    let action = if replace { "replace" } else { "start" };
                    return Err(eyre!(
                        "Refusing to {action}: existing launcher process (pid {pid}) has not finished acquiring the database lock"
                    ));
                }
                LauncherOwner::ActiveHolder => {}
            }
        }

        if !holder_pids.is_empty() {
            if !replace {
                return Err(eyre!("Fsel is already running"));
            }

            terminate_target_pids(&holder_pids)?;
            ensure_no_remaining_holders(history_db_path, &holder_pids)?;
            if !lock_contents.is_empty() {
                let _ = remove_lockfile_if_unchanged(lock_path, &lock_contents)?;
            }
            continue;
        }
    }

    Err(eyre!(
        "Failed to acquire launcher lockfile due to concurrent startup activity"
    ))
}

fn read_lock_contents(lock_path: &Path) -> Result<String> {
    match fs::read_to_string(lock_path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Ok(contents) => Ok(contents),
        Err(error) => Err(error).wrap_err("Failed to read lockfile"),
    }
}

enum LauncherOwner {
    ActiveHolder,
    Starting(i32),
    Stale,
}

fn classify_launcher_owner(lock_contents: &str, holder_pids: &BTreeSet<i32>) -> LauncherOwner {
    let Some(pid) = parse_lock_pid(lock_contents) else {
        return LauncherOwner::Stale;
    };

    if holder_pids.contains(&pid) {
        return LauncherOwner::ActiveHolder;
    }

    if crate::platform::process::process_exists(pid)
        && crate::platform::process::process_matches_current_exe(pid).unwrap_or(false)
        && !crate::platform::process::process_has_argument(pid, "--cclip").unwrap_or(false)
        && !crate::platform::process::process_has_argument(pid, "--dmenu").unwrap_or(false)
    {
        return LauncherOwner::Starting(pid);
    }

    LauncherOwner::Stale
}

fn holder_pid_set(history_db_path: &Path) -> BTreeSet<i32> {
    crate::platform::process::find_processes_holding_file(history_db_path)
        .unwrap_or_default()
        .into_iter()
        .collect()
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

        if waited_ms >= TERM_GRACE_MS && !escalated {
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

fn build_lock_contents(mode: &str) -> Result<String> {
    let pid = crate::platform::process::get_current_pid();
    let exe_path = std::env::current_exe()
        .wrap_err("Failed to resolve current executable for launcher lockfile")?
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_PKG_NAME")));
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    Ok(format!(
        "pid={pid}\nmode={mode}\nexe={}\nnonce={nonce}\n",
        exe_path.display()
    ))
}

fn write_lockfile_if_absent(lock_path: &Path, contents: &str) -> Result<bool> {
    let mut lock_file = match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(lock_path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => return Ok(false),
        Err(error) => return Err(error).wrap_err("Failed to create launcher lockfile"),
    };

    lock_file
        .write_all(contents.as_bytes())
        .wrap_err("Failed to write launcher lockfile")?;
    Ok(true)
}

fn parse_lock_pid(lock_contents: &str) -> Option<i32> {
    lock_contents
        .lines()
        .find_map(|line| line.strip_prefix("pid="))
        .unwrap_or(lock_contents.trim())
        .parse::<i32>()
        .ok()
}

fn remove_lockfile_if_unchanged(lock_path: &Path, expected_contents: &str) -> Result<bool> {
    match fs::read_to_string(lock_path) {
        Ok(current_contents) if current_contents == expected_contents => {
            if let Err(error) = fs::remove_file(lock_path)
                && error.kind() != io::ErrorKind::NotFound
            {
                return Err(error).wrap_err("Failed to remove launcher lockfile");
            }
            Ok(true)
        }
        Ok(_) => Ok(false),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).wrap_err("Failed to validate launcher lockfile ownership"),
    }
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
    use super::{
        LauncherOwner, LauncherSession, classify_launcher_owner, parse_lock_pid,
        should_retry_database_open,
    };
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
    fn parse_lock_pid_supports_legacy_and_structured_lockfiles() {
        assert_eq!(parse_lock_pid("123"), Some(123));
        assert_eq!(parse_lock_pid("pid=456\nmode=launcher\n"), Some(456));
        assert_eq!(parse_lock_pid("pid=not-a-pid\n"), None);
    }

    #[test]
    fn classify_launcher_owner_marks_missing_or_invalid_lock_as_stale() {
        assert!(matches!(
            classify_launcher_owner("not-a-pid", &std::collections::BTreeSet::new()),
            LauncherOwner::Stale
        ));
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
        let current_exe = std::env::current_exe().expect("current exe should be available");
        fs::write(
            &lock_path,
            format!(
                "pid={}\nmode=launcher\nexe={}\nnonce=1\n",
                crate::platform::process::get_current_pid(),
                current_exe.display()
            ),
        )
        .expect("lockfile should be written");

        let error = match LauncherSession::start(&history_db_path, &lock_path, false) {
            Ok(_) => panic!("existing lockfile should block startup without --replace"),
            Err(error) => error,
        };

        let message = error.to_string();
        assert!(
            message.contains("Fsel is already running") || message.contains("Refusing to start"),
            "unexpected error message: {message}"
        );
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
            let lock_contents =
                fs::read_to_string(&lock_path).expect("lockfile should be readable");
            assert!(lock_contents.contains(&format!(
                "pid={}",
                crate::platform::process::get_current_pid()
            )));
            assert!(lock_contents.contains("mode=launcher"));
            assert!(session.db().begin_read().is_ok());
        }

        assert!(!lock_path.exists());
        let _ = fs::remove_dir_all(dir);
    }
}
