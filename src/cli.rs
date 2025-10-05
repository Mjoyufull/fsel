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


    // Validate mutually exclusive options
    if default.program.is_some() && default.search_string.is_some() {
        eprintln!("Error: Cannot use -p/--program and -ss together");
        eprintln!("Use -p for direct launch or -ss for pre-filled TUI search");
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
