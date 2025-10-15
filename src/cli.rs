use directories::ProjectDirs;
use serde::Deserialize;
use std::str::FromStr;
use std::{env, fs, io, path, process};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MatchMode {
    Exact,
    Fuzzy,
}

impl Default for MatchMode {
    fn default() -> Self {
        MatchMode::Fuzzy
    }
}

fn usage() -> ! {
    let cmd = env::args().next().unwrap_or_else(|| "fsel".to_string());

    println!(
        "fsel — Fast terminal application launcher
Usage:
  {cmd} [OPTIONS]

Core Modes:
  -p, --program <NAME>   Launch program directly (bypass TUI)
      --cclip            Clipboard history mode
      --dmenu            Dmenu-compatible mode

Control Flags:
  -r, --replace          Replace running fsel/cclip instance
  -d, --detach           Detach launched applications (GUI-safe)
  -v, --verbose          Increase verbosity (repeatable)
      --systemd-run      Launch via systemd-run --user --scope
      --uwsm             Launch via uwsm app

Quick Extras:
      --clear-cache      Clear app cache
      --refresh-cache    Rescan desktop entries
      --filter-desktop[=no] Respect OnlyShowIn/NotShowIn (default: yes)

Help:
  -H, --help             Show detailed option tree
  -h                     Show this overview
  -V, --version          Show version info
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
│  ├─ -p, --program <NAME>         Launch program directly (bypass TUI)
│  ├─ --cclip                      Clipboard history mode
│  └─ --dmenu                      Dmenu-compatible mode
│
├─ Control Flags
│  ├─ -r, --replace                Replace running fsel/cclip instance
│  ├─ -d, --detach                 Detach launched applications (GUI-safe)
│  ├─ -v, --verbose                Increase verbosity (repeatable)
│  ├─ --systemd-run                Launch via systemd-run --user --scope
│  ├─ --uwsm                       Launch via uwsm app
│  ├─ --no-exec                    Print selection to stdout instead of launching
│  └─ -ss <SEARCH>                 Pre-fill TUI search (must be last option)
│
├─ Quick Extras
│  ├─ --clear-history              Clear launch history
│  ├─ --clear-cache                Clear app cache
│  ├─ --refresh-cache              Rescan desktop entries
│  ├─ --filter-desktop[=no]        Respect OnlyShowIn/NotShowIn (default: yes)
│  ├─ --hide-before-typing         Hide list until first character typed
│  ├─ --list-executables-in-path   Include executables from $PATH
│  └─ --match-mode <MODE>          fuzzy | exact (default: fuzzy)
│
├─ Dmenu Mode Options
│  ├─ --dmenu0                     Like --dmenu but null-separated input
│  ├─ --password[=CHAR]            Password mode (mask input)
│  ├─ --index                      Output index instead of text
│  ├─ --with-nth <COLS>            Display only specific columns (e.g. 1,3)
│  ├─ --accept-nth <COLS>          Output only specified columns
│  ├─ --match-nth <COLS>           Match only specified columns
│  ├─ --delimiter <CHAR>           Column delimiter (default: space)
│  ├─ --only-match                 Disallow custom input
│  ├─ --exit-if-empty              Exit if stdin is empty
│  ├─ --select <STRING>            Preselect matching entry
│  ├─ --select-index <N>           Preselect entry by index
│  ├─ --auto-select                Auto-select when one match remains
│  └─ --prompt-only                Input-only mode (no list)
│
├─ Clipboard
│  └─ --cclip                      Clipboard history viewer with previews
│
└─ General
   ├─ -h                           Show short help
   ├─ -H, --help                   Show detailed help
   └─ -V, --version                Show version info
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
    /// Enable Sway integration (default when `$SWAYSOCK` is not empty)
    pub sway: bool,
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
    pub keybinds: crate::keybinds::Keybinds,
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
    /// App launcher settings
    pub filter_desktop: bool,
    pub list_executables_in_path: bool,
    pub hide_before_typing: bool,
    pub match_mode: MatchMode,
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
    /// Dmenu-specific disable mouse option
    pub dmenu_disable_mouse: Option<bool>,
    /// Cclip-specific disable mouse option
    pub cclip_disable_mouse: Option<bool>,
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
            sway: false,
            cursor: "█".to_string(),
            verbose: None,
            hard_stop: false,
            disable_mouse: false,
            no_exec: false,
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
            keybinds: crate::keybinds::Keybinds::default(),
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
            // App launcher defaults
            filter_desktop: true,
            list_executables_in_path: false,
            hide_before_typing: false,
            match_mode: MatchMode::Fuzzy,
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
            dmenu_disable_mouse: None,
            cclip_disable_mouse: None,
        }
    }
}

