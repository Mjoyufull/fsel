use super::helpers::{BOOLEAN_EXPECTED, OverrideSource, set_parsed, set_string};
use crate::config::{ConfigError, FselConfig};

pub(super) fn apply(cfg: &mut FselConfig, source: &impl OverrideSource) -> Result<(), ConfigError> {
    set_string(source, "FSEL_HIGHLIGHT_COLOR", &mut cfg.ui.highlight_color);
    set_string(
        source,
        "FSEL_APPS_BACKGROUND_COLOR",
        &mut cfg.ui.apps_background_color,
    );
    set_string(
        source,
        "FSEL_APPS_SELECTION_BACKGROUND_COLOR",
        &mut cfg.ui.apps_selection_background_color,
    );
    set_string(source, "FSEL_CURSOR", &mut cfg.ui.cursor);
    set_parsed(
        source,
        "FSEL_HARD_STOP",
        &mut cfg.ui.hard_stop,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_ROUNDED_BORDERS",
        &mut cfg.ui.rounded_borders,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_DISABLE_MOUSE",
        &mut cfg.ui.disable_mouse,
        BOOLEAN_EXPECTED,
    )?;
    Ok(())
}
