use super::app_ui::AppIcons;
use crate::cli::Opts;
use crate::core::state::State;
use eyre::Result;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ListAreas {
    text: Rect,
    icon: Option<Rect>,
    selection: Option<Rect>,
}

pub(crate) fn launcher_visible_rows(total_height: u16, cli: &Opts) -> usize {
    let (_, _, apps_area) =
        super::app_ui::launcher_panel_areas(Rect::new(0, 0, 1, total_height), cli);
    visible_rows(apps_block(cli).inner(apps_area).height, cli)
}

fn visible_rows(content_height: u16, cli: &Opts) -> usize {
    usize::from(content_height / app_row_height(cli))
}

pub(crate) fn app_row_height(cli: &Opts) -> u16 {
    if cli.desktop_icon_mode.shows_list() {
        cli.desktop_icon_list_height.max(1)
    } else {
        1
    }
}

pub(crate) fn launcher_list_icon_area(size: Rect, cli: &Opts) -> Rect {
    let (_, _, apps_area) = super::app_ui::launcher_panel_areas(size, cli);
    let inner = apps_block(cli).inner(apps_area);
    let content = list_content_area(inner, cli);
    let Some(icon_strip) = list_areas(content, cli).icon else {
        return Rect::default();
    };
    Rect::new(0, 0, icon_strip.width, app_row_height(cli))
}

pub(super) fn render(
    frame: &mut Frame,
    state: &State,
    cli: &Opts,
    area: Rect,
    app_icons: Option<&mut AppIcons<'_>>,
) -> Result<bool> {
    let block = apps_block(cli);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let content = list_content_area(inner, cli);

    let row_height = app_row_height(cli);
    let max_visible = visible_rows(inner.height, cli);
    let visible_apps = state
        .shown
        .iter()
        .skip(state.scroll_offset)
        .take(max_visible)
        .collect::<Vec<_>>();
    let areas = list_areas(content, cli);

    let items = visible_apps
        .iter()
        .map(|app| {
            let mut spans = Vec::new();
            if app.pinned && cli.show_pin_icons {
                spans.push(Span::styled(
                    &cli.pin_icon,
                    Style::default().fg(cli.pin_color),
                ));
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled(
                &app.name,
                Style::default().fg(cli.apps_text_color),
            ));

            let mut lines = vec![Line::from(spans)];
            lines.resize_with(usize::from(row_height), Line::default);
            ListItem::new(lines)
        })
        .collect::<Vec<_>>();
    let highlight_style = Style::default()
        .fg(cli.highlight_color)
        .add_modifier(Modifier::BOLD);
    let arrow_before_icon = areas.selection.is_some();
    let list = List::new(items)
        .highlight_style(highlight_style)
        .highlight_symbol(if arrow_before_icon || !cli.show_selection_marker {
            ""
        } else {
            "> "
        })
        .highlight_spacing(if arrow_before_icon || !cli.show_selection_marker {
            HighlightSpacing::Never
        } else {
            HighlightSpacing::Always
        });
    let mut list_state = ListState::default();
    if let Some(selected) = state.selected
        && selected >= state.scroll_offset
        && selected < state.scroll_offset + max_visible
    {
        list_state.select(Some(selected - state.scroll_offset));
    }
    if let Some(selected) = list_state.selected() {
        let y = inner.y + selected as u16 * row_height;
        let height = row_height.min(inner.y + inner.height - y);
        render_selection_background(
            frame,
            Rect::new(inner.x, y, inner.width, height),
            cli.apps_background_color,
            cli.apps_selection_background_color,
            cli.apps_selection_rounded,
        );
    }
    frame.render_stateful_widget(list, areas.text, &mut list_state);

    if let (Some(selected), Some(selection_area)) = (list_state.selected(), areas.selection) {
        let y = selection_area.y + selected as u16 * row_height;
        frame.render_widget(
            Paragraph::new("> ").style(highlight_style),
            Rect::new(selection_area.x, y, selection_area.width, 1),
        );
    }

    let mut render_failed = false;
    if let (Some(icons), Some(icon_strip)) = (app_icons, areas.icon) {
        for (row, app) in visible_apps.iter().enumerate() {
            let Some(icon) = app.icon.as_ref() else {
                continue;
            };
            if icons.failed_list_icons.contains(icon) {
                continue;
            }
            let Some(key) = icons.list_keys.get(icon) else {
                continue;
            };
            if !icons.image_manager.is_cached(key) {
                continue;
            }
            let icon_area = Rect::new(
                icon_strip.x,
                icon_strip.y + row as u16 * row_height,
                icon_strip.width,
                row_height,
            );
            if !icons.image_manager.render_cached(frame, key, icon_area)? {
                icons.failed_list_icons.insert(icon.clone());
                render_failed = true;
            }
        }
    }

    Ok(render_failed)
}

