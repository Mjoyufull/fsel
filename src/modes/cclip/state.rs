use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::Color;

use crate::cli::{Opts, PanelPosition};
use crate::ui::{GraphicsAdapter, InputConfig};

pub(super) struct CclipOptions {
    pub(super) disable_mouse: bool,
    pub(super) hard_stop: bool,
    pub(super) wrap_long_lines: bool,
    pub(super) show_line_numbers: bool,
    pub(super) show_tag_color_names: bool,
    pub(super) hide_image_message: bool,
    pub(super) highlight_color: Color,
    pub(super) main_border_color: Color,
    pub(super) items_border_color: Color,
    pub(super) input_border_color: Color,
    pub(super) main_text_color: Color,
    pub(super) items_text_color: Color,
    pub(super) input_text_color: Color,
    pub(super) header_title_color: Color,
    pub(super) rounded_borders: bool,
    pub(super) content_panel_height_percent: u16,
    pub(super) input_panel_height: u16,
    pub(super) content_panel_position: PanelPosition,
    pub(super) cursor: String,
    pub(super) term_is_foot: bool,
    pub(super) graphics_adapter: GraphicsAdapter,
    pub(super) explicit_image_preview: Option<bool>,
}

impl CclipOptions {
    pub(super) fn from_cli(cli: &Opts) -> Self {
        Self {
            disable_mouse: cli
                .cclip_disable_mouse
                .or(cli.dmenu_disable_mouse)
                .unwrap_or(cli.disable_mouse),
            hard_stop: cli
                .cclip_hard_stop
                .or(cli.dmenu_hard_stop)
                .unwrap_or(cli.hard_stop),
            wrap_long_lines: cli.cclip_wrap_long_lines.unwrap_or(true),
            show_line_numbers: super::items::show_line_numbers(cli),
            show_tag_color_names: cli.cclip_show_tag_color_names.unwrap_or(false),
            hide_image_message: cli.cclip_hide_inline_image_message.unwrap_or(false),
            highlight_color: cli
                .cclip_highlight_color
                .or(cli.dmenu_highlight_color)
                .unwrap_or(cli.highlight_color),
            main_border_color: cli
                .cclip_main_border_color
                .or(cli.dmenu_main_border_color)
                .unwrap_or(cli.main_border_color),
            items_border_color: cli
                .cclip_items_border_color
                .or(cli.dmenu_items_border_color)
                .unwrap_or(cli.apps_border_color),
            input_border_color: cli
                .cclip_input_border_color
                .or(cli.dmenu_input_border_color)
                .unwrap_or(cli.input_border_color),
            main_text_color: cli
                .cclip_main_text_color
                .or(cli.dmenu_main_text_color)
                .unwrap_or(cli.main_text_color),
            items_text_color: cli
                .cclip_items_text_color
                .or(cli.dmenu_items_text_color)
                .unwrap_or(cli.apps_text_color),
            input_text_color: cli
                .cclip_input_text_color
                .or(cli.dmenu_input_text_color)
                .unwrap_or(cli.input_text_color),
            header_title_color: cli
                .cclip_header_title_color
                .or(cli.dmenu_header_title_color)
                .unwrap_or(cli.header_title_color),
            rounded_borders: cli
                .cclip_rounded_borders
                .or(cli.dmenu_rounded_borders)
                .unwrap_or(cli.rounded_borders),
            content_panel_height_percent: cli
                .cclip_title_panel_height_percent
                .or(cli.dmenu_title_panel_height_percent)
                .unwrap_or(cli.title_panel_height_percent),
            input_panel_height: cli
                .cclip_input_panel_height
                .or(cli.dmenu_input_panel_height)
                .unwrap_or(cli.input_panel_height),
            content_panel_position: cli
                .cclip_title_panel_position
                .or(cli.dmenu_title_panel_position)
                .unwrap_or(cli.title_panel_position.unwrap_or(PanelPosition::Top)),
            cursor: cli
                .cclip_cursor
                .clone()
                .or(cli.dmenu_cursor.clone())
                .unwrap_or_else(|| cli.cursor.clone()),
            term_is_foot: std::env::var("TERM")
                .unwrap_or_default()
                .starts_with("foot"),
            graphics_adapter: GraphicsAdapter::detect(None),
            explicit_image_preview: cli.cclip_image_preview,
        }
    }

    pub(super) fn set_graphics_adapter(&mut self, adapter: GraphicsAdapter) {
        self.graphics_adapter = adapter;
    }

    pub(super) fn input_config(&self) -> InputConfig {
        InputConfig {
            exit_key: KeyCode::Null,
            disable_mouse: self.disable_mouse,
            render_rate: None,
            ..InputConfig::default()
        }
    }

    pub(super) fn content_height(&self, total_height: u16) -> u16 {
        crate::ui::effective_content_height(total_height, self.content_panel_height_percent)
    }

    pub(super) fn split_layout(&self, area: Rect) -> crate::ui::PanelLayout {
        crate::ui::split_content_panels(
            area,
            self.content_height(area.height),
            self.input_panel_height,
            self.content_panel_position,
        )
    }

    pub(super) fn items_panel_height(&self, total_height: u16) -> u16 {
        crate::ui::items_panel_height(
            total_height,
            self.content_height(total_height),
            self.input_panel_height,
        )
    }

    pub(super) fn max_visible_items(&self, total_height: u16) -> usize {
        self.items_panel_height(total_height).saturating_sub(2) as usize
    }

    pub(super) fn items_panel_bounds(&self, total_height: u16) -> (u16, u16) {
        crate::ui::items_panel_bounds(
            total_height,
            self.content_height(total_height),
            self.input_panel_height,
            self.content_panel_position,
        )
    }

    pub(super) fn image_preview_enabled(&self, supports_graphics: bool) -> bool {
        self.explicit_image_preview.unwrap_or(supports_graphics)
    }
}
