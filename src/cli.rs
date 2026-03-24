#![allow(clippy::field_reassign_with_default)]
use std::env;
use std::path;
use std::sync::atomic::AtomicBool;

pub static DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum MatchMode {
    Exact,
    #[default]
    Fuzzy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RankingMode {
    #[default]
    Frecency,
    Recency,
    Frequency,
}

impl RankingMode {
    pub fn as_str(self) -> &'static str {
        match self {
            RankingMode::Frecency => "frecency",
            RankingMode::Recency => "recency",
            RankingMode::Frequency => "frequency",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PinnedOrderMode {
    #[default]
    Ranking,
    Alphabetical,
    OldestPinned,
    NewestPinned,
}

impl PinnedOrderMode {
    pub fn as_str(self) -> &'static str {
        match self {
            PinnedOrderMode::Ranking => "ranking",
            PinnedOrderMode::Alphabetical => "alphabetical",
            PinnedOrderMode::OldestPinned => "oldest_pinned",
            PinnedOrderMode::NewestPinned => "newest_pinned",
        }
    }
}

fn usage() -> ! {
    let cmd = env::args().next().unwrap_or_else(|| "fsel".to_string());

    println!(
        "fsel — Fast terminal application launcher
Usage:
  {cmd} [OPTIONS]

├─ Core Modes
│  ├─ -p, --program <NAME>         Launch one app immediately and skip the TUI
│  ├─ --dmenu                      Read choices from stdin and print the selection
│  └─ --cclip                      Browse clipboard history and copy the selection
│
├─ Common Flags
│  ├─ -c, --config <FILE>          Read config from FILE instead of ~/.config/fsel/config.toml
│  ├─ -r, --replace                Replace an existing fsel/cclip instance before starting
│  ├─ -d, --detach                 Start launched apps without keeping this terminal attached
│  ├─ -t, --tty                    Run terminal apps in this TTY instead of a terminal launcher
│  ├─ -v, --verbose                Print more diagnostics; repeat as -vv or -vvv for more detail
│  ├─ -T, --test                   Enable debug logging and imply maximum verbosity
│  └─ -ss <SEARCH>                 Pre-fill the search box; place this last on the command line
│
├─ Launch Methods
│  ├─ --no-exec                    Print the selected item instead of launching it
│  ├─ --launch-prefix <CMD>        Prefix launches with a custom command such as 'runapp --'
│  ├─ --systemd-run                Launch through systemd-run --user --scope
│  └─ --uwsm                       Launch through uwsm app --
│
├─ App Launcher Extras
│  ├─ --clear-history              Delete launch history, then exit
│  ├─ --clear-cache                Delete the desktop entry cache, then exit
│  ├─ --refresh-cache              Rescan desktop entries before showing results
│  ├─ --filter-desktop[=no]        Respect OnlyShowIn/NotShowIn; pass =no to ignore them
│  ├─ --hide-before-typing         Keep the list hidden until you type the first character
│  ├─ --list-executables-in-path   Include executables from $PATH in launcher mode
│  ├─ --match-mode <MODE>          Choose fuzzy or exact matching
│  └─ --prefix-depth <N>           Tune how long prefix matches outrank fuzzy matches
│
├─ Mode-Specific Flags
│  ├─ Dmenu: --dmenu0 --password[=CHAR] --index --with-nth --accept-nth
│  ├─        --match-nth --delimiter --only-match --exit-if-empty
│  ├─        --select --select-index --auto-select --prompt-only
│  └─ Cclip: --tag <NAME|list|clear|wipe> --cclip-show-tag-color-names
│
└─ Help
   ├─ -h                           Show this summary
   ├─ -H, --help                   Show the full option tree with notes
   └─ -V, --version                Print the version and exit
",
        cmd = cmd
    );
    std::process::exit(0);
}

fn detailed_usage() -> ! {
    let cmd = env::args().next().unwrap_or_else(|| "fsel".to_string());

    println!(
        "fsel — Fast terminal application launcher
Usage:
  {cmd} [OPTIONS]

├─ Core Modes
│  ├─ -p, --program <NAME>         Launch one app immediately and skip the TUI
│  ├─ --cclip                      Browse clipboard history and copy the selected item
│  └─ --dmenu                      Read choices from stdin and print the selection to stdout
│
├─ Startup and Output
│  ├─ -c, --config <FILE>          Read config from FILE before applying CLI overrides
│  ├─ -r, --replace                Replace an existing fsel/cclip instance before starting
│  ├─ -d, --detach                 Start launched GUI apps without keeping this terminal attached
│  ├─ -t, --tty                    Run terminal apps in this TTY and replace the fsel process
│  ├─ -v, --verbose                Print more diagnostics; repeat as -vv or -vvv for more detail
│  ├─ -T, --test                   Enable debug logging, write logs under ~/.config/fsel/logs/, and imply -vvv
│  ├─ --no-exec                    Print the selected item instead of launching it
│  └─ -ss <SEARCH>                 Pre-fill the search box; place this last so it captures the rest
│
├─ Launch Methods
│  ├─ --launch-prefix <CMD>        Prefix launches with a custom command such as 'runapp --'
│  ├─ --systemd-run                Launch through systemd-run --user --scope
│  └─ --uwsm                       Launch through uwsm app --
│
├─ App Launcher Tuning
│  ├─ --clear-history              Delete launch history, then exit
│  ├─ --clear-cache                Delete the desktop entry cache, then exit
│  ├─ --refresh-cache              Rescan desktop entries before showing results
│  ├─ --filter-desktop[=no]        Respect OnlyShowIn/NotShowIn; pass =no to ignore them
│  ├─ --hide-before-typing         Keep the list hidden until you type the first character
│  ├─ --list-executables-in-path   Include executables from $PATH in launcher mode
│  ├─ --match-mode <MODE>          Choose fuzzy or exact matching (default: fuzzy)
│  └─ --prefix-depth <N>           Set how long prefix matches outrank fuzzy matches (default: 3)
│
├─ Dmenu Mode Options
│  ├─ --dmenu0                     Read NUL-separated input instead of newline-separated input
│  ├─ --password[=CHAR]            Mask typed input; optionally choose the mask character
│  ├─ --index                      Print the selected row index instead of the row text
│  ├─ --with-nth <COLS>            Show only these 1-based columns (example: 1,3)
│  ├─ --accept-nth <COLS>          Print only these columns after selection
│  ├─ --match-nth <COLS>           Search only within these columns
│  ├─ --delimiter <CHAR>           Split columns on CHAR instead of spaces
│  ├─ --only-match                 Reject custom text and require a selection from stdin
│  ├─ --exit-if-empty              Quit immediately when stdin provides no items
│  ├─ --select <STRING>            Start with the first matching row preselected
│  ├─ --select-index <N>           Start with row N preselected
│  ├─ --auto-select                Accept automatically when the filtered list reaches one row
│  └─ --prompt-only                Show only the input prompt and hide the list pane
│
├─ Clipboard Mode Options
│  ├─ --tag <NAME>                 Show only clipboard entries tagged NAME
│  ├─ --tag list                   List known tags, then exit
│  ├─ --tag list <NAME>            List clipboard entries carrying NAME, then exit
│  ├─ --tag clear                  Remove stored tag metadata
│  ├─ --tag wipe                   Remove all tags from every clipboard entry
│  └─ --cclip-show-tag-color-names Show tag color names next to tags in cclip mode
│
├─ General
│  ├─ -h                           Show the short summary
│  ├─ -H, --help                   Show this full option tree
│  └─ -V, --version                Print the version and exit
│
└─ Notes
   ├─ Pick only one launch method: --launch-prefix, --systemd-run, or --uwsm
   ├─ --dmenu and --cclip both imply --no-exec
   ├─ --select and --select-index cannot be combined
   └─ Default config path: ~/.config/fsel/config.toml
",
        cmd = cmd
    );
    std::process::exit(0);
}

/// Command line interface.
#[derive(Debug)]
pub struct Opts {
    /// Highlight color used in the UI
    pub highlight_color: ratatui::style::Color,
    /// Clear the history database
    pub clear_history: bool,
    /// Clear the desktop file cache
    pub clear_cache: bool,
    /// Force refresh of desktop file list
    pub refresh_cache: bool,
    /// Command to run Terminal=true apps
    pub terminal_launcher: String,
    /// Replace already running instance of Fsel
    pub replace: bool,
    /// Cursor character for the search
    pub cursor: String,
    /// Verbosity level
    pub verbose: Option<u64>,
    /// Don't scroll past the last/first item
    pub hard_stop: bool,
    /// Disable mouse input in all modes
    pub disable_mouse: bool,
    /// Print selected application to stdout instead of launching
    pub no_exec: bool,
    /// Launch applications using a custom command prefix
    pub launch_prefix: Vec<String>,
    /// True when launch_prefix was explicitly configured as a custom prefix
    pub launch_prefix_set: bool,
    /// Launch applications using systemd-run --user --scope
    pub systemd_run: bool,
    /// Launch applications using uwsm app
    pub uwsm: bool,
    /// Detach launched applications from terminal session
    pub detach: bool,
    /// Use rounded borders
    pub rounded_borders: bool,
    /// Border colors for different panels
    pub main_border_color: ratatui::style::Color,
    pub apps_border_color: ratatui::style::Color,
    pub input_border_color: ratatui::style::Color,
    /// Text colors for different panels
    pub main_text_color: ratatui::style::Color,
    pub apps_text_color: ratatui::style::Color,
    pub input_text_color: ratatui::style::Color,
    /// Enable fancy mode (show selected app name in borders)
    pub fancy_mode: bool,
    /// Color for panel header titles
    pub header_title_color: ratatui::style::Color,
    /// Color for pin icon
    pub pin_color: ratatui::style::Color,
    /// Pin icon character
    pub pin_icon: String,
    /// Keybinds configuration
    pub keybinds: crate::ui::Keybinds,
    /// Layout configuration
    pub title_panel_height_percent: u16,
    pub input_panel_height: u16,
    pub title_panel_position: Option<PanelPosition>,
    /// Program name for direct launch (bypasses TUI)
    pub program: Option<String>,
    /// Search string to pre-populate in TUI
    pub search_string: Option<String>,
    /// Confirm before launching app with -p if it has no history
    pub confirm_first_launch: bool,
    /// Dmenu mode settings
    pub dmenu_mode: bool,
    pub dmenu_with_nth: Option<Vec<usize>>,
    pub dmenu_delimiter: String,
    pub dmenu_show_line_numbers: bool,
    pub dmenu_wrap_long_lines: bool,
    pub dmenu_null_separated: bool,
    pub dmenu_password_mode: bool,
    pub dmenu_password_character: String,
    pub dmenu_index_mode: bool,
    pub dmenu_accept_nth: Option<Vec<usize>>,
    pub dmenu_match_nth: Option<Vec<usize>>,
    pub dmenu_only_match: bool,
    pub dmenu_exit_if_empty: bool,
    pub dmenu_select: Option<String>,
    pub dmenu_select_index: Option<usize>,
    pub dmenu_auto_select: bool,
    pub dmenu_prompt_only: bool,
    pub dmenu_hide_before_typing: bool,
    /// Clipboard history mode settings
    pub cclip_mode: bool,
    /// Tag filter for cclip mode
    pub cclip_tag: Option<String>,
    /// List tags mode
    pub cclip_tag_list: bool,
    /// Clear all tags and metadata
    pub cclip_clear_tags: bool,
    /// Wipe ALL tags from cclip entries (cclip 3.2.0+)
    pub cclip_wipe_tags: bool,
    /// App launcher settings
    pub filter_desktop: bool,
    pub list_executables_in_path: bool,
    pub hide_before_typing: bool,
    pub match_mode: MatchMode,
    pub ranking_mode: RankingMode,
    pub pinned_order_mode: PinnedOrderMode,
    /// Dmenu-specific colors and layout (override regular mode when in dmenu)
    pub dmenu_highlight_color: Option<ratatui::style::Color>,
    pub dmenu_cursor: Option<String>,
    pub dmenu_hard_stop: Option<bool>,
    pub dmenu_rounded_borders: Option<bool>,
    pub dmenu_main_border_color: Option<ratatui::style::Color>,
    pub dmenu_items_border_color: Option<ratatui::style::Color>,
    pub dmenu_input_border_color: Option<ratatui::style::Color>,
    pub dmenu_main_text_color: Option<ratatui::style::Color>,
    pub dmenu_items_text_color: Option<ratatui::style::Color>,
    pub dmenu_input_text_color: Option<ratatui::style::Color>,
    pub dmenu_header_title_color: Option<ratatui::style::Color>,
    pub dmenu_title_panel_height_percent: Option<u16>,
    pub dmenu_input_panel_height: Option<u16>,
    pub dmenu_title_panel_position: Option<PanelPosition>,
    /// Cclip-specific colors and layout (inherit from dmenu, then regular mode)
    pub cclip_highlight_color: Option<ratatui::style::Color>,
    pub cclip_cursor: Option<String>,
    pub cclip_hard_stop: Option<bool>,
    pub cclip_rounded_borders: Option<bool>,
    pub cclip_main_border_color: Option<ratatui::style::Color>,
    pub cclip_items_border_color: Option<ratatui::style::Color>,
    pub cclip_input_border_color: Option<ratatui::style::Color>,
    pub cclip_main_text_color: Option<ratatui::style::Color>,
    pub cclip_items_text_color: Option<ratatui::style::Color>,
    pub cclip_input_text_color: Option<ratatui::style::Color>,
    pub cclip_header_title_color: Option<ratatui::style::Color>,
    pub cclip_title_panel_height_percent: Option<u16>,
    pub cclip_input_panel_height: Option<u16>,
    pub cclip_title_panel_position: Option<PanelPosition>,
    pub cclip_show_line_numbers: Option<bool>,
    pub cclip_wrap_long_lines: Option<bool>,
    pub cclip_image_preview: Option<bool>,
    pub cclip_hide_inline_image_message: Option<bool>,
    pub cclip_show_tag_color_names: Option<bool>,
    /// Dmenu-specific disable mouse option
    pub dmenu_disable_mouse: Option<bool>,
    /// Cclip-specific disable mouse option
    pub cclip_disable_mouse: Option<bool>,
    /// Character depth for prioritized prefix matching
    pub prefix_depth: usize,
    /// Enable full debug/test mode with detailed logging
    pub test_mode: bool,
    /// Launch in TTY mode (replace current process)
    pub tty: bool,
}

impl Default for Opts {
    fn default() -> Self {
        Self {
            highlight_color: ratatui::style::Color::LightBlue,
            clear_history: false,
            clear_cache: false,
            refresh_cache: false,
            terminal_launcher: "alacritty -e".to_string(),
            replace: false,
            cursor: "█".to_string(),
            verbose: None,
            hard_stop: false,
            disable_mouse: false,
            no_exec: false,
            launch_prefix: vec![],
            launch_prefix_set: false,
            systemd_run: false,
            uwsm: false,
            detach: false,
            rounded_borders: true,
            main_border_color: ratatui::style::Color::White,
            apps_border_color: ratatui::style::Color::White,
            input_border_color: ratatui::style::Color::White,
            main_text_color: ratatui::style::Color::White,
            apps_text_color: ratatui::style::Color::White,
            input_text_color: ratatui::style::Color::White,
            fancy_mode: false,
            header_title_color: ratatui::style::Color::White,
            pin_color: ratatui::style::Color::Rgb(255, 165, 0), // orange
            pin_icon: "📌".to_string(),
            keybinds: crate::ui::Keybinds::default(),
            title_panel_height_percent: 30,
            input_panel_height: 3,
            title_panel_position: None,
            program: None,
            search_string: None,
            confirm_first_launch: false,
            // Dmenu mode defaults
            dmenu_mode: false,
            dmenu_with_nth: None,
            dmenu_delimiter: " ".to_string(),
            dmenu_show_line_numbers: false,
            dmenu_wrap_long_lines: true,
            dmenu_null_separated: false,
            dmenu_password_mode: false,
            dmenu_password_character: "*".to_string(),
            dmenu_index_mode: false,
            dmenu_accept_nth: None,
            dmenu_match_nth: None,
            dmenu_only_match: false,
            dmenu_exit_if_empty: false,
            dmenu_select: None,
            dmenu_select_index: None,
            dmenu_auto_select: false,
            dmenu_prompt_only: false,
            dmenu_hide_before_typing: false,
            // Cclip mode defaults
            cclip_mode: false,
            cclip_tag: None,
            cclip_tag_list: false,
            cclip_clear_tags: false,
            cclip_wipe_tags: false,
            // App launcher defaults
            filter_desktop: true,
            list_executables_in_path: false,
            hide_before_typing: false,
            match_mode: MatchMode::Fuzzy,
            ranking_mode: RankingMode::Frecency,
            pinned_order_mode: PinnedOrderMode::Ranking,
            // Dmenu-specific styling (None means use regular mode values)
            dmenu_highlight_color: None,
            dmenu_cursor: None,
            dmenu_hard_stop: None,
            dmenu_rounded_borders: None,
            dmenu_main_border_color: None,
            dmenu_items_border_color: None,
            dmenu_input_border_color: None,
            dmenu_main_text_color: None,
            dmenu_items_text_color: None,
            dmenu_input_text_color: None,
            dmenu_header_title_color: None,
            dmenu_title_panel_height_percent: None,
            dmenu_input_panel_height: None,
            dmenu_title_panel_position: None,
            // Cclip-specific styling (None means inherit from dmenu, then regular mode)
            cclip_highlight_color: None,
            cclip_cursor: None,
            cclip_hard_stop: None,
            cclip_rounded_borders: None,
            cclip_main_border_color: None,
            cclip_items_border_color: None,
            cclip_input_border_color: None,
            cclip_main_text_color: None,
            cclip_items_text_color: None,
            cclip_input_text_color: None,
            cclip_header_title_color: None,
            cclip_title_panel_height_percent: None,
            cclip_input_panel_height: None,
            cclip_title_panel_position: None,
            cclip_show_line_numbers: None,
            cclip_wrap_long_lines: None,
            cclip_image_preview: None,
            cclip_hide_inline_image_message: None,
            cclip_show_tag_color_names: None,
            dmenu_disable_mouse: None,
            cclip_disable_mouse: None,
            prefix_depth: 3,
            test_mode: false,
            tty: false,
        }
    }
}

/// Parses the cli arguments
pub fn parse() -> Result<Opts, lexopt::Error> {
    use lexopt::prelude::*;

    // 1. First pass to find config file location
    let mut config_path_cli: Option<path::PathBuf> = None;
    let mut args_for_config_check = env::args().skip(1);
    while let Some(arg) = args_for_config_check.next() {
        if (arg == "-c" || arg == "--config")
            && let Some(val) = args_for_config_check.next()
        {
            config_path_cli = Some(path::PathBuf::from(val));
        }
    }

    // 2. Load Config from Files / Env
    let fsel_config = match crate::config::FselConfig::new(config_path_cli.clone()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error loading configuration: {}", e);
            std::process::exit(1);
        }
    };

    // 3. Initialize Opts with defaults from Config
    #[allow(clippy::field_reassign_with_default)]
    let mut default = Opts::default();

    // Map General Config
    default.terminal_launcher = fsel_config.general.terminal_launcher;
    if default.terminal_launcher == "tty" {
        default.tty = true;
        default.terminal_launcher.clear();
    }
    default.filter_desktop = fsel_config.general.filter_desktop;
    default.list_executables_in_path = fsel_config.general.list_executables_in_path;
    default.hide_before_typing = fsel_config.general.hide_before_typing;
    default.match_mode = match fsel_config.general.match_mode.as_str() {
        "exact" => MatchMode::Exact,
        _ => MatchMode::Fuzzy,
    };
    default.ranking_mode =
        parse_ranking_mode(&fsel_config.general.ranking_mode).unwrap_or(RankingMode::Frecency);
    default.pinned_order_mode = parse_pinned_order_mode(&fsel_config.general.pinned_order)
        .unwrap_or(PinnedOrderMode::Ranking);
    default.systemd_run = fsel_config.general.systemd_run;
    default.uwsm = fsel_config.general.uwsm;
    default.detach = fsel_config.general.detach;
    default.no_exec = fsel_config.general.no_exec;
    default.confirm_first_launch = fsel_config.general.confirm_first_launch;
    default.prefix_depth = fsel_config.general.prefix_depth;
    if [default.systemd_run, default.uwsm]
        .iter()
        .filter(|&&x| x)
        .count()
        > 1
    {
        eprintln!("Error: Only one launch method can be specified at a time");
        eprintln!("Available methods: --systemd-run, --uwsm");
        std::process::exit(1);
    }
    if default.systemd_run {
        set_systemd_run(&mut default);
    }
    if default.uwsm {
        set_uwsm(&mut default);
    }

    // apply [app_launcher] section overrides if they exist
    if let Some(filter) = fsel_config.app_launcher.filter_desktop {
        default.filter_desktop = filter;
    }
    if let Some(list_exec) = fsel_config.app_launcher.list_executables_in_path {
        default.list_executables_in_path = list_exec;
    }
    if let Some(hide) = fsel_config.app_launcher.hide_before_typing {
        default.hide_before_typing = hide;
    }
    if let Some(prefix) = fsel_config.app_launcher.launch_prefix {
        set_launch_prefix(&mut default, prefix);
    }
    if let Some(ref mode) = fsel_config.app_launcher.match_mode {
        default.match_mode = match mode.as_str() {
            "exact" => MatchMode::Exact,
            _ => MatchMode::Fuzzy,
        };
    }
    if let Some(confirm) = fsel_config.app_launcher.confirm_first_launch {
        default.confirm_first_launch = confirm;
    }
    if let Some(depth) = fsel_config.app_launcher.prefix_depth {
        default.prefix_depth = depth;
    }
    if let Some(ref ranking_mode) = fsel_config.app_launcher.ranking_mode {
        default.ranking_mode = parse_ranking_mode(ranking_mode).unwrap_or(default.ranking_mode);
    }
    if let Some(ref pinned_order_mode) = fsel_config.app_launcher.pinned_order {
        default.pinned_order_mode =
            parse_pinned_order_mode(pinned_order_mode).unwrap_or(default.pinned_order_mode);
    }

    // Map UI Config
    if let Ok(c) = string_to_color(&fsel_config.ui.highlight_color) {
        default.highlight_color = c;
    }
    default.cursor = fsel_config.ui.cursor;
    default.hard_stop = fsel_config.ui.hard_stop;
    default.rounded_borders = fsel_config.ui.rounded_borders;
    default.disable_mouse = fsel_config.ui.disable_mouse;
    if let Ok(c) = string_to_color(&fsel_config.ui.main_border_color) {
        default.main_border_color = c;
    }
    if let Ok(c) = string_to_color(&fsel_config.ui.apps_border_color) {
        default.apps_border_color = c;
    }
    if let Ok(c) = string_to_color(&fsel_config.ui.input_border_color) {
        default.input_border_color = c;
    }
    if let Ok(c) = string_to_color(&fsel_config.ui.main_text_color) {
        default.main_text_color = c;
    }
    if let Ok(c) = string_to_color(&fsel_config.ui.apps_text_color) {
        default.apps_text_color = c;
    }
    if let Ok(c) = string_to_color(&fsel_config.ui.input_text_color) {
        default.input_text_color = c;
    }
    default.fancy_mode = fsel_config.ui.fancy_mode;
    if let Ok(c) = string_to_color(&fsel_config.ui.header_title_color) {
        default.header_title_color = c;
    }
    if let Ok(c) = string_to_color(&fsel_config.ui.pin_color) {
        default.pin_color = c;
    }
    default.pin_icon = fsel_config.ui.pin_icon;
    default.keybinds = fsel_config.ui.keybinds;

    // Map Layout Config
    default.title_panel_height_percent = fsel_config.layout.title_panel_height_percent;
    default.input_panel_height = fsel_config.layout.input_panel_height;
    default.title_panel_position = match fsel_config
        .layout
        .title_panel_position
        .to_lowercase()
        .as_str()
    {
        "bottom" => Some(PanelPosition::Bottom),
        "middle" => Some(PanelPosition::Middle),
        "top" => Some(PanelPosition::Top),
        _ => None,
    };

    // Map Dmenu Config
    if let Some(d) = fsel_config.dmenu.delimiter {
        default.dmenu_delimiter = d;
    }
    if let Some(c) = fsel_config.dmenu.password_character {
        default.dmenu_password_character = c;
    }
    if let Some(show_line_numbers) = fsel_config.dmenu.show_line_numbers {
        default.dmenu_show_line_numbers = show_line_numbers;
    }
    if let Some(wrap_long_lines) = fsel_config.dmenu.wrap_long_lines {
        default.dmenu_wrap_long_lines = wrap_long_lines;
    }
    if let Some(exit_if_empty) = fsel_config.dmenu.exit_if_empty {
        default.dmenu_exit_if_empty = exit_if_empty;
    }
    if let Some(disable_mouse) = fsel_config.dmenu.disable_mouse {
        default.dmenu_disable_mouse = Some(disable_mouse);
    }
    if let Some(hard_stop) = fsel_config.dmenu.hard_stop {
        default.dmenu_hard_stop = Some(hard_stop);
    }
    if let Some(rounded_borders) = fsel_config.dmenu.rounded_borders {
        default.dmenu_rounded_borders = Some(rounded_borders);
    }
    if let Some(cursor) = fsel_config.dmenu.cursor {
        default.dmenu_cursor = Some(cursor);
    }
    if let Some(color_str) = fsel_config.dmenu.highlight_color
        && let Ok(c) = string_to_color(&color_str)
    {
        default.dmenu_highlight_color = Some(c);
    }
    if let Some(color_str) = fsel_config.dmenu.main_border_color
        && let Ok(c) = string_to_color(&color_str)
    {
        default.dmenu_main_border_color = Some(c);
    }
    if let Some(color_str) = fsel_config.dmenu.items_border_color
        && let Ok(c) = string_to_color(&color_str)
    {
        default.dmenu_items_border_color = Some(c);
    }
    if let Some(color_str) = fsel_config.dmenu.input_border_color
        && let Ok(c) = string_to_color(&color_str)
    {
        default.dmenu_input_border_color = Some(c);
    }
    if let Some(color_str) = fsel_config.dmenu.main_text_color
        && let Ok(c) = string_to_color(&color_str)
    {
        default.dmenu_main_text_color = Some(c);
    }
    if let Some(color_str) = fsel_config.dmenu.items_text_color
        && let Ok(c) = string_to_color(&color_str)
    {
        default.dmenu_items_text_color = Some(c);
    }
    if let Some(color_str) = fsel_config.dmenu.input_text_color
        && let Ok(c) = string_to_color(&color_str)
    {
        default.dmenu_input_text_color = Some(c);
    }
    if let Some(color_str) = fsel_config.dmenu.header_title_color
        && let Ok(c) = string_to_color(&color_str)
    {
        default.dmenu_header_title_color = Some(c);
    }
    if let Some(height) = fsel_config.dmenu.title_panel_height_percent {
        default.dmenu_title_panel_height_percent = Some(height);
    }
    if let Some(height) = fsel_config.dmenu.input_panel_height {
        default.dmenu_input_panel_height = Some(height);
    }
    if let Some(position_str) = fsel_config.dmenu.title_panel_position {
        default.dmenu_title_panel_position = match position_str.as_str() {
            "bottom" => Some(PanelPosition::Bottom),
            _ => None,
        };
    }

    // Map Cclip Config
    if let Some(image_preview) = fsel_config.cclip.image_preview {
        default.cclip_image_preview = Some(image_preview);
    }
    if let Some(hide_inline_image_message) = fsel_config.cclip.hide_inline_image_message {
        default.cclip_hide_inline_image_message = Some(hide_inline_image_message);
    }
    if let Some(show_tag_color_names) = fsel_config.cclip.show_tag_color_names {
        default.cclip_show_tag_color_names = Some(show_tag_color_names);
    }
    if let Some(show_line_numbers) = fsel_config.cclip.show_line_numbers {
        default.cclip_show_line_numbers = Some(show_line_numbers);
    }
    if let Some(wrap_long_lines) = fsel_config.cclip.wrap_long_lines {
        default.cclip_wrap_long_lines = Some(wrap_long_lines);
    }
    if let Some(disable_mouse) = fsel_config.cclip.disable_mouse {
        default.cclip_disable_mouse = Some(disable_mouse);
    }
    if let Some(hard_stop) = fsel_config.cclip.hard_stop {
        default.cclip_hard_stop = Some(hard_stop);
    }
    if let Some(rounded_borders) = fsel_config.cclip.rounded_borders {
        default.cclip_rounded_borders = Some(rounded_borders);
    }
    if let Some(cursor) = fsel_config.cclip.cursor {
        default.cclip_cursor = Some(cursor);
    }
    if let Some(color_str) = fsel_config.cclip.highlight_color
        && let Ok(c) = string_to_color(&color_str)
    {
        default.cclip_highlight_color = Some(c);
    }
    if let Some(color_str) = fsel_config.cclip.main_border_color
        && let Ok(c) = string_to_color(&color_str)
    {
        default.cclip_main_border_color = Some(c);
    }
    if let Some(color_str) = fsel_config.cclip.items_border_color
        && let Ok(c) = string_to_color(&color_str)
    {
        default.cclip_items_border_color = Some(c);
    }
    if let Some(color_str) = fsel_config.cclip.input_border_color
        && let Ok(c) = string_to_color(&color_str)
    {
        default.cclip_input_border_color = Some(c);
    }
    if let Some(color_str) = fsel_config.cclip.main_text_color
        && let Ok(c) = string_to_color(&color_str)
    {
        default.cclip_main_text_color = Some(c);
    }
    if let Some(color_str) = fsel_config.cclip.items_text_color
        && let Ok(c) = string_to_color(&color_str)
    {
        default.cclip_items_text_color = Some(c);
    }
    if let Some(color_str) = fsel_config.cclip.input_text_color
        && let Ok(c) = string_to_color(&color_str)
    {
        default.cclip_input_text_color = Some(c);
    }
    if let Some(color_str) = fsel_config.cclip.header_title_color
        && let Ok(c) = string_to_color(&color_str)
    {
        default.cclip_header_title_color = Some(c);
    }
    if let Some(height) = fsel_config.cclip.title_panel_height_percent {
        default.cclip_title_panel_height_percent = Some(height);
    }
    if let Some(height) = fsel_config.cclip.input_panel_height {
        default.cclip_input_panel_height = Some(height);
    }
    if let Some(position_str) = fsel_config.cclip.title_panel_position {
        default.cclip_title_panel_position = match position_str.as_str() {
            "bottom" => Some(PanelPosition::Bottom),
            _ => None,
        };
    }

    // 4. Parse CLI Overrides
    let mut parser = lexopt::Parser::from_env();
    let mut cli_launch_methods = 0;

    // Check for -ss option first and handle it specially
    let args: Vec<String> = env::args().collect();
    if let Some(ss_pos) = args.iter().position(|arg| arg == "-ss") {
        // Everything after -ss is the search string
        if ss_pos + 1 < args.len() {
            let search_parts: Vec<String> = args[ss_pos + 1..].to_vec();
            default.search_string = Some(search_parts.join(" "));

            // Create a new parser without the -ss and search string parts
            let filtered_args: Vec<String> = args[..ss_pos].to_vec();
            parser = lexopt::Parser::from_args(filtered_args.into_iter().skip(1));
        } else {
            // -ss with no arguments, just set empty search string and open TUI normally
            default.search_string = Some(String::new());

            // Create a new parser without the -ss part
            let filtered_args: Vec<String> = args[..ss_pos].to_vec();
            parser = lexopt::Parser::from_args(filtered_args.into_iter().skip(1));
        }
    }

    // Check if invoked as dmenu (this should override config if present)
    if let Some(arg0) = env::args().next()
        && arg0.ends_with("dmenu")
    {
        default.dmenu_mode = true;
    }

    while let Some(arg) = parser.next()? {
        match arg {
            Short('t') | Long("tty") => {
                default.tty = true;
                default.terminal_launcher.clear();
            }
            Short('r') | Long("replace") => {
                default.replace = true;
            }
            Short('c') | Long("config") => {
                // Already handled in pre-pass, but consume it
                let _ = parser.value()?;
            }
            Long("clear-history") => {
                default.clear_history = true;
            }
            Long("clear-cache") => {
                default.clear_cache = true;
            }
            Long("refresh-cache") => {
                default.refresh_cache = true;
            }
            Long("no-exec") => {
                default.no_exec = true;
            }
            Long("launch-prefix") => {
                cli_launch_methods += 1;
                set_launch_prefix(
                    &mut default,
                    parse_launch_prefix(
                        &parser
                            .value()?
                            .into_string()
                            .map_err(|_| "Launch prefix must be valid UTF-8")?,
                    )
                    .map_err(lexopt::Error::from)?,
                );
            }
            Long("systemd-run") => {
                cli_launch_methods += 1;
                set_systemd_run(&mut default);
            }
            Long("uwsm") => {
                cli_launch_methods += 1;
                set_uwsm(&mut default);
            }
            Short('d') | Long("detach") => {
                default.detach = true;
            }
            Long("dmenu") => {
                default.dmenu_mode = true;
            }
            Long("cclip") => {
                default.cclip_mode = true;
            }
            Long("tag") => {
                let tag_arg = parser
                    .value()?
                    .into_string()
                    .map_err(|_| "Tag argument must be valid UTF-8")?;
                if tag_arg == "list" {
                    default.cclip_tag_list = true;
                    // Check if there's another argument for specific tag
                    let next_tag_value = parser.value();
                    if let Ok(val) = next_tag_value {
                        default.cclip_tag = Some(
                            val.into_string()
                                .map_err(|_| "Tag name must be valid UTF-8")?,
                        );
                    }
                } else if tag_arg == "clear" {
                    default.cclip_clear_tags = true;
                } else if tag_arg == "wipe" {
                    default.cclip_wipe_tags = true;
                } else {
                    default.cclip_tag = Some(tag_arg);
                }
            }
            Long("cclip-show-tag-color-names") => {
                default.cclip_show_tag_color_names = Some(true);
            }
            Long("dmenu0") => {
                default.dmenu_mode = true;
                default.dmenu_null_separated = true;
            }
            Long("password") => {
                let val = parser.optional_value();
                default.dmenu_password_mode = true;
                if let Some(v) = val {
                    default.dmenu_password_character = v
                        .into_string()
                        .map_err(|_| "Password character must be valid UTF-8")?;
                }
            }
            Long("index") => {
                default.dmenu_index_mode = true;
            }
            Long("accept-nth") => {
                let cols_str = parser
                    .value()?
                    .into_string()
                    .map_err(|_| "Column specification must be valid UTF-8")?;
                let cols: Result<Vec<usize>, _> = cols_str
                    .split(',')
                    .map(|s| s.trim().parse::<usize>())
                    .collect();
                default.dmenu_accept_nth = Some(cols.map_err(|_| "Invalid column specification")?);
            }
            Long("match-nth") => {
                let cols_str = parser
                    .value()?
                    .into_string()
                    .map_err(|_| "Column specification must be valid UTF-8")?;
                let cols: Result<Vec<usize>, _> = cols_str
                    .split(',')
                    .map(|s| s.trim().parse::<usize>())
                    .collect();
                default.dmenu_match_nth = Some(cols.map_err(|_| "Invalid column specification")?);
            }
            Long("only-match") => {
                default.dmenu_only_match = true;
            }
            Long("exit-if-empty") => {
                default.dmenu_exit_if_empty = true;
            }
            Long("select") => {
                default.dmenu_select = Some(
                    parser
                        .value()?
                        .into_string()
                        .map_err(|_| "Select string must be valid UTF-8")?,
                );
            }
            Long("select-index") => {
                let idx_str = parser
                    .value()?
                    .into_string()
                    .map_err(|_| "Index must be valid UTF-8")?;
                default.dmenu_select_index =
                    Some(idx_str.parse::<usize>().map_err(|_| "Invalid index")?);
            }
            Long("auto-select") => {
                default.dmenu_auto_select = true;
            }
            Long("prompt-only") => {
                default.dmenu_prompt_only = true;
            }
            Long("hide-before-typing") => {
                default.hide_before_typing = true;
            }
            Long("filter-desktop") => {
                let val = parser.optional_value();
                if let Some(v) = val {
                    let v_str = v
                        .into_string()
                        .map_err(|_| "filter-desktop value must be valid UTF-8")?;
                    default.filter_desktop = v_str != "no";
                } else {
                    default.filter_desktop = true;
                }
            }
            Long("list-executables-in-path") => {
                default.list_executables_in_path = true;
            }
            Long("match-mode") => {
                let mode_str = parser
                    .value()?
                    .into_string()
                    .map_err(|_| "Match mode must be valid UTF-8")?;
                default.match_mode = match mode_str.as_str() {
                    "exact" => MatchMode::Exact,
                    "fuzzy" => MatchMode::Fuzzy,
                    _ => return Err("Invalid match mode. Use 'exact' or 'fuzzy'".into()),
                };
            }
            Long("prefix-depth") => {
                let depth_str = parser
                    .value()?
                    .into_string()
                    .map_err(|_| "Prefix depth must be valid UTF-8")?;
                default.prefix_depth = depth_str
                    .parse::<usize>()
                    .map_err(|_| "Invalid prefix depth")?;
            }
            Short('T') | Long("test") => {
                default.test_mode = true;
                default.verbose = Some(3);
            }
            Long("with-nth") => {
                let cols_str = parser
                    .value()?
                    .into_string()
                    .map_err(|_| "Column specification must be valid UTF-8")?;
                let cols: Result<Vec<usize>, _> = cols_str
                    .split(',')
                    .map(|s| s.trim().parse::<usize>())
                    .collect();
                default.dmenu_with_nth = Some(cols.map_err(
                    |_| "Invalid column specification. Use comma-separated numbers like: 1,2,4",
                )?);
            }
            Long("delimiter") => {
                default.dmenu_delimiter = parser
                    .value()?
                    .into_string()
                    .map_err(|_| "Delimiter must be valid UTF-8")?;
            }
            Short('p') | Long("program") => {
                default.program = Some(
                    parser
                        .value()?
                        .into_string()
                        .map_err(|_| "Program name must be valid UTF-8")?,
                );
            }
            Short('v') | Long("verbose") => {
                if let Some(v) = default.verbose {
                    default.verbose = Some(v + 1);
                } else {
                    default.verbose = Some(1);
                }
            }
            Short('h') => {
                usage();
            }
            Short('H') | Long("help") => {
                detailed_usage();
            }
            Short('V') | Long("version") => {
                println!("{}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            _ => {
                // Handle common misspellings and provide helpful error messages
                let error_msg = match &arg {
                    Long(name) => match *name {
                        "clip" => "Unknown option '--clip'. Did you mean '--cclip'?",
                        "menu" => "Unknown option '--menu'. Did you mean '--dmenu'?",
                        "dme" | "dmen" => "Unknown option. Did you mean '--dmenu'?",
                        "cc" | "ccli" => "Unknown option. Did you mean '--cclip'?",
                        "help" => "Unknown option '--help'. Use '-h' or '--help'.",
                        "version" => "Unknown option '--version'. Use '-V' or '--version'.",
                        _ => "Unknown option. Use '-h' or '--help' to see available options.",
                    },
                    Short(c) => match c {
                        'C' => "Unknown option '-C'. Did you mean '-c' for --config?",
                        'P' => "Unknown option '-P'. Did you mean '-p' for --program?",
                        'R' => "Unknown option '-R'. Did you mean '-r' for --replace?",
                        'H' => "Unknown option '-H'. Did you mean '-h' for --help?",
                        _ => "Unknown option. Use '-h' or '--help' to see available options.",
                    },
                    Value(_val) => {
                        // Unexpected value
                        return Err(arg.unexpected());
                    }
                };

                eprintln!("Error: {}", error_msg);
                eprintln!();
                eprintln!("Quick help:");
                eprintln!("  -c, --config <FILE>    Read config from FILE");
                eprintln!("  -p, --program <NAME>   Launch one app immediately and skip the TUI");
                eprintln!("  -ss <SEARCH>           Pre-fill the search box; place this last");
                eprintln!(
                    "  --dmenu                Read choices from stdin and print the selection"
                );
                eprintln!(
                    "  --cclip                Browse clipboard history and copy the selection"
                );
                eprintln!(
                    "  --no-exec              Print the selected item instead of launching it"
                );
                eprintln!("  -r, --replace          Replace an existing fsel/cclip instance");
                eprintln!(
                    "  -d, --detach           Start launched apps without keeping the terminal attached"
                );
                eprintln!("  -v, --verbose          Print more diagnostics; repeat as -vv or -vvv");
                eprintln!("  -h                     Show the short summary");
                eprintln!("  -H, --help             Show the full option tree");
                eprintln!("  -V, --version          Print the version and exit");
                eprintln!();
                eprintln!("Run 'fsel -h' for a summary or 'fsel --help' for the full option tree.");
                std::process::exit(1);
            }
        }
    }

    // Validate mutually exclusive options
    if default.program.is_some() && default.search_string.is_some() {
        eprintln!("Error: Cannot use -p/--program and -ss together");
        eprintln!("Use -p for direct launch or -ss for pre-filled TUI search");
        std::process::exit(1);
    }

    // Validate dmenu mode conflicts
    if default.dmenu_mode {
        if default.program.is_some() {
            eprintln!("Error: --dmenu cannot be used with -p/--program");
            eprintln!("Dmenu mode reads from stdin and outputs to stdout");
            std::process::exit(1);
        }
        // dmenu mode implies no-exec behavior
        default.no_exec = true;
    }

    // Validate prompt-only conflicts
    if default.dmenu_prompt_only && default.dmenu_mode {
        default.dmenu_show_line_numbers = false;
    }

    // Validate select conflicts
    if default.dmenu_select.is_some() && default.dmenu_select_index.is_some() {
        eprintln!("Error: Cannot use --select and --select-index together");
        std::process::exit(1);
    }

    // Validate cclip mode conflicts
    if default.cclip_mode {
        if default.program.is_some() {
            eprintln!("Error: --cclip cannot be used with -p/--program");
            eprintln!("Cclip mode browses clipboard history and copies selection");
            std::process::exit(1);
        }
        // cclip mode implies no-exec behavior
        default.no_exec = true;
    }

    // Validate tag options require cclip mode
    if (default.cclip_tag.is_some()
        || default.cclip_tag_list
        || default.cclip_clear_tags
        || default.cclip_wipe_tags)
        && !default.cclip_mode
    {
        eprintln!("Error: --tag requires --cclip mode");
        eprintln!("Use one of these forms:");
        eprintln!("  fsel --cclip --tag <name>");
        eprintln!("  fsel --cclip --tag list");
        eprintln!("  fsel --cclip --tag list <name>");
        eprintln!("  fsel --cclip --tag clear");
        eprintln!("  fsel --cclip --tag wipe");
        std::process::exit(1);
    }

    // Validate mutually exclusive special modes
    if default.dmenu_mode && default.cclip_mode {
        eprintln!("Error: --dmenu and --cclip cannot be used together");
        std::process::exit(1);
    }

    // Validate flag conflicts - no-exec overrides all launch methods
    if default.no_exec {
        if active_launch_method_count(&default) > 0 {
            eprintln!("Warning: --no-exec overrides other launch method flags");
        }
    } else {
        // Check for mutually exclusive launch methods
        if cli_launch_methods > 1 || active_launch_method_count(&default) > 1 {
            eprintln!("Error: Only one launch method can be specified at a time");
            eprintln!("Available methods: --launch-prefix, --systemd-run, --uwsm");
            std::process::exit(1);
        }
    }

    Ok(default)
}

// Re-export PanelPosition from ui module for backwards compatibility
pub use crate::ui::PanelPosition;

fn systemd_run_prefix() -> Vec<String> {
    vec!["systemd-run".into(), "--user".into(), "--scope".into()]
}

fn uwsm_prefix() -> Vec<String> {
    vec!["uwsm".into(), "app".into(), "--".into()]
}

fn clear_launch_method(opts: &mut Opts) {
    opts.systemd_run = false;
    opts.uwsm = false;
    opts.launch_prefix_set = false;
    opts.launch_prefix.clear();
}

fn set_launch_prefix(opts: &mut Opts, prefix: Vec<String>) {
    clear_launch_method(opts);
    opts.launch_prefix_set = !prefix.is_empty();
    opts.launch_prefix = prefix;
}

fn set_systemd_run(opts: &mut Opts) {
    clear_launch_method(opts);
    opts.systemd_run = true;
    opts.launch_prefix = systemd_run_prefix();
}

fn set_uwsm(opts: &mut Opts) {
    clear_launch_method(opts);
    opts.uwsm = true;
    opts.launch_prefix = uwsm_prefix();
}

fn active_launch_method_count(opts: &Opts) -> usize {
    [opts.systemd_run, opts.uwsm, opts.launch_prefix_set]
        .iter()
        .filter(|&&x| x)
        .count()
}

fn parse_launch_prefix(value: &str) -> Result<Vec<String>, &'static str> {
    let prefix =
        shell_words::split(value).map_err(|_| "Launch prefix must use valid shell syntax")?;
    if prefix.is_empty() {
        return Err("Launch prefix cannot be empty");
    }
    Ok(prefix)
}

/// Parses a [String] into a ratatui [color]
///
/// Case-insensitive
///
/// [String]: std::string::String
/// [color]: ratatui::style::Color
pub fn string_to_color<T: Into<String>>(val: T) -> Result<ratatui::style::Color, &'static str> {
    let color_str = val.into();
    let color_lower = color_str.to_lowercase();

    // Try hex color first (e.g., "#ff0000" or "ff0000")
    if let Some(hex_color) = parse_hex_color(&color_str) {
        return Ok(hex_color);
    }

    // Try RGB format (e.g., "rgb(255,0,0)")
    if let Some(rgb_color) = parse_rgb_color(&color_str) {
        return Ok(rgb_color);
    }

    // Try 8-bit color index (e.g., "125")
    if let Ok(index) = color_str.parse::<u8>() {
        return Ok(ratatui::style::Color::Indexed(index));
    }

    // Named colors (case-insensitive)
    match color_lower.as_ref() {
        "black" => Ok(ratatui::style::Color::Black),
        "red" => Ok(ratatui::style::Color::Red),
        "green" => Ok(ratatui::style::Color::Green),
        "yellow" => Ok(ratatui::style::Color::Yellow),
        "blue" => Ok(ratatui::style::Color::Blue),
        "magenta" | "purple" => Ok(ratatui::style::Color::Magenta),
        "cyan" | "teal" => Ok(ratatui::style::Color::Cyan),
        "gray" | "grey" => Ok(ratatui::style::Color::Gray),
        "darkgray" | "darkgrey" => Ok(ratatui::style::Color::DarkGray),
        "lightred" => Ok(ratatui::style::Color::LightRed),
        "lightgreen" => Ok(ratatui::style::Color::LightGreen),
        "lightyellow" => Ok(ratatui::style::Color::LightYellow),
        "lightblue" => Ok(ratatui::style::Color::LightBlue),
        "lightmagenta" | "pink" => Ok(ratatui::style::Color::LightMagenta),
        "lightcyan" => Ok(ratatui::style::Color::LightCyan),
        "white" => Ok(ratatui::style::Color::White),
        "reset" => Ok(ratatui::style::Color::Reset),
        // Additional common colors using indexed colors
        "orange" => Ok(ratatui::style::Color::Indexed(208)),
        "brown" => Ok(ratatui::style::Color::Indexed(130)),
        "lime" => Ok(ratatui::style::Color::Indexed(46)),
        "navy" => Ok(ratatui::style::Color::Indexed(17)),
        "maroon" => Ok(ratatui::style::Color::Indexed(88)),
        "olive" => Ok(ratatui::style::Color::Indexed(58)),
        "silver" => Ok(ratatui::style::Color::Indexed(7)),
        _ => Err(
            "unknown color format. Use: named colors (red, blue, etc.), hex (#ff0000), RGB (rgb(255,0,0)), or 8-bit index (0-255)",
        ),
    }
}