/// Parses the cli arguments
pub fn parse() -> Result<Opts, lexopt::Error> {
    use lexopt::prelude::*;
    let mut parser = lexopt::Parser::from_env();
    let mut default = Opts::default();
    let mut config_file: Option<path::PathBuf> = None;

    if let Ok(_socket) = env::var("SWAYSOCK") {
        default.sway = true;
    }
    
    // Check if invoked as dmenu
    if let Some(arg0) = env::args().next() {
        if arg0.ends_with("dmenu") {
            default.dmenu_mode = true;
        }
    }
    
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

    while let Some(arg) = parser.next()? {
        match arg {
            Short('s') | Long("nosway") => {
                default.sway = false;
            }
            Short('r') | Long("replace") => {
                default.replace = true;
            }
            Short('c') | Long("config") => {
                config_file = Some(path::PathBuf::from(parser.value()?));
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
            Long("systemd-run") => {
                default.systemd_run = true;
            }
            Long("uwsm") => {
                default.uwsm = true;
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
                let tag_arg = parser.value()?.into_string().map_err(|_| "Tag argument must be valid UTF-8")?;
                if tag_arg == "list" {
                    default.cclip_tag_list = true;
                    // Check if there's another argument for specific tag
                    if let Ok(val) = parser.value() {
                        default.cclip_tag = Some(val.into_string().map_err(|_| "Tag name must be valid UTF-8")?);
                    }
                } else {
                    default.cclip_tag = Some(tag_arg);
                }
            }
            Long("dmenu0") => {
                default.dmenu_mode = true;
                default.dmenu_null_separated = true;
            }
            Long("password") => {
                let val = parser.optional_value();
                default.dmenu_password_mode = true;
                if let Some(v) = val {
                    default.dmenu_password_character = v.into_string().map_err(|_| "Password character must be valid UTF-8")?;
                }
            }
            Long("index") => {
                default.dmenu_index_mode = true;
            }
            Long("accept-nth") => {
                let cols_str = parser.value()?.into_string().map_err(|_| "Column specification must be valid UTF-8")?;
                let cols: Result<Vec<usize>, _> = cols_str.split(',')
                    .map(|s| s.trim().parse::<usize>())
                    .collect();
                default.dmenu_accept_nth = Some(cols.map_err(|_| "Invalid column specification")?);
            }
            Long("match-nth") => {
                let cols_str = parser.value()?.into_string().map_err(|_| "Column specification must be valid UTF-8")?;
                let cols: Result<Vec<usize>, _> = cols_str.split(',')
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
                default.dmenu_select = Some(parser.value()?.into_string().map_err(|_| "Select string must be valid UTF-8")?);
            }
            Long("select-index") => {
                let idx_str = parser.value()?.into_string().map_err(|_| "Index must be valid UTF-8")?;
                default.dmenu_select_index = Some(idx_str.parse::<usize>().map_err(|_| "Invalid index")?);
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
                    let v_str = v.into_string().map_err(|_| "filter-desktop value must be valid UTF-8")?;
                    default.filter_desktop = v_str != "no";
                } else {
                    default.filter_desktop = true;
                }
            }
            Long("list-executables-in-path") => {
                default.list_executables_in_path = true;
            }
            Long("match-mode") => {
                let mode_str = parser.value()?.into_string().map_err(|_| "Match mode must be valid UTF-8")?;
                default.match_mode = match mode_str.as_str() {
                    "exact" => MatchMode::Exact,
                    "fuzzy" => MatchMode::Fuzzy,
                    _ => return Err("Invalid match mode. Use 'exact' or 'fuzzy'".into()),
                };
            }
            Long("with-nth") => {
                let cols_str = parser.value()?.into_string().map_err(|_| "Column specification must be valid UTF-8")?;
                let cols: Result<Vec<usize>, _> = cols_str.split(',')
                    .map(|s| s.trim().parse::<usize>())
                    .collect();
                default.dmenu_with_nth = Some(cols.map_err(|_| "Invalid column specification. Use comma-separated numbers like: 1,2,4")?);
            }
            Long("delimiter") => {
                default.dmenu_delimiter = parser.value()?.into_string().map_err(|_| "Delimiter must be valid UTF-8")?;
            }
            Short('p') | Long("program") => {
                default.program = Some(parser.value()?.into_string().map_err(|_| "Program name must be valid UTF-8")?);
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
                    Long(name) => {
                        match *name {
                            "clip" => "Unknown option '--clip'. Did you mean '--cclip'?",
                            "menu" => "Unknown option '--menu'. Did you mean '--dmenu'?",
                            "dme" | "dmen" => "Unknown option. Did you mean '--dmenu'?",
                            "cc" | "ccli" => "Unknown option. Did you mean '--cclip'?",
                            "sway" => "Unknown option '--sway'. Sway integration is enabled by default when SWAYSOCK is set. Use '-s' or '--nosway' to disable it.",
                            "help" => "Unknown option '--help'. Use '-h' or '--help'.",
                            "version" => "Unknown option '--version'. Use '-V' or '--version'.",
                            _ => "Unknown option. Use '-h' or '--help' to see available options."
                        }
                    }
                    Short(c) => {
                        match c {
                            'C' => "Unknown option '-C'. Did you mean '-c' for --config?",
                            'P' => "Unknown option '-P'. Did you mean '-p' for --program?",
                            'S' => "Unknown option '-S'. Did you mean '-s' for --nosway?",
                            'R' => "Unknown option '-R'. Did you mean '-r' for --replace?",
                            'H' => "Unknown option '-H'. Did you mean '-h' for --help?",
                            _ => "Unknown option. Use '-h' or '--help' to see available options."
                        }
                    }
                    Value(_val) => {
                        // Unexpected value
                        return Err(arg.unexpected());
                    }
                };
                
                eprintln!("Error: {}", error_msg);
                eprintln!();
                eprintln!("Available options:");
                eprintln!("  -s, --nosway           Disable Sway integration");
                eprintln!("  -c, --config <file>    Specify config file");
                eprintln!("  -r, --replace          Replace existing instance (fsel/cclip only)");
                eprintln!("  -p, --program [name]   Launch program directly (optional)");
                eprintln!("  -ss <search>           Pre-fill search (must be last)");
                eprintln!("  -v, --verbose          Increase verbosity");
                eprintln!("  -h, --help             Show help");
                eprintln!("  -V, --version          Show version");
                eprintln!("      --dmenu            Dmenu mode");
                eprintln!("      --cclip            Clipboard history mode");
                eprintln!("      --no-exec          Print command instead of running");
                eprintln!("      --systemd-run      Use systemd-run");
                eprintln!("      --uwsm             Use uwsm");
                eprintln!("  -d, --detach           Detach from terminal");
                eprintln!();
                eprintln!("For more details, use: fsel --help");
                std::process::exit(1);
            }
        }
    }

    let mut file_conf: Option<FileConf> = None;

    // Read config file: First command line, then config dir
    {
        if config_file.is_none() {
            if let Some(proj_dirs) = ProjectDirs::from("ch", "forkbomb9", env!("CARGO_PKG_NAME")) {
                let mut tmp = proj_dirs.config_dir().to_path_buf();
                tmp.push("config.toml");
                config_file = Some(tmp);
            }
        }

        if let Some(f) = config_file {
            match fs::read_to_string(&f) {
                Ok(content) => match FileConf::read_with_enhanced_errors(&content) {
                    Ok(conf) => {
                        file_conf = Some(conf);
                    }
                    Err(e) => {
                        println!(
                            "Error reading config file {}:\n{}",
                            f.display(),
                            e
                        );
                        process::exit(1);
                    }
                },
                Err(e) => {
                    if io::ErrorKind::NotFound != e.kind() {
                        println!("Error reading config file {}:\n\t{}", f.display(), e);
                        process::exit(1);
                    }
                }
            }
        }
    }

    let file_conf = file_conf.unwrap_or_default();

    if let Some(color) = file_conf.highlight_color {
        match string_to_color(color) {
            Ok(color) => default.highlight_color = color,
            Err(e) => {
                // Improve error messages in future version
                eprintln!("Error parsing config file: {e}");
                std::process::exit(1);
            }
        }
    }

    if let Some(command) = file_conf.terminal_launcher {
        default.terminal_launcher = command;
    }

    if let Some(c) = file_conf.cursor {
        default.cursor = c;
    }

    if let Some(h) = file_conf.hard_stop {
        default.hard_stop = h;
    }

    if let Some(rb) = file_conf.rounded_borders {
        default.rounded_borders = rb;
    }
    
    if let Some(dm) = file_conf.disable_mouse {
        default.disable_mouse = dm;
    }

    // Parse border colors
    if let Some(color) = file_conf.main_border_color {
        match string_to_color(color) {
            Ok(c) => default.main_border_color = c,
            Err(_) => eprintln!("Warning: Invalid main_border_color in config"),
        }
    }
    if let Some(color) = file_conf.apps_border_color {
        match string_to_color(color) {
            Ok(c) => default.apps_border_color = c,
            Err(_) => eprintln!("Warning: Invalid apps_border_color in config"),
        }
    }
    if let Some(color) = file_conf.input_border_color {
        match string_to_color(color) {
            Ok(c) => default.input_border_color = c,
            Err(_) => eprintln!("Warning: Invalid input_border_color in config"),
        }
    }

    // Parse text colors
    if let Some(color) = file_conf.main_text_color {
        match string_to_color(color) {
            Ok(c) => default.main_text_color = c,
            Err(_) => eprintln!("Warning: Invalid main_text_color in config"),
        }
    }
    if let Some(color) = file_conf.apps_text_color {
        match string_to_color(color) {
            Ok(c) => default.apps_text_color = c,
            Err(_) => eprintln!("Warning: Invalid apps_text_color in config"),
        }
    }
    if let Some(color) = file_conf.input_text_color {
        match string_to_color(color) {
            Ok(c) => default.input_text_color = c,
            Err(_) => eprintln!("Warning: Invalid input_text_color in config"),
        }
    }

    if let Some(fm) = file_conf.fancy_mode {
        default.fancy_mode = fm;
    }

    if let Some(color) = file_conf.header_title_color {
        match string_to_color(color) {
            Ok(c) => default.header_title_color = c,
            Err(_) => eprintln!("Warning: Invalid header_title_color in config"),
        }
    }
    
    if let Some(color) = file_conf.pin_color {
        match string_to_color(color) {
            Ok(c) => default.pin_color = c,
            Err(_) => eprintln!("Warning: Invalid pin_color in config"),
        }
    }
    
    if let Some(icon) = file_conf.pin_icon {
        default.pin_icon = icon;
    }
    
    if let Some(keybinds) = file_conf.keybinds {
        default.keybinds = keybinds;
    }

    // Parse layout configuration with validation
    if let Some(height) = file_conf.title_panel_height_percent {
        if height >= 10 && height <= 70 {
            default.title_panel_height_percent = height;
        } else {
            eprintln!("Warning: title_panel_height_percent must be between 10-70%, using default");
        }
    }
    if let Some(height) = file_conf.input_panel_height {
        if height >= 1 && height <= 10 {
            default.input_panel_height = height;
        } else {
            eprintln!("Warning: input_panel_height must be between 1-10 lines, using default");
        }
    }
    if let Some(position) = file_conf.title_panel_position {
        default.title_panel_position = Some(position);
    }


    // Load dmenu configuration if present
    if let Some(dmenu_conf) = &file_conf.dmenu {
        if let Some(color) = &dmenu_conf.highlight_color {
            match string_to_color(color) {
                Ok(c) => default.dmenu_highlight_color = Some(c),
                Err(_) => eprintln!("Warning: Invalid dmenu highlight_color in config"),
            }
        }
        if let Some(cursor) = &dmenu_conf.cursor {
            default.dmenu_cursor = Some(cursor.clone());
        }
        if let Some(hard_stop) = dmenu_conf.hard_stop {
            default.dmenu_hard_stop = Some(hard_stop);
        }
        if let Some(rounded_borders) = dmenu_conf.rounded_borders {
            default.dmenu_rounded_borders = Some(rounded_borders);
        }
        
        // Load dmenu border colors
        if let Some(color) = &dmenu_conf.main_border_color {
            match string_to_color(color) {
                Ok(c) => default.dmenu_main_border_color = Some(c),
                Err(_) => eprintln!("Warning: Invalid dmenu main_border_color in config"),
            }
        }
        if let Some(color) = &dmenu_conf.items_border_color {
            match string_to_color(color) {
                Ok(c) => default.dmenu_items_border_color = Some(c),
                Err(_) => eprintln!("Warning: Invalid dmenu items_border_color in config"),
            }
        }
        if let Some(color) = &dmenu_conf.input_border_color {
            match string_to_color(color) {
                Ok(c) => default.dmenu_input_border_color = Some(c),
                Err(_) => eprintln!("Warning: Invalid dmenu input_border_color in config"),
            }
        }
        
        // Load dmenu text colors
        if let Some(color) = &dmenu_conf.main_text_color {
            match string_to_color(color) {
                Ok(c) => default.dmenu_main_text_color = Some(c),
                Err(_) => eprintln!("Warning: Invalid dmenu main_text_color in config"),
            }
        }
        if let Some(color) = &dmenu_conf.items_text_color {
            match string_to_color(color) {
                Ok(c) => default.dmenu_items_text_color = Some(c),
                Err(_) => eprintln!("Warning: Invalid dmenu items_text_color in config"),
            }
        }
        if let Some(color) = &dmenu_conf.input_text_color {
            match string_to_color(color) {
                Ok(c) => default.dmenu_input_text_color = Some(c),
                Err(_) => eprintln!("Warning: Invalid dmenu input_text_color in config"),
            }
        }
        if let Some(color) = &dmenu_conf.header_title_color {
            match string_to_color(color) {
                Ok(c) => default.dmenu_header_title_color = Some(c),
                Err(_) => eprintln!("Warning: Invalid dmenu header_title_color in config"),
            }
        }
        
        // Load dmenu layout
        if let Some(height) = dmenu_conf.title_panel_height_percent {
            if height >= 10 && height <= 70 {
                default.dmenu_title_panel_height_percent = Some(height);
            } else {
                eprintln!("Warning: dmenu title_panel_height_percent must be between 10-70%, using default");
            }
        }
        if let Some(height) = dmenu_conf.input_panel_height {
            if height >= 1 && height <= 10 {
                default.dmenu_input_panel_height = Some(height);
            } else {
                eprintln!("Warning: dmenu input_panel_height must be between 1-10 lines, using default");
            }
        }
        if let Some(position) = &dmenu_conf.title_panel_position {
            default.dmenu_title_panel_position = Some(position.clone());
        }
        
        // Load other dmenu options
        if let Some(delimiter) = &dmenu_conf.delimiter {
            default.dmenu_delimiter = delimiter.clone();
        }
        if let Some(show_line_numbers) = dmenu_conf.show_line_numbers {
            default.dmenu_show_line_numbers = show_line_numbers;
        }
        if let Some(wrap_long_lines) = dmenu_conf.wrap_long_lines {
            default.dmenu_wrap_long_lines = wrap_long_lines;
        }
        if let Some(disable_mouse) = dmenu_conf.disable_mouse {
            default.dmenu_disable_mouse = Some(disable_mouse);
        }
        if let Some(password_char) = &dmenu_conf.password_character {
            default.dmenu_password_character = password_char.clone();
        }
        if let Some(exit_if_empty) = dmenu_conf.exit_if_empty {
            default.dmenu_exit_if_empty = exit_if_empty;
        }
    }
    
    // Load app launcher configuration if present
    if let Some(app_conf) = &file_conf.app_launcher {
        if let Some(filter_desktop) = app_conf.filter_desktop {
            default.filter_desktop = filter_desktop;
        }
        if let Some(list_execs) = app_conf.list_executables_in_path {
            default.list_executables_in_path = list_execs;
        }
        if let Some(hide_before) = app_conf.hide_before_typing {
            default.hide_before_typing = hide_before;
        }
        if let Some(mode_str) = &app_conf.match_mode {
            default.match_mode = match mode_str.as_str() {
                "exact" => MatchMode::Exact,
                "fuzzy" => MatchMode::Fuzzy,
                _ => {
                    eprintln!("Warning: Invalid match_mode in config, using default");
                    MatchMode::Fuzzy
                }
            };
        }
        if let Some(confirm) = app_conf.confirm_first_launch {
            default.confirm_first_launch = confirm;
        }
    }

    // Load cclip configuration if present
    if let Some(cclip_conf) = &file_conf.cclip {
        if let Some(color) = &cclip_conf.highlight_color {
            match string_to_color(color) {
                Ok(c) => default.cclip_highlight_color = Some(c),
                Err(_) => eprintln!("Warning: Invalid cclip highlight_color in config"),
            }
        }
        if let Some(cursor) = &cclip_conf.cursor {
            default.cclip_cursor = Some(cursor.clone());
        }
        if let Some(hard_stop) = cclip_conf.hard_stop {
            default.cclip_hard_stop = Some(hard_stop);
        }
        if let Some(rounded_borders) = cclip_conf.rounded_borders {
            default.cclip_rounded_borders = Some(rounded_borders);
        }
        
        // Load cclip border colors
        if let Some(color) = &cclip_conf.main_border_color {
            match string_to_color(color) {
                Ok(c) => default.cclip_main_border_color = Some(c),
                Err(_) => eprintln!("Warning: Invalid cclip main_border_color in config"),
            }
        }
        if let Some(color) = &cclip_conf.items_border_color {
            match string_to_color(color) {
                Ok(c) => default.cclip_items_border_color = Some(c),
                Err(_) => eprintln!("Warning: Invalid cclip items_border_color in config"),
            }
        }
        if let Some(color) = &cclip_conf.input_border_color {
            match string_to_color(color) {
                Ok(c) => default.cclip_input_border_color = Some(c),
                Err(_) => eprintln!("Warning: Invalid cclip input_border_color in config"),
            }
        }
        
        // Load cclip text colors
        if let Some(color) = &cclip_conf.main_text_color {
            match string_to_color(color) {
                Ok(c) => default.cclip_main_text_color = Some(c),
                Err(_) => eprintln!("Warning: Invalid cclip main_text_color in config"),
            }
        }
        if let Some(color) = &cclip_conf.items_text_color {
            match string_to_color(color) {
                Ok(c) => default.cclip_items_text_color = Some(c),
                Err(_) => eprintln!("Warning: Invalid cclip items_text_color in config"),
            }
        }
        if let Some(color) = &cclip_conf.input_text_color {
            match string_to_color(color) {
                Ok(c) => default.cclip_input_text_color = Some(c),
                Err(_) => eprintln!("Warning: Invalid cclip input_text_color in config"),
            }
        }
        if let Some(color) = &cclip_conf.header_title_color {
            match string_to_color(color) {
                Ok(c) => default.cclip_header_title_color = Some(c),
                Err(_) => eprintln!("Warning: Invalid cclip header_title_color in config"),
            }
        }
        
        // Load cclip layout
        if let Some(height) = cclip_conf.title_panel_height_percent {
            if height >= 10 && height <= 70 {
                default.cclip_title_panel_height_percent = Some(height);
            } else {
                eprintln!("Warning: cclip title_panel_height_percent must be between 10-70%, using default");
            }
        }
        if let Some(height) = cclip_conf.input_panel_height {
            if height >= 1 && height <= 10 {
                default.cclip_input_panel_height = Some(height);
            } else {
                eprintln!("Warning: cclip input_panel_height must be between 1-10 lines, using default");
            }
        }
        if let Some(position) = &cclip_conf.title_panel_position {
            default.cclip_title_panel_position = Some(position.clone());
        }
        
        // Load other cclip options
        if let Some(show_line_numbers) = cclip_conf.show_line_numbers {
            default.cclip_show_line_numbers = Some(show_line_numbers);
        }
        if let Some(wrap_long_lines) = cclip_conf.wrap_long_lines {
            default.cclip_wrap_long_lines = Some(wrap_long_lines);
        }
        if let Some(image_preview) = cclip_conf.image_preview {
            default.cclip_image_preview = Some(image_preview);
        }
        if let Some(hide_message) = cclip_conf.hide_inline_image_message {
            default.cclip_hide_inline_image_message = Some(hide_message);
        }
        if let Some(disable_mouse) = cclip_conf.disable_mouse {
            default.cclip_disable_mouse = Some(disable_mouse);
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
    if (default.cclip_tag.is_some() || default.cclip_tag_list) && !default.cclip_mode {
        eprintln!("Error: --tag requires --cclip mode");
        eprintln!("Usage: fsel --cclip --tag <name>");
        eprintln!("       fsel --cclip --tag list");
        std::process::exit(1);
    }
    
    // Validate mutually exclusive special modes
    if default.dmenu_mode && default.cclip_mode {
        eprintln!("Error: --dmenu and --cclip cannot be used together");
        std::process::exit(1);
    }
    
    // Validate flag conflicts - no-exec overrides all launch methods
    if default.no_exec {
        if default.sway || default.systemd_run || default.uwsm {
            eprintln!("Warning: --no-exec overrides other launch method flags");
        }
    } else {
        // Check for mutually exclusive launch methods
        let launch_methods = [default.systemd_run, default.uwsm, default.sway].iter().filter(|&&x| x).count();
        if launch_methods > 1 {
            eprintln!("Error: Only one launch method can be specified at a time");
            eprintln!("Available methods: --systemd-run, --uwsm, Sway integration (auto-detected)");
            std::process::exit(1);
        }
    }

    Ok(default)
}

/// Title panel position
#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PanelPosition {
    /// Panel at the top (default behavior)
    Top,
    /// Panel in the middle (where results/apps usually are)
    Middle,
    /// Panel at the bottom (above input field)
    Bottom,
}

