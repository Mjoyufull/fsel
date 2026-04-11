use super::helpers::{INTEGER_EXPECTED, OverrideSource, PANEL_POSITION_EXPECTED, set_parsed};
use crate::config::{ConfigError, FselConfig};

pub(super) fn apply(cfg: &mut FselConfig, source: &impl OverrideSource) -> Result<(), ConfigError> {
    set_parsed(
        source,
        "FSEL_TITLE_PANEL_HEIGHT_PERCENT",
        &mut cfg.layout.title_panel_height_percent,
        INTEGER_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_INPUT_PANEL_HEIGHT",
        &mut cfg.layout.input_panel_height,
        INTEGER_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_TITLE_PANEL_POSITION",
        &mut cfg.layout.title_panel_position,
        PANEL_POSITION_EXPECTED,
    )?;
    Ok(())
}
