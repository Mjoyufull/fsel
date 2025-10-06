use directories::ProjectDirs;
use serde::Deserialize;
use std::{env, fs, io, path, process};

fn usage() -> ! {
    println!(
        "Usage: {} [options]

  -s, --nosway           Disable Sway integration.
  -c, --config <config>  Specify a config file.
  -r, --replace          Replace existing gyr instances
      --clear_history    Clear launch history.
  -p, --program <name>   Launch program directly (bypass TUI).
  -ss <search>           Pre-fill search in TUI (must be last option).
  -v, --verbose          Increase verbosity level (multiple).
      --no-exec          Print selected application to stdout instead of launching.
      --systemd-run      Launch applications using systemd-run --user --scope.
      --uwsm             Launch applications using uwsm app.
      --dmenu            Dmenu mode: read from stdin, output selection to stdout.
      --cclip            Clipboard history mode: browse cclip history with previews.
      --with-nth <cols>  Display only specified columns (comma-separated, e.g., 1,3).
      --delimiter <char> Column delimiter for --with-nth (default: space).
  -h, --help             Show this help message.
  -V, --version          Show the version number and quit.
",
        &env::args().next().unwrap_or_else(|| "gyr".to_string())
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
    /// Command to run Terminal=true apps
    pub terminal_launcher: String,
    /// Replace already running instance of Gyr
    pub replace: bool,
    /// Enable Sway integration (default when `$SWAYSOCK` is not empty)
    pub sway: bool,
    /// Cursor character for the search
    pub cursor: String,
    /// Verbosity level
    pub verbose: Option<u64>,
    /// Don't scroll past the last/first item
    pub hard_stop: bool,
    /// Print selected application to stdout instead of launching
    pub no_exec: bool,
    /// Launch applications using systemd-run --user --scope
    pub systemd_run: bool,
    /// Launch applications using uwsm app
    pub uwsm: bool,
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
    /// Layout configuration
    pub title_panel_height_percent: u16,
    pub input_panel_height: u16,
    /// Program name for direct launch (bypasses TUI)
    pub program: Option<String>,
    /// Search string to pre-populate in TUI
    pub search_string: Option<String>,
    /// Dmenu mode settings
    pub dmenu_mode: bool,
    pub dmenu_with_nth: Option<Vec<usize>>,
    pub dmenu_delimiter: String,
    pub dmenu_show_line_numbers: bool,
    pub dmenu_wrap_long_lines: bool,
    /// Clipboard history mode settings
    pub cclip_mode: bool,
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
    pub dmenu_content_panel_height_percent: Option<u16>,
    pub dmenu_input_panel_height: Option<u16>,
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
    pub cclip_content_panel_height_percent: Option<u16>,
    pub cclip_input_panel_height: Option<u16>,
    pub cclip_show_line_numbers: Option<bool>,
    pub cclip_wrap_long_lines: Option<bool>,
    pub cclip_image_preview: Option<bool>,
}

impl Default for Opts {
    fn default() -> Self {
        Self {
            highlight_color: ratatui::style::Color::LightBlue,
            clear_history: false,
            terminal_launcher: "alacritty -e".to_string(),
            replace: false,
            sway: false,
            cursor: "█".to_string(),
            verbose: None,
            hard_stop: false,
            no_exec: false,
            systemd_run: false,
            uwsm: false,
            rounded_borders: true,
            main_border_color: ratatui::style::Color::White,
            apps_border_color: ratatui::style::Color::White,
            input_border_color: ratatui::style::Color::White,
            main_text_color: ratatui::style::Color::White,
            apps_text_color: ratatui::style::Color::White,
            input_text_color: ratatui::style::Color::White,
            fancy_mode: false,
            header_title_color: ratatui::style::Color::White,
            title_panel_height_percent: 30,
            input_panel_height: 3,
            program: None,
            search_string: None,
            // Dmenu mode defaults
            dmenu_mode: false,
            dmenu_with_nth: None,
            dmenu_delimiter: " ".to_string(),
            dmenu_show_line_numbers: false,
            dmenu_wrap_long_lines: true,
            // Cclip mode defaults
            cclip_mode: false,
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
            dmenu_content_panel_height_percent: None,
            dmenu_input_panel_height: None,
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
            cclip_content_panel_height_percent: None,
            cclip_input_panel_height: None,
            cclip_show_line_numbers: None,
            cclip_wrap_long_lines: None,
            cclip_image_preview: None,
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
            Long("clear_history") => {
                default.clear_history = true;
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
            Long("dmenu") => {
                default.dmenu_mode = true;
            }
            Long("cclip") => {
                default.cclip_mode = true;
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
            Short('h') | Long("help") => {
                usage();
            }
            Short('V') | Long("version") => {
                println!("{}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            _ => return Err(arg.unexpected()),
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
                Ok(content) => match FileConf::read(&content) {
                    Ok(conf) => {
                        file_conf = Some(conf);
                    }
                    Err(e) => {
                        println!(
                            "Error reading config file {}:\n\t{}",
                            f.display(),
                            e.message()
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
        if let Some(height) = dmenu_conf.content_panel_height_percent {
            if height >= 10 && height <= 70 {
                default.dmenu_content_panel_height_percent = Some(height);
            } else {
                eprintln!("Warning: dmenu content_panel_height_percent must be between 10-70%, using default");
            }
        }
        if let Some(height) = dmenu_conf.input_panel_height {
            if height >= 1 && height <= 10 {
                default.dmenu_input_panel_height = Some(height);
            } else {
                eprintln!("Warning: dmenu input_panel_height must be between 1-10 lines, using default");
            }
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
        if let Some(height) = cclip_conf.content_panel_height_percent {
            if height >= 10 && height <= 70 {
                default.cclip_content_panel_height_percent = Some(height);
            } else {
                eprintln!("Warning: cclip content_panel_height_percent must be between 10-70%, using default");
            }
        }
        if let Some(height) = cclip_conf.input_panel_height {
            if height >= 1 && height <= 10 {
                default.cclip_input_panel_height = Some(height);
            } else {
                eprintln!("Warning: cclip input_panel_height must be between 1-10 lines, using default");
            }
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
    }

    // Validate mutually exclusive options
    if default.program.is_some() && default.search_string.is_some() {
        eprintln!("Error: Cannot use -p/--program and -ss together");
        eprintln!("Use -p for direct launch or -ss for pre-filled TUI search");
        std::process::exit(1);
    }
    
    // Validate dmenu mode conflicts
    if default.dmenu_mode {
        if default.program.is_some() || default.search_string.is_some() {
            eprintln!("Error: --dmenu cannot be used with -p/--program or -ss");
            eprintln!("Dmenu mode reads from stdin and outputs to stdout");
            std::process::exit(1);
        }
        // dmenu mode implies no-exec behavior
        default.no_exec = true;
    }
    
    // Validate cclip mode conflicts
    if default.cclip_mode {
        if default.program.is_some() || default.search_string.is_some() {
            eprintln!("Error: --cclip cannot be used with -p/--program or -ss");
            eprintln!("Cclip mode browses clipboard history and copies selection");
            std::process::exit(1);
        }
        // cclip mode implies no-exec behavior
        default.no_exec = true;
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
    /// Use rounded borders (default: true)
    pub rounded_borders: Option<bool>,
    /// Border color for the main panel (Gyr)
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
    /// Title panel height percentage (10-70%)
    pub title_panel_height_percent: Option<u16>,
    /// Input panel height in lines
    pub input_panel_height: Option<u16>,
    /// Dmenu-specific configuration
    pub dmenu: Option<DmenuConf>,
    /// Cclip-specific configuration
    pub cclip: Option<CclipConf>,
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
    pub content_panel_height_percent: Option<u16>,
    pub input_panel_height: Option<u16>,
    /// Default delimiter for column parsing
    pub delimiter: Option<String>,
    /// Show line numbers in selection
    pub show_line_numbers: Option<bool>,
    /// Wrap long lines in content display
    pub wrap_long_lines: Option<bool>,
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
    pub content_panel_height_percent: Option<u16>,
    pub input_panel_height: Option<u16>,
    /// Show line numbers in selection
    pub show_line_numbers: Option<bool>,
    /// Wrap long lines in content display
    pub wrap_long_lines: Option<bool>,
    /// Enable image previews using chafa
    pub image_preview: Option<bool>,
}

impl FileConf {
    /// Parse a file.
    pub fn read(raw: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(raw)
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
