use crate::core::ranking::ScoreBreakdown;
use ratatui::widgets::ListItem;
use std::fmt;

mod discover;
mod parse;

pub use discover::read_with_options;

/// An XDG Specification app with full desktop-entry metadata.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct App {
    /// App name (Name field).
    pub name: String,
    /// Command to run (Exec field).
    pub command: String,
    /// App description/comment (Comment field).
    pub description: String,
    /// Generic name of application (GenericName field).
    pub generic_name: Option<String>,
    /// Keywords for searching (Keywords field).
    pub keywords: Vec<String>,
    /// Categories this application belongs to (Categories field).
    pub categories: Vec<String>,
    /// MIME types this application can handle (MimeType field).
    pub mime_types: Vec<String>,
    /// Icon name or path (Icon field).
    pub icon: Option<String>,
    /// Run in terminal (Terminal field).
    pub is_terminal: bool,
    /// Path from which to run the command (Path field).
    pub path: Option<String>,
    /// Show only in these DEs (OnlyShowIn field).
    pub only_show_in: Vec<String>,
    /// Hide in these DEs (NotShowIn field).
    pub not_show_in: Vec<String>,
    /// Whether the app is hidden (Hidden field).
    pub hidden: bool,
    /// Application startup notification (StartupNotify field).
    pub startup_notify: bool,
    /// WM class for startup notification (StartupWMClass field).
    pub startup_wm_class: Option<String>,
    /// Command to test if executable exists (TryExec field).
    pub try_exec: Option<String>,
    /// Desktop Entry type (usually "Application").
    pub entry_type: String,
    /// Desktop file ID for tracking.
    pub desktop_id: Option<String>,

    /// Matching score (used in UI).
    pub score: i64,
    /// Number of times this app was run.
    pub history: u64,
    /// Whether this app is pinned/favorited.
    pub pinned: bool,
    /// Last access timestamp (Unix epoch seconds).
    pub last_access: Option<u64>,
    /// Detailed score breakdown for debugging (`-T`).
    pub breakdown: Option<ScoreBreakdown>,

    #[doc(hidden)]
    actions: Option<Vec<String>>,
}

impl App {
    /// Returns a corrected score that blends history and matching score.
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

impl Ord for App {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self.pinned, other.pinned) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }

        self.corrected_score()
            .cmp(&other.corrected_score())
            .reverse()
            .then(self.name.to_lowercase().cmp(&other.name.to_lowercase()))
    }
}

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

#[derive(Default)]
struct Action {
    name: String,
    from: String,
}

impl Action {
    fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    fn from(mut self, from: impl Into<String>) -> Self {
        self.from = from.into();
        self
    }
}