impl Default for PanelPosition {
    fn default() -> Self {
        PanelPosition::Top
    }
}

impl FromStr for PanelPosition {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "top" => Ok(PanelPosition::Top),
            "middle" => Ok(PanelPosition::Middle),
            "bottom" => Ok(PanelPosition::Bottom),
            _ => Err(format!("Invalid panel position: '{}'. Valid options: top, middle, bottom", s)),
        }
    }
}

/// File configuration, parsed with [serde]
///
/// [serde]: serde
#[derive(Debug, Deserialize, Default)]
pub struct FileConf {
    /// Highlight color used in the UI
    pub highlight_color: Option<String>,
    /// Command to run Terminal=true apps
    pub terminal_launcher: Option<String>,
    /// Cursor character for the search
    pub cursor: Option<String>,
    /// Don't scroll past the last/first item
    pub hard_stop: Option<bool>,
    /// Disable mouse input in all modes
    pub disable_mouse: Option<bool>,
    /// Use rounded borders (default: true)
    pub rounded_borders: Option<bool>,
    /// Border color for the main panel (Fsel)
    pub main_border_color: Option<String>,
    /// Border color for the apps panel
    pub apps_border_color: Option<String>,
    /// Border color for the input panel
    pub input_border_color: Option<String>,
    /// Text color for the main panel
    pub main_text_color: Option<String>,
    /// Text color for the apps panel
    pub apps_text_color: Option<String>,
    /// Text color for the input panel
    pub input_text_color: Option<String>,
    /// Enable fancy mode (show selected app name in borders)
    pub fancy_mode: Option<bool>,
    /// Color for panel header titles
    pub header_title_color: Option<String>,
    /// Color for pin icon
    pub pin_color: Option<String>,
    /// Pin icon character
    pub pin_icon: Option<String>,
    /// Keybinds configuration
    pub keybinds: Option<crate::keybinds::Keybinds>,
    /// Title panel height percentage (10-70%)
    pub title_panel_height_percent: Option<u16>,
    /// Input panel height in lines
    pub input_panel_height: Option<u16>,
    /// Position of the title/content/description panel (top, middle, bottom)
    pub title_panel_position: Option<PanelPosition>,
    /// Dmenu-specific configuration
    pub dmenu: Option<DmenuConf>,
    /// Cclip-specific configuration
    pub cclip: Option<CclipConf>,
    /// App launcher-specific configuration
    pub app_launcher: Option<AppLauncherConf>,
}

