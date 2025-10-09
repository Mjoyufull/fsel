use std::convert::{AsRef, TryInto};
use std::fmt;
use std::fs;
use std::path;
use std::sync::mpsc;
use std::thread;
use std::env;

use eyre::eyre;
use ratatui::widgets::ListItem;
use walkdir::WalkDir;

/// Get the current locale for desktop entry localization
fn get_locale() -> Vec<String> {
    let mut locales = Vec::new();
    
    // Check LC_MESSAGES first, then LANG, then LC_ALL
    let locale_var = env::var("LC_MESSAGES")
        .or_else(|_| env::var("LANG"))
        .or_else(|_| env::var("LC_ALL"))
        .unwrap_or_else(|_| "C".to_string());
    
    if locale_var != "C" && locale_var != "POSIX" {
        let base_locale = locale_var.split('.').next().unwrap_or(&locale_var);
        
        // Add full locale (e.g., "en_US")
        locales.push(base_locale.to_string());
        
        // Add language only (e.g., "en")
        if let Some(lang) = base_locale.split('_').next() {
            if lang != base_locale {
                locales.push(lang.to_string());
            }
        }
    }
    
    locales
}

/// Parse a semicolon-separated list into a vector
fn parse_semicolon_list(value: &str) -> Vec<String> {
    value.split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Get the best localized value for a key
fn get_localized_value(key: &str, value: &str, existing_value: &Option<String>, locales: &[String]) -> Option<String> {
    if let Some(bracket_pos) = key.find('[') {
        let locale_part = &key[bracket_pos + 1..key.len() - 1];
        
        // Only process localized versions of the key we care about
        if existing_value.is_none() {
            // If we don't have any value yet, check if this locale matches
            if locales.contains(&locale_part.to_string()) {
                return Some(value.to_string());
            }
        }
        None
    } else {
        // Non-localized key, use as fallback if no localized version was found
        if existing_value.is_none() {
            Some(value.to_string())
        } else {
            None
        }
    }
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
        
        // load pinned status
        let pinned_apps = super::helpers::load_pinned_apps(&self.db);
        app.pinned = pinned_apps.contains(&app.name);
        
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
pub fn read_with_options(
    dirs: Vec<impl Into<path::PathBuf>>,
    db: &sled::Db,
    filter_desktop: bool,
    list_executables: bool,
) -> mpsc::Receiver<App> {
    let (sender, receiver) = mpsc::channel();

    let dirs: Vec<path::PathBuf> = dirs.into_iter().map(Into::into).collect();
    let db = AppHistory { db: db.clone() };
    
    // Get current desktop environment for filtering
    let current_desktop = if filter_desktop {
        env::var("XDG_CURRENT_DESKTOP")
            .ok()
            .map(|d| d.split(':').map(|s| s.to_string()).collect::<Vec<_>>())
    } else {
        None
    };

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

        // Process desktop files
        for file_path in desktop_files {
            if let Ok(contents) = fs::read_to_string(&file_path) {
                // Skip files without [Desktop Entry] section early
                if !contents.contains("[Desktop Entry]") {
                    continue;
                }
                
                if let Ok(mut app) = App::parse(&contents, None) {
                    // Set desktop ID from file path
                    if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
                        app.desktop_id = Some(file_name.to_string());
                    }
                    
                    // Filter by OnlyShowIn/NotShowIn if enabled
                    if let Some(ref desktops) = current_desktop {
                        // Check NotShowIn first
                        if !app.not_show_in.is_empty() {
                            let should_hide = app.not_show_in.iter()
                                .any(|d| desktops.iter().any(|cd| cd.eq_ignore_ascii_case(d)));
                            if should_hide {
                                continue;
                            }
                        }
                        
                        // Check OnlyShowIn
                        if !app.only_show_in.is_empty() {
                            let should_show = app.only_show_in.iter()
                                .any(|d| desktops.iter().any(|cd| cd.eq_ignore_ascii_case(d)));
                            if !should_show {
                                continue;
                            }
                        }
                    }
                    
                    let app_with_history = db.get(app.clone());
                    
                    // Handle actions
                    if let Some(actions) = &app.actions {
                        for action in actions {
                            let ac = Action::default().name(action).from(app.name.clone());
                            if let Ok(mut a) = App::parse(&contents, Some(&ac)) {
                                // Set desktop ID for action too
                                if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
                                    a.desktop_id = Some(format!("{}#{}", file_name, action));
                                }
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
        
        // Add executables from PATH if requested
        if list_executables {
            if let Ok(path_var) = env::var("PATH") {
                let mut seen_executables = std::collections::HashSet::new();
                
                for path_dir in path_var.split(':') {
                    if let Ok(entries) = fs::read_dir(path_dir) {
                        for entry in entries.filter_map(Result::ok) {
                            let path = entry.path();
                            
                            // Check if it's an executable file
                            if path.is_file() {
                                #[cfg(unix)]
                                {
                                    use std::os::unix::fs::PermissionsExt;
                                    if let Ok(metadata) = fs::metadata(&path) {
                                        let permissions = metadata.permissions();
                                        // Check if executable bit is set
                                        if permissions.mode() & 0o111 != 0 {
                                            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                                                // Avoid duplicates
                                                if seen_executables.insert(file_name.to_string()) {
                                                    let app = App {
                                                        name: file_name.to_string(),
                                                        command: path.to_string_lossy().to_string(),
                                                        description: format!("Executable: {}", file_name),
                                                        generic_name: None,
                                                        keywords: vec![],
                                                        categories: vec!["Executable".to_string()],
                                                        mime_types: vec![],
                                                        icon: None,
                                                        is_terminal: false,
                                                        path: None,
                                                        only_show_in: vec![],
                                                        not_show_in: vec![],
                                                        hidden: false,
                                                        startup_notify: false,
                                                        startup_wm_class: None,
                                                        try_exec: None,
                                                        entry_type: "Application".to_string(),
                                                        actions: None,
                                                        desktop_id: None,
                                                        history: 0,
                                                        score: 0,
                                                        pinned: false,
                                                    };
                                                    
                                                    let app_with_history = db.get(app);
                                                    if sender.send(app_with_history).is_err() {
                                                        return;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        drop(sender);
    });

    receiver
}

/// An XDG Specification App with full XDG Desktop Entry support
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct App {
    /// App name (Name field)
    pub name: String,
    /// Command to run (Exec field)
    pub command: String,
    /// App description/comment (Comment field)
    pub description: String,
    /// Generic name of application (GenericName field)
    pub generic_name: Option<String>,
    /// Keywords for searching (Keywords field)
    pub keywords: Vec<String>,
    /// Categories this application belongs to (Categories field)
    pub categories: Vec<String>,
    /// MIME types this application can handle (MimeType field)
    pub mime_types: Vec<String>,
    /// Icon name or path (Icon field)
    pub icon: Option<String>,
    /// Run in terminal (Terminal field)
    pub is_terminal: bool,
    /// Path from which to run the command (Path field)
    pub path: Option<String>,
    /// Show only in these DEs (OnlyShowIn field)
    pub only_show_in: Vec<String>,
    /// Hide in these DEs (NotShowIn field)
    pub not_show_in: Vec<String>,
    /// Whether the app is hidden (Hidden field)
    pub hidden: bool,
    /// Application startup notification (StartupNotify field)
    pub startup_notify: bool,
    /// WM class for startup notification (StartupWMClass field)
    pub startup_wm_class: Option<String>,
    /// Command to test if executable exists (TryExec field)
    pub try_exec: Option<String>,
    /// Desktop Entry type (usually "Application")
    pub entry_type: String,
    /// Desktop file ID for tracking
    pub desktop_id: Option<String>,
    
    /// Matching score (used in [UI](super::ui::UI))
    /// Not part of the specification
    pub score: i64,
    /// Number of times this app was run
    /// Not part of the specification  
    pub history: u64,
    /// Whether this app is pinned/favorited
    /// Not part of the specification
    pub pinned: bool,

    // Private field for internal use only
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
        // pinned apps always come first
        match (self.pinned, other.pinned) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }
        
        // then sort by score, highest to lowest
        self.corrected_score()
            .cmp(&other.corrected_score())
            .reverse()
            // then sort alphabetically
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

// Display apps in list
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
    /// Parse an application with full XDG Desktop Entry specification support
    /// Includes localization, all standard fields, and proper validation
    pub fn parse<T: AsRef<str>>(contents: T, action: Option<&Action>) -> eyre::Result<App> {
        let contents: &str = contents.as_ref();
        let locales = get_locale();
        
        let pattern = if let Some(a) = &action {
            if a.name.is_empty() {
                return Err(eyre!("Action is empty"));
            }
            format!("[Desktop Action {}]", a.name)
        } else {
            "[Desktop Entry]".to_string()
        };

        // Initialize all fields
        let mut name = None;
        let mut generic_name = None;
        let mut exec = None;
        let mut description = None;
        let mut keywords = Vec::new();
        let mut categories = Vec::new();
        let mut mime_types = Vec::new();
        let mut icon = None;
        let mut terminal_exec = false;
        let mut path = None;
        let mut only_show_in = Vec::new();
        let mut not_show_in = Vec::new();
        let mut hidden = false;
        let mut no_display = false;
        let mut startup_notify = false;
        let mut startup_wm_class = None;
        let mut try_exec = None;
        let mut entry_type = None;
        let mut actions = None;

        let mut search = false;

        // Parse desktop entry with full XDG specification support
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { 
                continue; 
            }
            
            // Stop parsing when we hit another section
            if line.starts_with('[') && search && line != pattern {
                break;
            }

            if line == pattern {
                search = true;
                continue;
            }

            if search {
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    let value = value.trim();
                    
                    // Handle both localized and non-localized keys
                    let base_key = key.split('[').next().unwrap_or(key);
                    
                    match base_key {
                        "Type" => {
                            entry_type = Some(value.to_string());
                        }
                        "Name" => {
                            if let Some(val) = get_localized_value(key, value, &name, &locales) {
                                name = Some(if let Some(a) = &action {
                                    format!("{} ({})", &a.from, val)
                                } else {
                                    val
                                });
                            }
                        }
                        "GenericName" => {
                            if let Some(val) = get_localized_value(key, value, &generic_name, &locales) {
                                generic_name = Some(val);
                            }
                        }
                        "Comment" => {
                            if let Some(val) = get_localized_value(key, value, &description, &locales) {
                                description = Some(val);
                            }
                        }
                        "Keywords" => {
                            if keywords.is_empty() {
                                keywords = parse_semicolon_list(value);
                            }
                        }
                        "Categories" => {
                            if categories.is_empty() {
                                categories = parse_semicolon_list(value);
                            }
                        }
                        "MimeType" => {
                            if mime_types.is_empty() {
                                mime_types = parse_semicolon_list(value);
                            }
                        }
                        "Icon" => {
                            if icon.is_none() {
                                icon = Some(value.to_string());
                            }
                        }
                        "Terminal" => {
                            terminal_exec = value.eq_ignore_ascii_case("true");
                        }
                        "Exec" => {
                            if exec.is_none() {
                                // Remove XDG field codes (%f, %F, %u, %U, etc.)
                                let cleaned = value.split_whitespace()
                                    .filter(|part| !part.starts_with('%'))
                                    .collect::<Vec<_>>()
                                    .join(" ");
                                exec = Some(cleaned);
                            }
                        }
                        "Path" => {
                            if path.is_none() {
                                path = Some(value.to_string());
                            }
                        }
                        "TryExec" => {
                            if try_exec.is_none() {
                                try_exec = Some(value.to_string());
                            }
                        }
                        "OnlyShowIn" => {
                            if only_show_in.is_empty() {
                                only_show_in = parse_semicolon_list(value);
                            }
                        }
                        "NotShowIn" => {
                            if not_show_in.is_empty() {
                                not_show_in = parse_semicolon_list(value);
                            }
                        }
                        "Hidden" => {
                            hidden = value.eq_ignore_ascii_case("true");
                        }
                        "NoDisplay" => {
                            no_display = value.eq_ignore_ascii_case("true");
                        }
                        "StartupNotify" => {
                            startup_notify = value.eq_ignore_ascii_case("true");
                        }
                        "StartupWMClass" => {
                            if startup_wm_class.is_none() {
                                startup_wm_class = Some(value.to_string());
                            }
                        }
                        "Actions" => {
                            if actions.is_none() && action.is_none() {
                                actions = Some(parse_semicolon_list(value));
                            }
                        }
                        _ => {} // Ignore unknown keys
                    }
                }
            }
        }

        // Validate required fields according to XDG spec
        let entry_type = entry_type.unwrap_or_else(|| "Application".to_string());
        if entry_type != "Application" {
            return Err(eyre!("Not an Application type desktop entry"));
        }

        let name = name.unwrap_or_else(|| "Unknown".to_string());
        
        if exec.is_none() {
            return Err(eyre!("Missing required Exec field"));
        }
        let command = exec.unwrap();

        // Skip hidden apps
        if hidden || no_display {
            return Err(eyre!("Application is hidden"));
        }

        // Check desktop environment filtering
        let current_desktop = env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
        if !only_show_in.is_empty() {
            if !current_desktop.is_empty() && 
               !only_show_in.iter().any(|de| current_desktop.contains(de)) {
                return Err(eyre!("Application not shown in current desktop environment"));
            }
        }
        if !not_show_in.is_empty() && !current_desktop.is_empty() {
            if not_show_in.iter().any(|de| current_desktop.contains(de)) {
                return Err(eyre!("Application explicitly hidden in current desktop environment"));
            }
        }

        // Validate TryExec if present
        if let Some(ref try_exec_cmd) = try_exec {
            if which::which(try_exec_cmd).is_err() {
                return Err(eyre!("TryExec command not found: {}", try_exec_cmd));
            }
        }

        Ok(App {
            score: 0,
            history: 0,
            pinned: false,
            name,
            command,
            description: description.unwrap_or_default(),
            generic_name,
            keywords,
            categories,
            mime_types,
            icon,
            is_terminal: terminal_exec,
            path,
            only_show_in,
            not_show_in,
            hidden,
            startup_notify,
            startup_wm_class,
            try_exec,
            entry_type,
            desktop_id: None,
            actions,
        })
    }
}

/// An app action
///
/// In fsel every action is some app, with the action name in parentheses
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
