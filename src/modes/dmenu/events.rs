use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::ui::DmenuUI;

use super::options::DmenuOptions;

pub(super) enum LoopOutcome {
    Continue,
    Exit,
    Print(String),
}

pub(super) fn handle_key_event(
    ui: &mut DmenuUI,
    key: KeyEvent,
    options: &DmenuOptions,
    terminal_height: u16,
) -> LoopOutcome {
    match (key.code, key.modifiers) {
        (KeyCode::Esc, _)
        | (KeyCode::Char('q'), KeyModifiers::CONTROL)
        | (KeyCode::Char('c'), KeyModifiers::CONTROL) => return LoopOutcome::Exit,
        (KeyCode::Enter, _) | (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
            return handle_submit(ui, options);
        }
        (KeyCode::Char(ch), KeyModifiers::NONE) | (KeyCode::Char(ch), KeyModifiers::SHIFT) => {
            ui.query.push(ch);
            ui.filter();
            auto_select_if_single_match(ui, options);
        }
        (KeyCode::Backspace, _) => {
            ui.query.pop();
            ui.filter();
            auto_select_if_single_match(ui, options);
        }
        (KeyCode::Left, _) => {
            if !ui.shown.is_empty() {
                ui.selected = Some(0);
                ui.scroll_offset = 0;
            }
        }
        (KeyCode::Right, _) => {
            if !ui.shown.is_empty() {
                let last_index = ui.shown.len() - 1;
                ui.selected = Some(last_index);

                let max_visible = options.max_visible_items(terminal_height);
                if max_visible > 0 && ui.shown.len() > max_visible {
                    ui.scroll_offset = ui.shown.len().saturating_sub(max_visible);
                } else {
                    ui.scroll_offset = 0;
                }
            }
        }
        (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
            move_selection(ui, options, terminal_height, 1);
        }
        (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
            move_selection(ui, options, terminal_height, -1);
        }
        _ => {}
    }

    ui.info(options.highlight_color);
    LoopOutcome::Continue
}

pub(super) fn handle_mouse_event(
    ui: &mut DmenuUI,
    mouse_event: MouseEvent,
    options: &DmenuOptions,
    terminal_height: u16,
) -> LoopOutcome {
    let mouse_row = mouse_event.row;
    let (items_panel_start, items_panel_height) = options.items_panel_bounds(terminal_height);
    let items_content_start = items_panel_start + 1;
    let max_visible_rows = items_panel_height.saturating_sub(2);
    let items_content_end = items_content_start + max_visible_rows;

    let update_selection_for_mouse_pos = |ui: &mut DmenuUI, mouse_row: u16| {
        if !ui.shown.is_empty() && mouse_row >= items_content_start && mouse_row < items_content_end
        {
            let row_in_content = mouse_row - items_content_start;
            let hovered_item_index = ui.scroll_offset + row_in_content as usize;
            if hovered_item_index < ui.shown.len() {
                ui.selected = Some(hovered_item_index);
                ui.info(options.highlight_color);
            }
        }
    };

    match mouse_event.kind {
        MouseEventKind::Moved => {
            update_selection_for_mouse_pos(ui, mouse_row);
        }
        MouseEventKind::Down(MouseButton::Left) => {
            if mouse_row >= items_content_start
                && mouse_row < items_content_end
                && !ui.shown.is_empty()
            {
                let row_in_content = mouse_row - items_content_start;
                let clicked_item_index = ui.scroll_offset + row_in_content as usize;

                if clicked_item_index < ui.shown.len() {
                    return LoopOutcome::Print(ui.shown[clicked_item_index].original_line.clone());
                }
            }
        }
        MouseEventKind::ScrollUp => {
            if !ui.shown.is_empty() && ui.scroll_offset > 0 {
                ui.scroll_offset -= 1;
                update_selection_for_mouse_pos(ui, mouse_row);
            }
        }
        MouseEventKind::ScrollDown => {
            if !ui.shown.is_empty() {
                let max_visible = max_visible_rows as usize;
                if ui.scroll_offset + max_visible < ui.shown.len() {
                    ui.scroll_offset += 1;
                    update_selection_for_mouse_pos(ui, mouse_row);
                }
            }
        }
        _ => {}
    }

    LoopOutcome::Continue
}