/// Dmenu-specific configuration section
#[derive(Debug, Deserialize, Default)]
pub struct DmenuConf {
    /// Highlight color used in dmenu mode
    pub highlight_color: Option<String>,
    /// Cursor character for dmenu search
    pub cursor: Option<String>,
    /// Don't scroll past the last/first item in dmenu
    pub hard_stop: Option<bool>,
    /// Use rounded borders in dmenu (default: true)
    pub rounded_borders: Option<bool>,
    /// Border colors for dmenu mode
    pub main_border_color: Option<String>,
    pub items_border_color: Option<String>,
    pub input_border_color: Option<String>,
    /// Text colors for dmenu mode
    pub main_text_color: Option<String>,
    pub items_text_color: Option<String>,
    pub input_text_color: Option<String>,
    /// Color for panel header titles in dmenu
    pub header_title_color: Option<String>,
    /// Layout configuration for dmenu
    pub title_panel_height_percent: Option<u16>,
    pub input_panel_height: Option<u16>,
    /// Position of the content panel (top, middle, bottom)
    pub title_panel_position: Option<PanelPosition>,
    /// Default delimiter for column parsing
    pub delimiter: Option<String>,
    /// Show line numbers in selection
    pub show_line_numbers: Option<bool>,
    /// Wrap long lines in content display
    pub wrap_long_lines: Option<bool>,
    /// Disable mouse input in dmenu mode
    pub disable_mouse: Option<bool>,
    pub password_character: Option<String>,
    pub exit_if_empty: Option<bool>,
}

