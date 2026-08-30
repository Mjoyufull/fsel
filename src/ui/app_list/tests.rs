use super::{
    label_row, launcher_list_icon_area, launcher_visible_rows, list_areas,
    render_selection_background,
};
use crate::cli::{DesktopIconMode, Opts};
use crate::ui::{HorizontalPosition, PanelPosition, VerticalAlignment};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::style::Color;

#[test]
fn list_icons_reduce_visible_apps_by_configured_row_height() {
    let cli = Opts {
        desktop_icon_mode: DesktopIconMode::List,
        desktop_icon_list_height: 2,
        title_panel_height_percent: 25,
        input_panel_height: 3,
        ..Opts::default()
    };

    assert_eq!(launcher_visible_rows(40, &cli), 12);
}

#[test]
fn list_label_alignment_preserves_top_and_supports_center_and_bottom() {
    assert_eq!(label_row(3, VerticalAlignment::Top), 0);
    assert_eq!(label_row(3, VerticalAlignment::Center), 1);
    assert_eq!(label_row(3, VerticalAlignment::Bottom), 2);
    assert_eq!(label_row(2, VerticalAlignment::Center), 1);
}

#[test]
fn borderless_apps_panel_uses_the_released_rows() {
    let cli = Opts {
        desktop_icon_mode: DesktopIconMode::List,
        desktop_icon_list_height: 2,
        title_panel_height_percent: 25,
        input_panel_height: 3,
        show_apps_border: false,
        ..Opts::default()
    };

    assert_eq!(launcher_visible_rows(40, &cli), 13);
}

#[test]
fn visible_rows_saturates_when_panel_sizes_overflow() {
    let cli = Opts {
        title_panel_height_percent: u16::MAX,
        input_panel_height: u16::MAX,
        ..Opts::default()
    };

    assert_eq!(launcher_visible_rows(u16::MAX, &cli), 0);
}

#[test]
fn middle_title_position_uses_the_actual_apps_pane_height() {
    let cli = Opts {
        desktop_icon_mode: DesktopIconMode::List,
        desktop_icon_list_height: 2,
        title_panel_position: Some(PanelPosition::Middle),
        title_panel_height_percent: 25,
        input_panel_height: 3,
        ..Opts::default()
    };

    assert_eq!(launcher_visible_rows(40, &cli), 6);
}

#[test]
fn list_icons_can_reserve_the_right_side() {
    let cli = Opts {
        desktop_icon_mode: DesktopIconMode::Both,
        desktop_icon_position: HorizontalPosition::Right,
        desktop_icon_list_width: 4,
        ..Opts::default()
    };

    let areas = list_areas(Rect::new(10, 3, 30, 8), &cli);

    assert_eq!(areas.text, Rect::new(10, 3, 26, 8));
    assert_eq!(areas.icon, Some(Rect::new(36, 3, 4, 8)));
    assert_eq!(areas.selection, None);
}

#[test]
fn list_icons_can_reserve_the_left_side() {
    let cli = Opts {
        desktop_icon_mode: DesktopIconMode::List,
        desktop_icon_position: HorizontalPosition::Left,
        desktop_icon_list_width: 5,
        ..Opts::default()
    };

    let areas = list_areas(Rect::new(2, 4, 20, 6), &cli);

    assert_eq!(areas.text, Rect::new(7, 4, 15, 6));
    assert_eq!(areas.icon, Some(Rect::new(2, 4, 5, 6)));
    assert_eq!(areas.selection, None);
}

#[test]
fn selection_arrow_can_be_reserved_before_a_left_icon() {
    let cli = Opts {
        desktop_icon_mode: DesktopIconMode::List,
        desktop_icon_position: HorizontalPosition::Left,
        desktop_icon_list_width: 5,
        desktop_icon_arrow_before: true,
        ..Opts::default()
    };

    let areas = list_areas(Rect::new(2, 4, 20, 6), &cli);

    assert_eq!(areas.selection, Some(Rect::new(2, 4, 2, 6)));
    assert_eq!(areas.icon, Some(Rect::new(4, 4, 5, 6)));
    assert_eq!(areas.text, Rect::new(9, 4, 13, 6));
}

#[test]
fn hidden_selection_marker_releases_its_gutter() {
    let cli = Opts {
        desktop_icon_mode: DesktopIconMode::List,
        desktop_icon_position: HorizontalPosition::Left,
        desktop_icon_list_width: 5,
        desktop_icon_arrow_before: true,
        show_selection_marker: false,
        ..Opts::default()
    };

    let areas = list_areas(Rect::new(2, 4, 20, 6), &cli);

    assert_eq!(areas.selection, None);
    assert_eq!(areas.icon, Some(Rect::new(2, 4, 5, 6)));
    assert_eq!(areas.text, Rect::new(7, 4, 15, 6));
}

#[test]
fn list_icon_gap_is_reserved_between_icon_and_label() {
    let cli = Opts {
        desktop_icon_mode: DesktopIconMode::List,
        desktop_icon_position: HorizontalPosition::Left,
        desktop_icon_list_width: 5,
        desktop_icon_list_gap: 3,
        ..Opts::default()
    };

    let areas = list_areas(Rect::new(2, 4, 20, 6), &cli);

    assert_eq!(areas.icon, Some(Rect::new(2, 4, 5, 6)));
    assert_eq!(areas.text, Rect::new(10, 4, 12, 6));
}

#[test]
fn narrow_list_keeps_selection_and_label_space() {
    let cli = Opts {
        desktop_icon_mode: DesktopIconMode::List,
        desktop_icon_position: HorizontalPosition::Left,
        desktop_icon_list_width: 16,
        ..Opts::default()
    };

    let areas = list_areas(Rect::new(2, 4, 4, 6), &cli);

    assert_eq!(areas.icon, Some(Rect::new(2, 4, 1, 6)));
    assert_eq!(areas.text, Rect::new(3, 4, 3, 6));
}

#[test]
fn list_worker_area_matches_each_rendered_icon_slot() {
    let cli = Opts {
        desktop_icon_mode: DesktopIconMode::List,
        desktop_icon_list_width: 5,
        desktop_icon_list_height: 2,
        ..Opts::default()
    };

    assert_eq!(
        launcher_list_icon_area(Rect::new(0, 0, 100, 40), &cli),
        Rect::new(0, 0, 5, 2)
    );
}

#[test]
fn rounded_selection_uses_half_cell_caps() {
    let backend = TestBackend::new(5, 1);
    let mut terminal = Terminal::new(backend).expect("test terminal should initialize");
    terminal
        .draw(|frame| {
            render_selection_background(
                frame,
                Rect::new(0, 0, 5, 1),
                Color::Black,
                Color::Blue,
                true,
            );
        })
        .expect("selection should render");
    let buffer = terminal.backend().buffer();

    assert_eq!(buffer[(0, 0)].symbol(), "▐");
    assert_eq!(buffer[(0, 0)].fg, Color::Blue);
    assert_eq!(buffer[(0, 0)].bg, Color::Black);
    assert_eq!(buffer[(1, 0)].bg, Color::Blue);
    assert_eq!(buffer[(4, 0)].symbol(), "▌");
}