fn handle_submit(ui: &mut DmenuUI, options: &DmenuOptions) -> LoopOutcome {
    auto_select_if_single_match(ui, options);

    if let Some(selected) = ui.selected
        && selected < ui.shown.len()
    {
        return LoopOutcome::Print(selected_output(ui, options, selected));
    }

    if !options.only_match && !ui.query.is_empty() {
        return LoopOutcome::Print(ui.query.clone());
    }

    if options.only_match {
        LoopOutcome::Continue
    } else {
        LoopOutcome::Exit
    }
}

fn selected_output(ui: &DmenuUI, options: &DmenuOptions, selected: usize) -> String {
    if options.index_mode {
        selected.to_string()
    } else if let Some(ref accept_cols) = options.accept_nth {
        ui.shown[selected].get_accept_nth_output(accept_cols)
    } else {
        ui.shown[selected].original_line.clone()
    }
}

fn auto_select_if_single_match(ui: &mut DmenuUI, options: &DmenuOptions) {
    if options.auto_select && ui.shown.len() == 1 {
        ui.selected = Some(0);
    }
}

fn move_selection(ui: &mut DmenuUI, options: &DmenuOptions, terminal_height: u16, direction: i32) {
    let Some(selected) = ui.selected else {
        return;
    };

    let Some(last_index) = ui.shown.len().checked_sub(1) else {
        return;
    };

    ui.selected = if direction > 0 {
        if selected < last_index {
            Some(selected + 1)
        } else if !options.hard_stop {
            Some(0)
        } else {
            Some(selected)
        }
    } else if selected > 0 {
        Some(selected - 1)
    } else if !options.hard_stop {
        Some(last_index)
    } else {
        Some(selected)
    };

    let Some(new_selected) = ui.selected else {
        return;
    };

    let max_visible = options.max_visible_items(terminal_height);
    if max_visible == 0 {
        ui.scroll_offset = 0;
    } else if new_selected < ui.scroll_offset {
        ui.scroll_offset = new_selected;
    } else if new_selected >= ui.scroll_offset + max_visible {
        ui.scroll_offset = new_selected.saturating_sub(max_visible - 1);
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::Opts;
    use crate::common::Item;
    use crate::ui::DmenuUI;

    use super::{DmenuOptions, LoopOutcome, handle_key_event};

    #[test]
    fn submit_returns_query_when_no_selection_and_only_match_is_disabled() {
        let mut ui = DmenuUI::new(
            vec![Item::new_simple("a".into(), "a".into(), 1)],
            false,
            false,
        );
        ui.query = "typed".to_string();
        ui.filter();
        ui.selected = None;

        let outcome = handle_key_event(
            &mut ui,
            crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Enter,
                crossterm::event::KeyModifiers::NONE,
            ),
            &DmenuOptions::from_cli(&Opts::default()),
            20,
        );

        assert!(matches!(outcome, LoopOutcome::Print(output) if output == "typed"));
    }

    #[test]
    fn submit_uses_accept_nth_output_when_requested() {
        let mut ui = DmenuUI::new(
            vec![Item::new_simple(
                "left:right".into(),
                "left:right".into(),
                1,
            )],
            false,
            false,
        );
        ui.filter();

        let cli = Opts {
            dmenu_accept_nth: Some(vec![1]),
            ..Opts::default()
        };

        let outcome = handle_key_event(
            &mut ui,
            crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Enter,
                crossterm::event::KeyModifiers::NONE,
            ),
            &DmenuOptions::from_cli(&cli),
            20,
        );

        assert!(matches!(outcome, LoopOutcome::Print(output) if output == "left:right"));
    }
}