/// Cclip-specific configuration section (inherits from dmenu, then regular mode)
#[derive(Debug, Deserialize, Default)]
pub struct CclipConf {
    /// Highlight color used in cclip mode
    pub highlight_color: Option<String>,
    /// Cursor character for cclip search
    pub cursor: Option<String>,
    /// Don't scroll past the last/first item in cclip
    pub hard_stop: Option<bool>,
    /// Use rounded borders in cclip (default: true)
    pub rounded_borders: Option<bool>,
    /// Border colors for cclip mode
    pub main_border_color: Option<String>,
    pub items_border_color: Option<String>,
    pub input_border_color: Option<String>,
    /// Text colors for cclip mode
    pub main_text_color: Option<String>,
    pub items_text_color: Option<String>,
    pub input_text_color: Option<String>,
    /// Color for panel header titles in cclip
    pub header_title_color: Option<String>,
    /// Layout configuration for cclip
    pub title_panel_height_percent: Option<u16>,
    pub input_panel_height: Option<u16>,
    /// Position of the content preview panel (top, middle, bottom)
    pub title_panel_position: Option<PanelPosition>,
    /// Show line numbers in selection
    pub show_line_numbers: Option<bool>,
    /// Wrap long lines in content display
    pub wrap_long_lines: Option<bool>,
    /// Enable image previews using chafa
    pub image_preview: Option<bool>,
    /// Hide the inline image preview message (show blank instead)
    pub hide_inline_image_message: Option<bool>,
    /// Disable mouse input in cclip mode
    pub disable_mouse: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AppLauncherConf {
    pub filter_desktop: Option<bool>,
    pub list_executables_in_path: Option<bool>,
    pub hide_before_typing: Option<bool>,
    pub match_mode: Option<String>,
    pub confirm_first_launch: Option<bool>,
}

impl FileConf {
    /// Parse a file with enhanced error reporting
    pub fn read_with_enhanced_errors(raw: &str) -> Result<Self, String> {
        match toml::from_str::<Self>(raw) {
            Ok(config) => Ok(config),
            Err(e) => {
                let error_msg = e.message();
                
                // Check if it's an unknown field error and provide helpful guidance
                if error_msg.contains("unknown field") {
                    let enhanced_msg = Self::enhance_unknown_field_error(error_msg);
                    Err(enhanced_msg)
                } else {
                    Err(format!("{}", error_msg))
                }
            }
        }
    }