/// Parse hex color in format #RRGGBB or RRGGBB
fn parse_hex_color(color_str: &str) -> Option<ratatui::style::Color> {
    let hex = color_str.strip_prefix('#').unwrap_or(color_str);

    if hex.len() == 6
        && hex.chars().all(|c| c.is_ascii_hexdigit())
        && let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        )
    {
        return Some(ratatui::style::Color::Rgb(r, g, b));
    }

    // Support 3-digit hex (#RGB -> #RRGGBB)
    if hex.len() == 3
        && hex.chars().all(|c| c.is_ascii_hexdigit())
        && let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&format!("{}{}", &hex[0..1], &hex[0..1]), 16),
            u8::from_str_radix(&format!("{}{}", &hex[1..2], &hex[1..2]), 16),
            u8::from_str_radix(&format!("{}{}", &hex[2..3], &hex[2..3]), 16),
        )
    {
        return Some(ratatui::style::Color::Rgb(r, g, b));
    }

    None
}

/// Parse RGB color in format rgb(r,g,b) or (r,g,b)
fn parse_rgb_color(color_str: &str) -> Option<ratatui::style::Color> {
    let rgb_str = color_str.trim();

    // Match rgb(r,g,b) format
    if rgb_str.starts_with("rgb(") && rgb_str.ends_with(')') {
        let values = &rgb_str[4..rgb_str.len() - 1];
        return parse_rgb_values(values);
    }

    // Match (r,g,b) format
    if rgb_str.starts_with('(') && rgb_str.ends_with(')') {
        let values = &rgb_str[1..rgb_str.len() - 1];
        return parse_rgb_values(values);
    }

    None
}

