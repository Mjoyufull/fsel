use std::convert::{AsRef, TryInto};
use std::fmt;
use std::fs;
use std::path;
use std::sync::mpsc;
use std::thread;

use eyre::eyre;
use ratatui::widgets::ListItem;
use walkdir::WalkDir;


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

}

/// Find XDG applications in `dirs` (recursive).
///
/// Spawns a new thread and sends apps via a mpsc [Receiver]
///
/// Updates history using the database
///
/// [Receiver]: std::sync::mpsc::Receiver
pub fn read(dirs: Vec<impl Into<path::PathBuf>>, db: &sled::Db) -> mpsc::Receiver<App> {
    let (sender, receiver) = mpsc::channel();

    let dirs: Vec<path::PathBuf> = dirs.into_iter().map(Into::into).collect();
    let db = AppHistory { db: db.clone() };

    let _worker = thread::spawn(move || {
        // Collect all .desktop files first
        let mut desktop_files = Vec::new();
        for dir in &dirs {
            for entry in WalkDir::new(dir)
                .min_depth(1)
                .max_depth(3) // Limit depth to avoid deep recursion
                .into_iter()
                .filter_map(Result::ok)
                .filter(|entry| {
                    !entry.file_type().is_dir() && 
                    entry.path().extension().and_then(|s| s.to_str()) == Some("desktop")
                })
            {
                desktop_files.push(entry.path().to_path_buf());
            }
        }

        // Process files in batches for better performance
        for file_path in desktop_files {
            if let Ok(contents) = fs::read_to_string(&file_path) {
                // Skip files without [Desktop Entry] section early
                if !contents.contains("[Desktop Entry]") {
                    continue;
                }
                
                if let Ok(app) = App::parse(&contents, None) {
                    let app_with_history = db.get(app.clone());
                    
                    // Handle actions
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

                    if sender.send(app_with_history).is_err() {
                        return; // Receiver dropped
                    }
                }
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

        // Fast parsing with early exits and optimizations
        for line in contents.lines() {
            if line.is_empty() { continue; }
            
            if line.starts_with("[Desktop") && search {
                break; // Early exit when we hit another section
            }

            if line == pattern {
                search = true;
                continue;
            }

            if search {
                // Use match for better performance than multiple if-else
                if let Some((key, value)) = line.split_once('=') {
                    match key {
                        "Name" if name.is_none() => {
                            name = Some(if let Some(a) = &action {
                                format!("{} ({})", &a.from, value)
                            } else {
                                value.to_string()
                            });
                        }
                        "Comment" if description.is_none() => {
                            description = Some(value.to_string());
                        }
                        "Terminal" => {
                            terminal_exec = value == "true";
                        }
                        "Exec" if exec.is_none() => {
                            // Fast regex-free approach for XDG parameter removal
                            let mut trimmed = value;
                            if let Some(pos) = value.find(" %") {
                                trimmed = &value[..pos];
                            }
                            exec = Some(trimmed.to_string());
                        }
                        "NoDisplay" => {
                            if value.eq_ignore_ascii_case("true") {
                                return Err(eyre!("App is hidden"));
                            }
                        }
                        "Path" if path.is_none() => {
                            path = Some(value.to_string());
                        }
                        "Actions" if actions.is_none() && action.is_none() => {
                            actions = Some(value.split(';').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect());
                        }
                        _ => {} // Ignore other keys
                    }
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