    fn enhance_unknown_field_error(original_error: &str) -> String {
        // Extract the unknown field name from the error message
        let unknown_field = if let Some(start) = original_error.find("unknown field `") {
            let start = start + "unknown field `".len();
            if let Some(end) = original_error[start..].find('`') {
                Some(&original_error[start..start + end])
            } else {
                None
            }
        } else {
            None
        };
        
        // Determine section and provide targeted help
        if original_error.contains("expected one of") && original_error.contains("filter_desktop") {
            let mut result = String::from("Config Error: Invalid field in [app_launcher] section");
            
            if let Some(field) = unknown_field {
                // Check if it's a color/UI field that belongs at root level
                let root_level_fields = [
                    "main_border_color", "apps_border_color", "input_border_color",
                    "main_text_color", "apps_text_color", "input_text_color", 
                    "highlight_color", "header_title_color", "pin_color", "pin_icon",
                    "cursor", "rounded_borders", "hard_stop", "fancy_mode", "terminal_launcher"
                ];
                
                if root_level_fields.contains(&field) {
                    result.push_str(&format!("\n\n'{}' belongs at ROOT LEVEL", field));
                    result.push_str("\nMove it outside [app_launcher] section");
                } else {
                    result.push_str(&format!("\n\n'{}' is not a valid field", field));
                }
            }
            
            result.push_str("\n\n[app_launcher] accepts:");
            result.push_str("\n  filter_desktop, list_executables_in_path,");
            result.push_str("\n  hide_before_typing, match_mode, confirm_first_launch");
            
            result
        } else if original_error.contains("expected one of") && original_error.contains("delimiter") {
            format!("Config Error: Invalid field in [dmenu] section{}", 
                if let Some(field) = unknown_field { 
                    format!("\n'{}' is not valid here", field) 
                } else { 
                    String::new() 
                })
        } else if original_error.contains("expected one of") && original_error.contains("image_preview") {
            format!("Config Error: Invalid field in [cclip] section{}", 
                if let Some(field) = unknown_field { 
                    format!("\n'{}' is not valid here", field) 
                } else { 
                    String::new() 
                })
        } else {
            format!("{}\n\nTip: Color/UI options go at root level", original_error)
        }
    }
}

/// Parses a [String] into a ratatui [color]
///
/// Case-insensitive
///
/// [String]: std::string::String
/// [color]: tui::style::Color
fn string_to_color<T: Into<String>>(val: T) -> Result<ratatui::style::Color, &'static str> {
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
        "magenta" => Ok(ratatui::style::Color::Magenta),
        "cyan" => Ok(ratatui::style::Color::Cyan),
        "gray" | "grey" => Ok(ratatui::style::Color::Gray),
        "darkgray" | "darkgrey" => Ok(ratatui::style::Color::DarkGray),
        "lightred" => Ok(ratatui::style::Color::LightRed),
        "lightgreen" => Ok(ratatui::style::Color::LightGreen),
        "lightyellow" => Ok(ratatui::style::Color::LightYellow),
        "lightblue" => Ok(ratatui::style::Color::LightBlue),
        "lightmagenta" => Ok(ratatui::style::Color::LightMagenta),
        "lightcyan" => Ok(ratatui::style::Color::LightCyan),
        "white" => Ok(ratatui::style::Color::White),
        "reset" => Ok(ratatui::style::Color::Reset),
        _ => Err("unknown color format. Use: named colors (red, blue, etc.), hex (#ff0000), RGB (rgb(255,0,0)), or 8-bit index (0-255)"),
    }
}

