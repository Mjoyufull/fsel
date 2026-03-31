use super::schema::{
    AppLauncherConfig, CclipConfig, DmenuConfig, FselConfig, GeneralConfig, LayoutConfig, UiConfig,
};

pub(super) fn default_terminal_launcher() -> String {
    "alacritty -e".to_string()
}

pub(super) fn default_true() -> bool {
    true
}

pub(super) fn default_match_mode() -> String {
    "fuzzy".to_string()
}

pub(super) fn default_ranking_mode() -> String {
    "frecency".to_string()
}

pub(super) fn default_pinned_order() -> String {
    "ranking".to_string()
}

pub(super) fn default_highlight_color() -> String {
    "LightBlue".to_string()
}

pub(super) fn default_cursor() -> String {
    "█".to_string()
}

pub(super) fn default_white() -> String {
    "White".to_string()
}

pub(super) fn default_pin_color() -> String {
    "rgb(255, 165, 0)".to_string()
}

pub(super) fn default_pin_icon() -> String {
    "📌".to_string()
}

pub(super) fn default_title_panel_height() -> u16 {
    30
}

pub(super) fn default_input_panel_height() -> u16 {
    3
}

pub(super) fn default_title_panel_position() -> String {
    "top".to_string()
}

pub(super) fn default_prefix_depth() -> usize {
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
                ranking_mode: default_ranking_mode(),
                pinned_order: default_pinned_order(),
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
