use std::convert::{AsRef, TryInto};
use std::fmt;
use std::fs;
use std::path;
use std::sync::mpsc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use eyre::eyre;
use ratatui::widgets::ListItem;
use safe_regex::{regex, Matcher1};
use walkdir::WalkDir;

/// Cache entry for a single application
#[derive(serde::Serialize, serde::Deserialize)]
struct CachedApp {
    app: App,
    file_path: String,
    file_mtime: u64,
    cached_at: u64,
}

/// Cache entry for a directory scan
#[derive(serde::Serialize, serde::Deserialize)]
struct DirectoryCache {
    directory: String,
    last_scan: u64,
    apps: Vec<CachedApp>,
}

pub struct AppHistory {
    pub db: sled::Db,
}

impl AppHistory {
    pub fn get(&self, app: App) -> App {
        let mut app = app;
        if let Some(packed) = self.db.get(app.name.as_bytes()).unwrap() {
            let unpacked = super::bytes::unpack(
                packed
                    .as_ref()
                    .try_into()
                    .expect("Invalid data stored in database"),
            );
            app.history = unpacked;
        }
        app
    }

    /// Get current unix timestamp
    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Get file modification time
    fn get_mtime(path: &path::Path) -> u64 {
        fs::metadata(path)
            .and_then(|meta| meta.modified())
            .map(|time| time.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs())
            .unwrap_or(0)
    }

    /// Cache key for directory
    fn cache_key(dir: &path::Path) -> String {
        format!("cache:dir:{}", dir.to_string_lossy())
    }

    /// Get cached apps for a directory if valid
    pub fn get_cached_apps(&self, dir: &path::Path, ttl_seconds: u64) -> Option<Vec<App>> {
        let key = Self::cache_key(dir);
        if let Ok(Some(data)) = self.db.get(key.as_bytes()) {
            if let Ok(cache) = bincode::deserialize::<DirectoryCache>(&data) {
                let now = Self::now();
                // Check if cache is still valid
                if now - cache.last_scan < ttl_seconds {
                    // Check if any files have been modified
                    let mut all_valid = true;
                    for cached_app in &cache.apps {
                        let file_path = path::Path::new(&cached_app.file_path);
                        if !file_path.exists() || Self::get_mtime(file_path) != cached_app.file_mtime {
                            all_valid = false;
                            break;
                        }
                    }
                    if all_valid {
                        return Some(cache.apps.into_iter().map(|ca| self.get(ca.app)).collect());
                    }
                }
            }
        }
        None
    }

    /// Cache apps for a directory
    pub fn cache_apps(&self, dir: &path::Path, apps: &[App], file_paths: &[path::PathBuf]) {
        let now = Self::now();
        let cached_apps: Vec<CachedApp> = apps
            .iter()
            .zip(file_paths.iter())
            .map(|(app, file_path)| CachedApp {
                app: app.clone(),
                file_path: file_path.to_string_lossy().to_string(),
                file_mtime: Self::get_mtime(file_path),
                cached_at: now,
            })
            .collect();

        let directory_cache = DirectoryCache {
            directory: dir.to_string_lossy().to_string(),
            last_scan: now,
            apps: cached_apps,
        };

        if let Ok(data) = bincode::serialize(&directory_cache) {
            let key = Self::cache_key(dir);
            let _ = self.db.insert(key.as_bytes(), data);
        }
    }

    /// Clear all cache entries
    pub fn clear_cache(&self) -> eyre::Result<u32> {
        let mut cleared = 0;
        for item in self.db.scan_prefix(b"cache:") {
            if let Ok((key, _)) = item {
                self.db.remove(key)?;
                cleared += 1;
            }
        }
        Ok(cleared)
    }
}

/// Find XDG applications in `dirs` (recursive).
///
/// Spawns a new thread and sends apps via a mpsc [Receiver]
///
/// Updates history using the database and optionally uses cache
///
/// [Receiver]: std::sync::mpsc::Receiver
pub fn read(dirs: Vec<impl Into<path::PathBuf>>, db: &sled::Db, enable_cache: bool, cache_ttl: u64) -> mpsc::Receiver<App> {
    let (sender, receiver) = mpsc::channel();

    let dirs: Vec<path::PathBuf> = dirs.into_iter().map(Into::into).collect();
    let db = AppHistory { db: db.clone() };

    let _worker = thread::spawn(move || {
        for dir in dirs {
            // Try to get cached apps first
            if enable_cache {
                if let Some(cached_apps) = db.get_cached_apps(&dir, cache_ttl) {
                    // Send cached apps and continue to next directory
                    for app in cached_apps {
                        if sender.send(app).is_err() {
                            return; // Receiver dropped
                        }
                    }
                    continue;
                }
            }

            // Cache miss or caching disabled - scan directory
            let mut files: Vec<path::PathBuf> = vec![];
            let mut apps_for_cache: Vec<App> = vec![];

            for entry in WalkDir::new(&dir)
                .min_depth(1)
                .into_iter()
                .filter(|entry| {
                    if let Ok(path) = entry {
                        if !path.file_type().is_dir() {
                            return true;
                        }
                    }
                    false
                })
                .map(Result::unwrap)
            {
                files.push(entry.path().to_owned());
            }

            let mut processed_files = vec![];
            for file in &files {
                if let Ok(contents) = fs::read_to_string(file) {
                    if let Ok(app) = App::parse(&contents, None) {
                        let app_with_history = db.get(app.clone());
                        
                        if let Some(actions) = &app.actions {
                            for action in actions {
                                let ac = Action::default().name(action).from(app.name.clone());
                                if let Ok(a) = App::parse(&contents, Some(&ac)) {
                                    let action_app = db.get(a);
                                    if sender.send(action_app).is_err() {
                                        return; // Receiver dropped
                                    }
                                }
                            }
                        }

                        if sender.send(app_with_history.clone()).is_err() {
                            return; // Receiver dropped
                        }
                        
                        // Store for caching (without history to avoid duplication)
                        apps_for_cache.push(app);
                        processed_files.push(file.clone());
                    }
                }
            }

            // Cache the results if caching is enabled
            if enable_cache && !apps_for_cache.is_empty() {
                db.cache_apps(&dir, &apps_for_cache, &processed_files);
            }
        }
        drop(sender);
    });

    receiver
}

