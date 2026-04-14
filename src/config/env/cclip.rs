use super::helpers::{
    BOOLEAN_EXPECTED, INTEGER_EXPECTED, OverrideSource, PANEL_POSITION_EXPECTED,
    set_optional_parsed, set_optional_string,
};
use crate::config::{ConfigError, FselConfig};

pub(super) fn apply(cfg: &mut FselConfig, source: &impl OverrideSource) -> Result<(), ConfigError> {
    set_optional_parsed(
        source,
        "FSEL_CCLIP_IMAGE_PREVIEW",
        &mut cfg.cclip.image_preview,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_CCLIP_HIDE_INLINE_IMAGE_MESSAGE",
        &mut cfg.cclip.hide_inline_image_message,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_CCLIP_SHOW_TAG_COLOR_NAMES",
        &mut cfg.cclip.show_tag_color_names,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_CCLIP_SHOW_LINE_NUMBERS",
        &mut cfg.cclip.show_line_numbers,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_CCLIP_WRAP_LONG_LINES",
        &mut cfg.cclip.wrap_long_lines,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_CCLIP_DISABLE_MOUSE",
        &mut cfg.cclip.disable_mouse,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_CCLIP_HARD_STOP",
        &mut cfg.cclip.hard_stop,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_CCLIP_ROUNDED_BORDERS",
        &mut cfg.cclip.rounded_borders,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_string(source, "FSEL_CCLIP_CURSOR", &mut cfg.cclip.cursor);
    set_optional_string(
        source,
        "FSEL_CCLIP_HIGHLIGHT_COLOR",
        &mut cfg.cclip.highlight_color,
    );
    set_optional_string(
        source,
        "FSEL_CCLIP_MAIN_BORDER_COLOR",
        &mut cfg.cclip.main_border_color,
    );
    set_optional_string(
        source,
        "FSEL_CCLIP_ITEMS_BORDER_COLOR",
        &mut cfg.cclip.items_border_color,
    );
    set_optional_string(
        source,
        "FSEL_CCLIP_INPUT_BORDER_COLOR",
        &mut cfg.cclip.input_border_color,
    );
    set_optional_string(
        source,
        "FSEL_CCLIP_MAIN_TEXT_COLOR",
        &mut cfg.cclip.main_text_color,
    );
    set_optional_string(
        source,
        "FSEL_CCLIP_ITEMS_TEXT_COLOR",
        &mut cfg.cclip.items_text_color,
    );
    set_optional_string(
        source,
        "FSEL_CCLIP_INPUT_TEXT_COLOR",
        &mut cfg.cclip.input_text_color,
    );
    set_optional_string(
        source,
        "FSEL_CCLIP_HEADER_TITLE_COLOR",
        &mut cfg.cclip.header_title_color,
    );
    set_optional_parsed(
        source,
        "FSEL_CCLIP_TITLE_PANEL_HEIGHT_PERCENT",
        &mut cfg.cclip.title_panel_height_percent,
        INTEGER_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_CCLIP_INPUT_PANEL_HEIGHT",
        &mut cfg.cclip.input_panel_height,
        INTEGER_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_CCLIP_TITLE_PANEL_POSITION",
        &mut cfg.cclip.title_panel_position,
        PANEL_POSITION_EXPECTED,
    )?;
    Ok(())
}
