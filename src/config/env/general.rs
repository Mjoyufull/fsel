use super::helpers::{
    BOOLEAN_EXPECTED, INTEGER_EXPECTED, MATCH_MODE_EXPECTED, OverrideSource, PINNED_ORDER_EXPECTED,
    RANKING_MODE_EXPECTED, set_parsed, set_string,
};
use crate::config::{ConfigError, FselConfig};

pub(super) fn apply(cfg: &mut FselConfig, source: &impl OverrideSource) -> Result<(), ConfigError> {
    set_string(
        source,
        "FSEL_TERMINAL_LAUNCHER",
        &mut cfg.general.terminal_launcher,
    );
    set_parsed(
        source,
        "FSEL_FILTER_DESKTOP",
        &mut cfg.general.filter_desktop,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_FILTER_ACTIONS",
        &mut cfg.general.filter_actions,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_LIST_EXECUTABLES_IN_PATH",
        &mut cfg.general.list_executables_in_path,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_HIDE_BEFORE_TYPING",
        &mut cfg.general.hide_before_typing,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_MATCH_MODE",
        &mut cfg.general.match_mode,
        MATCH_MODE_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_RANKING_MODE",
        &mut cfg.general.ranking_mode,
        RANKING_MODE_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_PINNED_ORDER",
        &mut cfg.general.pinned_order,
        PINNED_ORDER_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_SYSTEMD_RUN",
        &mut cfg.general.systemd_run,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(source, "FSEL_UWSM", &mut cfg.general.uwsm, BOOLEAN_EXPECTED)?;
    set_parsed(
        source,
        "FSEL_DETACH",
        &mut cfg.general.detach,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_NO_EXEC",
        &mut cfg.general.no_exec,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_CONFIRM_FIRST_LAUNCH",
        &mut cfg.general.confirm_first_launch,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_PREFIX_DEPTH",
        &mut cfg.general.prefix_depth,
        INTEGER_EXPECTED,
    )?;
    Ok(())
}
