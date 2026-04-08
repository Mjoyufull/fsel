use super::{ConfigError, FselConfig};
use std::env;
use std::str::FromStr;

const BOOLEAN_EXPECTED: &str = "true or false";
const INTEGER_EXPECTED: &str = "an unsigned integer";
const MATCH_MODE_EXPECTED: &str = "'fuzzy' or 'exact'";
const RANKING_MODE_EXPECTED: &str = "'frecency', 'recency', or 'frequency'";
const PINNED_ORDER_EXPECTED: &str =
    "'ranking', 'alphabetical', 'oldest', 'oldest_pinned', 'newest', or 'newest_pinned'";
const PANEL_POSITION_EXPECTED: &str = "'top', 'middle', or 'bottom'";
const LAUNCH_PREFIX_EXPECTED: &str = "a shell-words command prefix";

pub(super) fn apply_env_overrides(cfg: &mut FselConfig) -> Result<(), ConfigError> {
    apply_general_overrides(cfg)?;
    apply_ui_overrides(cfg)?;
    apply_layout_overrides(cfg)?;
    apply_dmenu_overrides(cfg)?;
    apply_cclip_overrides(cfg)?;
    apply_app_launcher_overrides(cfg)?;
    Ok(())
}

fn apply_general_overrides(cfg: &mut FselConfig) -> Result<(), ConfigError> {
    set_string("FSEL_TERMINAL_LAUNCHER", &mut cfg.general.terminal_launcher);
    set_parsed(
        "FSEL_FILTER_DESKTOP",
        &mut cfg.general.filter_desktop,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        "FSEL_LIST_EXECUTABLES_IN_PATH",
        &mut cfg.general.list_executables_in_path,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        "FSEL_HIDE_BEFORE_TYPING",
        &mut cfg.general.hide_before_typing,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        "FSEL_MATCH_MODE",
        &mut cfg.general.match_mode,
        MATCH_MODE_EXPECTED,
    )?;
    set_parsed(
        "FSEL_RANKING_MODE",
        &mut cfg.general.ranking_mode,
        RANKING_MODE_EXPECTED,
    )?;
    set_parsed(
        "FSEL_PINNED_ORDER",
        &mut cfg.general.pinned_order,
        PINNED_ORDER_EXPECTED,
    )?;
    set_parsed(
        "FSEL_SYSTEMD_RUN",
        &mut cfg.general.systemd_run,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed("FSEL_UWSM", &mut cfg.general.uwsm, BOOLEAN_EXPECTED)?;
    set_parsed("FSEL_DETACH", &mut cfg.general.detach, BOOLEAN_EXPECTED)?;
    set_parsed("FSEL_NO_EXEC", &mut cfg.general.no_exec, BOOLEAN_EXPECTED)?;
    set_parsed(
        "FSEL_CONFIRM_FIRST_LAUNCH",
        &mut cfg.general.confirm_first_launch,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        "FSEL_PREFIX_DEPTH",
        &mut cfg.general.prefix_depth,
        INTEGER_EXPECTED,
    )?;
    Ok(())
}

fn apply_ui_overrides(cfg: &mut FselConfig) -> Result<(), ConfigError> {
    set_string("FSEL_HIGHLIGHT_COLOR", &mut cfg.ui.highlight_color);
    set_string("FSEL_CURSOR", &mut cfg.ui.cursor);
    set_parsed("FSEL_HARD_STOP", &mut cfg.ui.hard_stop, BOOLEAN_EXPECTED)?;
    set_parsed(
        "FSEL_ROUNDED_BORDERS",
        &mut cfg.ui.rounded_borders,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        "FSEL_DISABLE_MOUSE",
        &mut cfg.ui.disable_mouse,
        BOOLEAN_EXPECTED,
    )?;
    Ok(())
}

fn apply_layout_overrides(cfg: &mut FselConfig) -> Result<(), ConfigError> {
    set_parsed(
        "FSEL_TITLE_PANEL_HEIGHT_PERCENT",
        &mut cfg.layout.title_panel_height_percent,
        INTEGER_EXPECTED,
    )?;
    set_parsed(
        "FSEL_INPUT_PANEL_HEIGHT",
        &mut cfg.layout.input_panel_height,
        INTEGER_EXPECTED,
    )?;
    set_parsed(
        "FSEL_TITLE_PANEL_POSITION",
        &mut cfg.layout.title_panel_position,
        PANEL_POSITION_EXPECTED,
    )?;
    Ok(())
}

fn apply_dmenu_overrides(cfg: &mut FselConfig) -> Result<(), ConfigError> {
    set_optional_string("FSEL_DMENU_DELIMITER", &mut cfg.dmenu.delimiter);
    set_optional_string(
        "FSEL_DMENU_PASSWORD_CHARACTER",
        &mut cfg.dmenu.password_character,
    );
    set_optional_parsed(
        "FSEL_DMENU_SHOW_LINE_NUMBERS",
        &mut cfg.dmenu.show_line_numbers,
        false,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        "FSEL_DMENU_WRAP_LONG_LINES",
        &mut cfg.dmenu.wrap_long_lines,
        false,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        "FSEL_DMENU_EXIT_IF_EMPTY",
        &mut cfg.dmenu.exit_if_empty,
        false,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        "FSEL_DMENU_DISABLE_MOUSE",
        &mut cfg.dmenu.disable_mouse,
        false,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        "FSEL_DMENU_HARD_STOP",
        &mut cfg.dmenu.hard_stop,
        false,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        "FSEL_DMENU_ROUNDED_BORDERS",
        &mut cfg.dmenu.rounded_borders,
        false,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_string("FSEL_DMENU_CURSOR", &mut cfg.dmenu.cursor);
    set_optional_string("FSEL_DMENU_HIGHLIGHT_COLOR", &mut cfg.dmenu.highlight_color);
    set_optional_string(
        "FSEL_DMENU_MAIN_BORDER_COLOR",
        &mut cfg.dmenu.main_border_color,
    );
    set_optional_string(
        "FSEL_DMENU_ITEMS_BORDER_COLOR",
        &mut cfg.dmenu.items_border_color,
    );
    set_optional_string(
        "FSEL_DMENU_INPUT_BORDER_COLOR",
        &mut cfg.dmenu.input_border_color,
    );
    set_optional_string("FSEL_DMENU_MAIN_TEXT_COLOR", &mut cfg.dmenu.main_text_color);
    set_optional_string(
        "FSEL_DMENU_ITEMS_TEXT_COLOR",
        &mut cfg.dmenu.items_text_color,
    );
    set_optional_string(
        "FSEL_DMENU_INPUT_TEXT_COLOR",
        &mut cfg.dmenu.input_text_color,
    );
    set_optional_string(
        "FSEL_DMENU_HEADER_TITLE_COLOR",
        &mut cfg.dmenu.header_title_color,
    );
    set_optional_parsed(
        "FSEL_DMENU_TITLE_PANEL_HEIGHT_PERCENT",
        &mut cfg.dmenu.title_panel_height_percent,
        cfg.layout.title_panel_height_percent,
        INTEGER_EXPECTED,
    )?;
    set_optional_parsed(
        "FSEL_DMENU_INPUT_PANEL_HEIGHT",
        &mut cfg.dmenu.input_panel_height,
        cfg.layout.input_panel_height,
        INTEGER_EXPECTED,
    )?;
    set_optional_parsed(
        "FSEL_DMENU_TITLE_PANEL_POSITION",
        &mut cfg.dmenu.title_panel_position,
        cfg.layout.title_panel_position,
        PANEL_POSITION_EXPECTED,
    )?;
    Ok(())
}

fn apply_cclip_overrides(cfg: &mut FselConfig) -> Result<(), ConfigError> {
    set_optional_parsed(
        "FSEL_CCLIP_IMAGE_PREVIEW",
        &mut cfg.cclip.image_preview,
        false,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        "FSEL_CCLIP_HIDE_INLINE_IMAGE_MESSAGE",
        &mut cfg.cclip.hide_inline_image_message,
        false,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        "FSEL_CCLIP_SHOW_TAG_COLOR_NAMES",
        &mut cfg.cclip.show_tag_color_names,
        false,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        "FSEL_CCLIP_SHOW_LINE_NUMBERS",
        &mut cfg.cclip.show_line_numbers,
        false,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        "FSEL_CCLIP_WRAP_LONG_LINES",
        &mut cfg.cclip.wrap_long_lines,
        false,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        "FSEL_CCLIP_DISABLE_MOUSE",
        &mut cfg.cclip.disable_mouse,
        false,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        "FSEL_CCLIP_HARD_STOP",
        &mut cfg.cclip.hard_stop,
        false,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        "FSEL_CCLIP_ROUNDED_BORDERS",
        &mut cfg.cclip.rounded_borders,
        false,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_string("FSEL_CCLIP_CURSOR", &mut cfg.cclip.cursor);
    set_optional_string("FSEL_CCLIP_HIGHLIGHT_COLOR", &mut cfg.cclip.highlight_color);
    set_optional_string(
        "FSEL_CCLIP_MAIN_BORDER_COLOR",
        &mut cfg.cclip.main_border_color,
    );
    set_optional_string(
        "FSEL_CCLIP_ITEMS_BORDER_COLOR",
        &mut cfg.cclip.items_border_color,
    );
    set_optional_string(
        "FSEL_CCLIP_INPUT_BORDER_COLOR",
        &mut cfg.cclip.input_border_color,
    );
    set_optional_string("FSEL_CCLIP_MAIN_TEXT_COLOR", &mut cfg.cclip.main_text_color);
    set_optional_string(
        "FSEL_CCLIP_ITEMS_TEXT_COLOR",
        &mut cfg.cclip.items_text_color,
    );
    set_optional_string(
        "FSEL_CCLIP_INPUT_TEXT_COLOR",
        &mut cfg.cclip.input_text_color,
    );
    set_optional_string(
        "FSEL_CCLIP_HEADER_TITLE_COLOR",
        &mut cfg.cclip.header_title_color,
    );
    set_optional_parsed(
        "FSEL_CCLIP_TITLE_PANEL_HEIGHT_PERCENT",
        &mut cfg.cclip.title_panel_height_percent,
        cfg.layout.title_panel_height_percent,
        INTEGER_EXPECTED,
    )?;
    set_optional_parsed(
        "FSEL_CCLIP_INPUT_PANEL_HEIGHT",
        &mut cfg.cclip.input_panel_height,
        cfg.layout.input_panel_height,
        INTEGER_EXPECTED,
    )?;
    set_optional_parsed(
        "FSEL_CCLIP_TITLE_PANEL_POSITION",
        &mut cfg.cclip.title_panel_position,
        cfg.layout.title_panel_position,
        PANEL_POSITION_EXPECTED,
    )?;
    Ok(())
}

fn apply_app_launcher_overrides(cfg: &mut FselConfig) -> Result<(), ConfigError> {
    set_optional_parsed(
        "FSEL_APP_LAUNCHER_FILTER_DESKTOP",
        &mut cfg.app_launcher.filter_desktop,
        cfg.general.filter_desktop,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        "FSEL_APP_LAUNCHER_LIST_EXECUTABLES_IN_PATH",
        &mut cfg.app_launcher.list_executables_in_path,
        cfg.general.list_executables_in_path,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        "FSEL_APP_LAUNCHER_HIDE_BEFORE_TYPING",
        &mut cfg.app_launcher.hide_before_typing,
        cfg.general.hide_before_typing,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_launch_prefix(
        "FSEL_APP_LAUNCHER_LAUNCH_PREFIX",
        &mut cfg.app_launcher.launch_prefix,
    )?;
    set_optional_parsed(
        "FSEL_APP_LAUNCHER_MATCH_MODE",
        &mut cfg.app_launcher.match_mode,
        cfg.general.match_mode,
        MATCH_MODE_EXPECTED,
    )?;
    set_optional_parsed(
        "FSEL_APP_LAUNCHER_RANKING_MODE",
        &mut cfg.app_launcher.ranking_mode,
        cfg.general.ranking_mode,
        RANKING_MODE_EXPECTED,
    )?;
    set_optional_parsed(
        "FSEL_APP_LAUNCHER_PINNED_ORDER",
        &mut cfg.app_launcher.pinned_order,
        cfg.general.pinned_order,
        PINNED_ORDER_EXPECTED,
    )?;
    set_optional_parsed(
        "FSEL_APP_LAUNCHER_CONFIRM_FIRST_LAUNCH",
        &mut cfg.app_launcher.confirm_first_launch,
        cfg.general.confirm_first_launch,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        "FSEL_APP_LAUNCHER_PREFIX_DEPTH",
        &mut cfg.app_launcher.prefix_depth,
        cfg.general.prefix_depth,
        INTEGER_EXPECTED,
    )?;
    Ok(())
}

fn set_string(key: &str, target: &mut String) {
    if let Ok(value) = env::var(key) {
        *target = value;
    }
}

fn set_parsed<T>(key: &str, target: &mut T, expected: &'static str) -> Result<(), ConfigError>
where
    T: FromStr,
{
    if let Ok(value) = env::var(key) {
        match value.parse() {
            Ok(parsed) => *target = parsed,
            Err(_) => return Err(invalid_environment_override(key, value, expected)),
        }
    }

    Ok(())
}

fn set_optional_string(key: &str, target: &mut Option<String>) {
    if let Ok(value) = env::var(key) {
        *target = Some(value);
    }
}

fn set_optional_parsed<T>(
    key: &str,
    target: &mut Option<T>,
    _fallback: T,
    expected: &'static str,
) -> Result<(), ConfigError>
where
    T: FromStr,
{
    if let Ok(value) = env::var(key) {
        match value.parse() {
            Ok(parsed) => *target = Some(parsed),
            Err(_) => return Err(invalid_environment_override(key, value, expected)),
        }
    }

    Ok(())
}

fn set_optional_launch_prefix(
    key: &str,
    target: &mut Option<Vec<String>>,
) -> Result<(), ConfigError> {
    if let Ok(value) = env::var(key) {
        match shell_words::split(&value) {
            Ok(prefix) => *target = Some(prefix),
            Err(_) => {
                return Err(invalid_environment_override(
                    key,
                    value,
                    LAUNCH_PREFIX_EXPECTED,
                ));
            }
        }
    }

    Ok(())
}

fn invalid_environment_override(key: &str, value: String, expected: &'static str) -> ConfigError {
    ConfigError::InvalidEnvironmentOverride {
        key: key.to_string(),
        value,
        expected,
    }
}