/// Parse hex color in format #RRGGBB or RRGGBB
fn parse_hex_color(color_str: &str) -> Option<ratatui::style::Color> {
    let hex = color_str.strip_prefix('#').unwrap_or(color_str);
    
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return Some(ratatui::style::Color::Rgb(r, g, b));
        }
    }
    
    // Support 3-digit hex (#RGB -> #RRGGBB)
    if hex.len() == 3 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&format!("{}{}", &hex[0..1], &hex[0..1]), 16),
            u8::from_str_radix(&format!("{}{}", &hex[1..2], &hex[1..2]), 16),
            u8::from_str_radix(&format!("{}{}", &hex[2..3], &hex[2..3]), 16),
        ) {
            return Some(ratatui::style::Color::Rgb(r, g, b));
        }
    }
    
    None
}

/// Parse RGB color in format rgb(r,g,b) or (r,g,b)
fn parse_rgb_color(color_str: &str) -> Option<ratatui::style::Color> {
    let rgb_str = color_str.trim();
    
    // Match rgb(r,g,b) format
    if rgb_str.starts_with("rgb(") && rgb_str.ends_with(')') {
        let values = &rgb_str[4..rgb_str.len()-1];
        return parse_rgb_values(values);
    }
    
    // Match (r,g,b) format
    if rgb_str.starts_with('(') && rgb_str.ends_with(')') {
        let values = &rgb_str[1..rgb_str.len()-1];
        return parse_rgb_values(values);
    }
    
    None
}

/// Parse RGB values from comma-separated string
fn parse_rgb_values(values: &str) -> Option<ratatui::style::Color> {
    let parts: Vec<&str> = values.split(',').map(|s| s.trim()).collect();
    if parts.len() == 3 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            parts[0].parse::<u8>(),
            parts[1].parse::<u8>(),
            parts[2].parse::<u8>(),
        ) {
            return Some(ratatui::style::Color::Rgb(r, g, b));
        }
    }
    None
}
