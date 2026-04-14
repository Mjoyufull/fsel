use super::helpers::{
    BOOLEAN_EXPECTED, INTEGER_EXPECTED, MATCH_MODE_EXPECTED, OverrideSource, PINNED_ORDER_EXPECTED,
    RANKING_MODE_EXPECTED, set_optional_launch_prefix, set_optional_parsed,
};
use crate::config::{ConfigError, FselConfig};

pub(super) fn apply(cfg: &mut FselConfig, source: &impl OverrideSource) -> Result<(), ConfigError> {
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_FILTER_DESKTOP",
        &mut cfg.app_launcher.filter_desktop,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_FILTER_ACTIONS",
        &mut cfg.app_launcher.filter_actions,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_LIST_EXECUTABLES_IN_PATH",
        &mut cfg.app_launcher.list_executables_in_path,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_HIDE_BEFORE_TYPING",
        &mut cfg.app_launcher.hide_before_typing,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_launch_prefix(
        source,
        "FSEL_APP_LAUNCHER_LAUNCH_PREFIX",
        &mut cfg.app_launcher.launch_prefix,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_MATCH_MODE",
        &mut cfg.app_launcher.match_mode,
        MATCH_MODE_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_RANKING_MODE",
        &mut cfg.app_launcher.ranking_mode,
        RANKING_MODE_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_PINNED_ORDER",
        &mut cfg.app_launcher.pinned_order,
        PINNED_ORDER_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_CONFIRM_FIRST_LAUNCH",
        &mut cfg.app_launcher.confirm_first_launch,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_PREFIX_DEPTH",
        &mut cfg.app_launcher.prefix_depth,
        INTEGER_EXPECTED,
    )?;
    Ok(())
}