fn apps_block(cli: &Opts) -> Block<'static> {
    let mut block = Block::default()
        .borders(if cli.show_apps_border {
            Borders::ALL
        } else {
            Borders::NONE
        })
        .style(Style::default().bg(cli.apps_background_color))
        .border_style(Style::default().fg(cli.apps_border_color))
        .border_type(if cli.rounded_borders {
            BorderType::Rounded
        } else {
            BorderType::Plain
        });
    if cli.show_panel_titles {
        block = block.title(Span::styled(
            " Apps ",
            Style::default().fg(cli.header_title_color),
        ));
    }
    block
}

fn list_areas(area: Rect, cli: &Opts) -> ListAreas {
    if !cli.desktop_icon_mode.shows_list() || area.width < 4 {
        return ListAreas {
            text: area,
            icon: None,
            selection: None,
        };
    }

    let marker_width = u16::from(cli.show_selection_marker).saturating_mul(2);
    let fixed_width = marker_width + 1;
    if area.width <= fixed_width {
        return ListAreas {
            text: area,
            icon: None,
            selection: None,
        };
    }
    let icon_width = cli.desktop_icon_list_width.min(area.width - fixed_width);
    let gap = cli
        .desktop_icon_list_gap
        .min(area.width.saturating_sub(icon_width + fixed_width));
    match cli.desktop_icon_position {
        super::HorizontalPosition::Left
            if cli.desktop_icon_arrow_before && cli.show_selection_marker =>
        {
            ListAreas {
                selection: Some(Rect::new(area.x, area.y, 2, area.height)),
                icon: Some(Rect::new(area.x + 2, area.y, icon_width, area.height)),
                text: Rect::new(
                    area.x + 2 + icon_width + gap,
                    area.y,
                    area.width - 2 - icon_width - gap,
                    area.height,
                ),
            }
        }
        super::HorizontalPosition::Left => ListAreas {
            text: Rect::new(
                area.x + icon_width + gap,
                area.y,
                area.width - icon_width - gap,
                area.height,
            ),
            icon: Some(Rect::new(area.x, area.y, icon_width, area.height)),
            selection: None,
        },
        super::HorizontalPosition::Right => ListAreas {
            text: Rect::new(area.x, area.y, area.width - icon_width - gap, area.height),
            icon: Some(Rect::new(
                area.x + area.width - icon_width,
                area.y,
                icon_width,
                area.height,
            )),
            selection: None,
        },
        // Center is a title-preview placement. Keep list icons on the default side.
        super::HorizontalPosition::Center => ListAreas {
            text: Rect::new(area.x, area.y, area.width - icon_width - gap, area.height),
            icon: Some(Rect::new(
                area.x + area.width - icon_width,
                area.y,
                icon_width,
                area.height,
            )),
            selection: None,
        },
    }
}

fn list_content_area(area: Rect, cli: &Opts) -> Rect {
    if cli.apps_selection_rounded && area.width > 2 {
        area.inner(ratatui::layout::Margin {
            horizontal: 1,
            vertical: 0,
        })
    } else {
        area
    }
}

fn render_selection_background(
    frame: &mut Frame,
    area: Rect,
    panel_color: ratatui::style::Color,
    selection_color: ratatui::style::Color,
    rounded: bool,
) {
    if !rounded || area.width < 3 {
        frame.render_widget(
            Block::default().style(Style::default().bg(selection_color)),
            area,
        );
        return;
    }

    frame.render_widget(
        Block::default().style(Style::default().bg(selection_color)),
        Rect::new(area.x + 1, area.y, area.width - 2, area.height),
    );
    let cap_style = Style::default().fg(selection_color).bg(panel_color);
    for y in area.y..area.y + area.height {
        frame.render_widget(
            Paragraph::new("▐").style(cap_style),
            Rect::new(area.x, y, 1, 1),
        );
        frame.render_widget(
            Paragraph::new("▌").style(cap_style),
            Rect::new(area.x + area.width - 1, y, 1, 1),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        launcher_list_icon_area, launcher_visible_rows, list_areas, render_selection_background,
    };
    use crate::cli::{DesktopIconMode, Opts};
    use crate::ui::{HorizontalPosition, PanelPosition};
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
}