/// Parse RGB values from comma-separated string
fn parse_rgb_values(values: &str) -> Option<ratatui::style::Color> {
    let parts: Vec<&str> = values.split(',').map(|s| s.trim()).collect();
    if parts.len() == 3
        && let (Ok(r), Ok(g), Ok(b)) = (
            parts[0].parse::<u8>(),
            parts[1].parse::<u8>(),
            parts[2].parse::<u8>(),
        )
    {
        return Some(ratatui::style::Color::Rgb(r, g, b));
    }
    None
}

fn parse_ranking_mode(value: &str) -> Option<RankingMode> {
    match value.trim().to_lowercase().as_str() {
        "frecency" => Some(RankingMode::Frecency),
        "recency" => Some(RankingMode::Recency),
        "frequency" => Some(RankingMode::Frequency),
        _ => None,
    }
}

fn parse_pinned_order_mode(value: &str) -> Option<PinnedOrderMode> {
    match value.trim().to_lowercase().as_str() {
        "ranking" => Some(PinnedOrderMode::Ranking),
        "alphabetical" => Some(PinnedOrderMode::Alphabetical),
        "oldest_pinned" | "oldest" => Some(PinnedOrderMode::OldestPinned),
        "newest_pinned" | "newest" | "last_pinned" => Some(PinnedOrderMode::NewestPinned),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Opts, active_launch_method_count, parse_launch_prefix, set_launch_prefix, set_systemd_run,
    };

    #[test]
    fn parse_launch_prefix_supports_shell_words() {
        assert_eq!(
            parse_launch_prefix("runapp --tag \"gui apps\" --").unwrap(),
            ["runapp", "--tag", "gui apps", "--"]
        );
    }

    #[test]
    fn parse_launch_prefix_rejects_empty_values() {
        assert!(parse_launch_prefix("").is_err());
    }

    #[test]
    fn later_launch_method_overrides_previous_state() {
        let mut opts = Opts::default();
        set_systemd_run(&mut opts);
        set_launch_prefix(&mut opts, vec!["runapp".into(), "--".into()]);
        assert_eq!(active_launch_method_count(&opts), 1);
        assert!(!opts.systemd_run);
        assert!(!opts.uwsm);
        assert!(opts.launch_prefix_set);
    }
}
