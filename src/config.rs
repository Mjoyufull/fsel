use directories::ProjectDirs;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::PathBuf;

/// Error type for config loading
#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Toml(toml::de::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "IO error: {}", e),
            ConfigError::Toml(e) => write!(f, "TOML parse error: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        ConfigError::Io(e)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(e: toml::de::Error) -> Self {
        ConfigError::Toml(e)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct FselConfig {
    #[serde(flatten)]
    pub general: GeneralConfig,
    #[serde(flatten)]
    pub ui: UiConfig,
    #[serde(flatten)]
    pub layout: LayoutConfig,

    // mode-specific configs live under their own sections
    #[serde(default)]
    pub dmenu: DmenuConfig,
    #[serde(default)]
    pub cclip: CclipConfig,

    // Legacy [app_launcher] section - takes precedence over root-level settings
    #[serde(default)]
    pub app_launcher: AppLauncherConfig,
}

/// Legacy [app_launcher] section for backward compatibility
#[derive(Debug, Deserialize, Clone, Default)]
pub struct AppLauncherConfig {
    pub filter_desktop: Option<bool>,
    pub list_executables_in_path: Option<bool>,
    pub hide_before_typing: Option<bool>,
    pub match_mode: Option<String>,
    pub confirm_first_launch: Option<bool>,
    pub prefix_depth: Option<usize>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct GeneralConfig {
    #[serde(default = "default_terminal_launcher")]
    pub terminal_launcher: String,
    #[serde(default = "default_true")]
    pub filter_desktop: bool,
    #[serde(default)]
    pub list_executables_in_path: bool,
    #[serde(default)]
    pub hide_before_typing: bool,
    #[serde(default = "default_match_mode")]
    pub match_mode: String,
    #[serde(default)]
    pub sway: bool,
    #[serde(default)]
    pub systemd_run: bool,
    #[serde(default)]
    pub uwsm: bool,
    #[serde(default)]
    pub detach: bool,
    #[serde(default)]
    pub no_exec: bool,
    #[serde(default)]
    pub confirm_first_launch: bool,
    #[serde(default = "default_prefix_depth")]
    pub prefix_depth: usize,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct UiConfig {
    #[serde(default = "default_highlight_color")]
    pub highlight_color: String,
    #[serde(default = "default_cursor")]
    pub cursor: String,
    #[serde(default)]
    pub hard_stop: bool,
    #[serde(default = "default_true")]
    pub rounded_borders: bool,
    #[serde(default)]
    pub disable_mouse: bool,
    #[serde(default = "default_white")]
    pub main_border_color: String,
    #[serde(default = "default_white")]
    pub apps_border_color: String,
    #[serde(default = "default_white")]
    pub input_border_color: String,
    #[serde(default = "default_white")]
    pub main_text_color: String,
    #[serde(default = "default_white")]
    pub apps_text_color: String,
    #[serde(default = "default_white")]
    pub input_text_color: String,
    #[serde(default)]
    pub fancy_mode: bool,
    #[serde(default = "default_white")]
    pub header_title_color: String,
    #[serde(default = "default_pin_color")]
    pub pin_color: String,
    #[serde(default = "default_pin_icon")]
    pub pin_icon: String,
    #[serde(default)]
    pub keybinds: crate::ui::Keybinds,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct LayoutConfig {
    #[serde(default = "default_title_panel_height")]
    pub title_panel_height_percent: u16,
    #[serde(default = "default_input_panel_height")]
    pub input_panel_height: u16,
    #[serde(default = "default_title_panel_position")]
    pub title_panel_position: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct DmenuConfig {
    pub delimiter: Option<String>,
    pub password_character: Option<String>,
    pub show_line_numbers: Option<bool>,
    pub wrap_long_lines: Option<bool>,
    pub exit_if_empty: Option<bool>,
    pub disable_mouse: Option<bool>,
    pub hard_stop: Option<bool>,
    pub rounded_borders: Option<bool>,
    pub cursor: Option<String>,
    pub highlight_color: Option<String>,
    pub main_border_color: Option<String>,
    pub items_border_color: Option<String>,
    pub input_border_color: Option<String>,
    pub main_text_color: Option<String>,
    pub items_text_color: Option<String>,
    pub input_text_color: Option<String>,
    pub header_title_color: Option<String>,
    pub title_panel_height_percent: Option<u16>,
    pub input_panel_height: Option<u16>,
    pub title_panel_position: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct CclipConfig {
    pub image_preview: Option<bool>,
    pub hide_inline_image_message: Option<bool>,
    pub show_tag_color_names: Option<bool>,
    pub show_line_numbers: Option<bool>,
    pub wrap_long_lines: Option<bool>,
    pub disable_mouse: Option<bool>,
    pub hard_stop: Option<bool>,
    pub rounded_borders: Option<bool>,
    pub cursor: Option<String>,
    pub highlight_color: Option<String>,
    pub main_border_color: Option<String>,
    pub items_border_color: Option<String>,
    pub input_border_color: Option<String>,
    pub main_text_color: Option<String>,
    pub items_text_color: Option<String>,
    pub input_text_color: Option<String>,
    pub header_title_color: Option<String>,
    pub title_panel_height_percent: Option<u16>,
    pub input_panel_height: Option<u16>,
    pub title_panel_position: Option<String>,
}

// Default value implementations for serde
fn default_terminal_launcher() -> String {
    "alacritty -e".to_string()
}
fn default_true() -> bool {
    true
}
fn default_match_mode() -> String {
    "fuzzy".to_string()
}
fn default_highlight_color() -> String {
    "LightBlue".to_string()
}
fn default_cursor() -> String {
    "█".to_string()
}
fn default_white() -> String {
    "White".to_string()
}
fn default_pin_color() -> String {
    "rgb(255, 165, 0)".to_string()
}
fn default_pin_icon() -> String {
    "📌".to_string()
}
fn default_title_panel_height() -> u16 {
    30
}
fn default_input_panel_height() -> u16 {
    3
}
fn default_title_panel_position() -> String {
    "top".to_string()
}
fn default_prefix_depth() -> usize {
    3
}

impl Default for FselConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                terminal_launcher: default_terminal_launcher(),
                filter_desktop: true,
                list_executables_in_path: false,
                hide_before_typing: false,
                match_mode: default_match_mode(),
                sway: env::var("SWAYSOCK").is_ok(),
                systemd_run: false,
                uwsm: false,
                detach: false,
                no_exec: false,
                confirm_first_launch: false,
                prefix_depth: default_prefix_depth(),
            },
            ui: UiConfig {
                highlight_color: default_highlight_color(),
                cursor: default_cursor(),
                hard_stop: false,
                rounded_borders: true,
                disable_mouse: false,
                main_border_color: default_white(),
                apps_border_color: default_white(),
                input_border_color: default_white(),
                main_text_color: default_white(),
                apps_text_color: default_white(),
                input_text_color: default_white(),
                fancy_mode: false,
                header_title_color: default_white(),
                pin_color: default_pin_color(),
                pin_icon: default_pin_icon(),
                keybinds: crate::ui::Keybinds::default(),
            },
            layout: LayoutConfig {
                title_panel_height_percent: default_title_panel_height(),
                input_panel_height: default_input_panel_height(),
                title_panel_position: default_title_panel_position(),
            },
            dmenu: DmenuConfig::default(),
            cclip: CclipConfig::default(),
            app_launcher: AppLauncherConfig::default(),
        }
    }
}

impl FselConfig {
    pub fn new(cli_config_path: Option<PathBuf>) -> Result<Self, ConfigError> {
        // Determine config file path
        // Priority: CLI arg > XDG_CONFIG_HOME > Default fallback
        let cli_provided = cli_config_path.is_some();
        let config_path = if let Some(path) = cli_config_path {
            Some(path)
        } else if let Some(proj_dirs) = ProjectDirs::from("", "", "fsel") {
            let mut p = proj_dirs.config_dir().to_path_buf();
            p.push("config.toml");
            Some(p)
        } else {
            None
        };

        // Load config from file or use defaults
        let mut cfg: FselConfig = if let Some(ref path) = config_path {
            if path.exists() {
                let contents = fs::read_to_string(path)?;
                toml::from_str(&contents)?
            } else if cli_provided {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Config file not found at {}", path.display()),
                )
                .into());
            } else {
                FselConfig::default()
            }
        } else {
            FselConfig::default()
        };

        // Override with FSEL_* environment variables
        // This replicates the behavior of config-rs Environment source
        if let Ok(val) = env::var("FSEL_TERMINAL_LAUNCHER") {
            cfg.general.terminal_launcher = val;
        }
        if let Ok(val) = env::var("FSEL_FILTER_DESKTOP") {
            cfg.general.filter_desktop = val.parse().unwrap_or(cfg.general.filter_desktop);
        }
        if let Ok(val) = env::var("FSEL_LIST_EXECUTABLES_IN_PATH") {
            cfg.general.list_executables_in_path =
                val.parse().unwrap_or(cfg.general.list_executables_in_path);
        }
        if let Ok(val) = env::var("FSEL_HIDE_BEFORE_TYPING") {
            cfg.general.hide_before_typing = val.parse().unwrap_or(cfg.general.hide_before_typing);
        }
        if let Ok(val) = env::var("FSEL_MATCH_MODE") {
            cfg.general.match_mode = val;
        }
        if let Ok(val) = env::var("FSEL_SWAY") {
            cfg.general.sway = val.parse().unwrap_or(cfg.general.sway);
        }
        if let Ok(val) = env::var("FSEL_SYSTEMD_RUN") {
            cfg.general.systemd_run = val.parse().unwrap_or(cfg.general.systemd_run);
        }
        if let Ok(val) = env::var("FSEL_UWSM") {
            cfg.general.uwsm = val.parse().unwrap_or(cfg.general.uwsm);
        }
        if let Ok(val) = env::var("FSEL_DETACH") {
            cfg.general.detach = val.parse().unwrap_or(cfg.general.detach);
        }
        if let Ok(val) = env::var("FSEL_NO_EXEC") {
            cfg.general.no_exec = val.parse().unwrap_or(cfg.general.no_exec);
        }
        if let Ok(val) = env::var("FSEL_CONFIRM_FIRST_LAUNCH") {
            cfg.general.confirm_first_launch =
                val.parse().unwrap_or(cfg.general.confirm_first_launch);
        }
        if let Ok(val) = env::var("FSEL_PREFIX_DEPTH") {
            cfg.general.prefix_depth = val.parse().unwrap_or(cfg.general.prefix_depth);
        }
        if let Ok(val) = env::var("FSEL_HIGHLIGHT_COLOR") {
            cfg.ui.highlight_color = val;
        }
        if let Ok(val) = env::var("FSEL_CURSOR") {
            cfg.ui.cursor = val;
        }
        if let Ok(val) = env::var("FSEL_HARD_STOP") {
            cfg.ui.hard_stop = val.parse().unwrap_or(cfg.ui.hard_stop);
        }
        if let Ok(val) = env::var("FSEL_ROUNDED_BORDERS") {
            cfg.ui.rounded_borders = val.parse().unwrap_or(cfg.ui.rounded_borders);
        }
        if let Ok(val) = env::var("FSEL_DISABLE_MOUSE") {
            cfg.ui.disable_mouse = val.parse().unwrap_or(cfg.ui.disable_mouse);
        }

        Ok(cfg)
    }
}