/// An XDG Specification App
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct App {
    /// App name
    pub name: String,
    /// Command to run
    pub command: String,
    /// App description
    pub description: String,
    /// Whether the app should be run in terminal
    pub is_terminal: bool,
    /// Path from which to run the command
    pub path: Option<String>,
    /// Matching score (used in [UI](super::ui::UI))
    ///
    /// Not part of the specification
    pub score: i64,
    /// Number of times this app was run
    ///
    /// Not part of the specification
    pub history: u64,

    // This is not pub because I use it only on this file
    #[doc(hidden)]
    actions: Option<Vec<String>>,
}

impl App {
    /// Returns a corrected score, mix of history and matching score
    pub fn corrected_score(&self) -> i64 {
        if self.history < 1 {
            self.score
        } else if self.score < 1 {
            self.history as i64
        } else {
            self.score * self.history as i64
        }
    }
}

// Custom Ord implementation, sorts by history then score then alphabetically
impl Ord for App {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Sort by score, highest to lowest
        self.corrected_score()
            .cmp(&other.corrected_score())
            .reverse()
            // Then sort alphabetically
            .then(self.name.to_lowercase().cmp(&other.name.to_lowercase()))
    }
}

// Custom PartialOrd, uses our custom Ord
impl PartialOrd for App {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for App {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

// Will be used to display `App`s in the list
impl AsRef<str> for App {
    fn as_ref(&self) -> &str {
        self.name.as_ref()
    }
}

impl<'a> From<App> for ListItem<'a> {
    fn from(app: App) -> ListItem<'a> {
        ListItem::new(app.name)
    }
}

impl<'a> From<&'a App> for ListItem<'a> {
    fn from(app: &'a App) -> ListItem<'a> {
        ListItem::new(app.name.clone())
    }
}

impl App {
    /// Parse an application, or, if `action.is_some()`, an app action
    pub fn parse<T: AsRef<str>>(contents: T, action: Option<&Action>) -> eyre::Result<App> {
        let contents: &str = contents.as_ref();

        let pattern = if let Some(a) = &action {
            if a.name.is_empty() {
                return Err(eyre!("Action is empty"));
            }
            format!("[Desktop Action {}]", a.name)
        } else {
            "[Desktop Entry]".to_string()
        };

        let mut name = None;
        let mut exec = None;
        let mut description = None;
        let mut terminal_exec = false;
        let mut path = None;
        let mut actions = None;

        let mut search = false;

        for line in contents.lines() {
            if line.starts_with("[Desktop") && search {
                search = false;
            }

            if line == pattern {
                search = true;
            }

            if search {
                if line.starts_with("Name=") && name.is_none() {
                    let line = line.trim_start_matches("Name=");
                    if let Some(a) = &action {
                        name = Some(format!("{} ({})", &a.from, line));
                    } else {
                        name = Some(line.to_string());
                    }
                } else if line.starts_with("Comment=") && description.is_none() {
                    let line = line.trim_start_matches("Comment=");
                    description = Some(line.to_string());
                } else if line.starts_with("Terminal=") {
                    if line.trim_start_matches("Terminal=") == "true" {
                        terminal_exec = true;
                    }
                } else if line.starts_with("Exec=") && exec.is_none() {
                    let line = line.trim_start_matches("Exec=");

                    // Trim %u/%U/%someLetter (which is used as arguments when launching XDG apps,
                    // not used by Gyr)
                    #[allow(clippy::assign_op_pattern)]
                    let matcher: Matcher1<_> = regex!(br".*( ?%[cDdFfikmNnUuv]).*");
                    let mut trimmed = line.to_string();

                    if let Some(range) = matcher.match_ranges(line.as_bytes()) {
                        trimmed.replace_range(range.0.start..range.0.end, "");
                    }

                    exec = Some(trimmed.to_string());
                } else if line.starts_with("NoDisplay=") {
                    let line = line.trim_start_matches("NoDisplay=");
                    if line.to_lowercase() == "true" {
                        return Err(eyre!("App is hidden"));
                    }
                } else if line.starts_with("Path=") && path.is_none() {
                    let line = line.trim_start_matches("Path=");
                    path = Some(line.to_string());
                } else if line.starts_with("Actions=") && actions.is_none() && action.is_none() {
                    let line = line.trim_start_matches("Actions=");
                    let vector = line
                        .split(';')
                        .map(ToString::to_string)
                        .collect::<Vec<String>>();
                    actions = Some(vector);
                }
            }
        }

        let name = name.unwrap_or_else(|| "Unknown".to_string());

        if exec.is_none() {
            return Err(eyre!("No command to run!"));
        }

        let exec = exec.unwrap();
        let description = description.unwrap_or_default();

        Ok(App {
            score: 0,
            history: 0,
            name,
            command: exec,
            description,
            is_terminal: terminal_exec,
            path,
            actions,
        })
    }
}

/// An app action
///
/// In gyr every action is some app, with the action name in parentheses
#[derive(Default)]
pub struct Action {
    /// Action name
    name: String,
    /// App name
    from: String,
}

impl Action {
    /// Set the action's name
    fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the action's app name
    fn from(mut self, from: impl Into<String>) -> Self {
        self.from = from.into();
        self
    }
}
