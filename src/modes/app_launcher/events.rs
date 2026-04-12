use crate::cli::Opts;
use crate::core::ranking::current_unix_seconds;
use crate::core::state::{Message, State};
use crate::ui::InputEvent as Event;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

pub(crate) fn handle_event(
    state: &mut State,
    event: Event<crossterm::event::KeyEvent>,
    cli: &Opts,
    db: &std::sync::Arc<redb::Database>,
    total_height: u16,
) {
    match event {
        Event::Input(key) => handle_key_event(state, key, cli, db, total_height),
        Event::Mouse(mouse_event) => handle_mouse_event(state, mouse_event, cli, total_height),
        Event::Tick | Event::Render => {}
    }
}

fn handle_key_event(
    state: &mut State,
    key: KeyEvent,
    cli: &Opts,
    db: &std::sync::Arc<redb::Database>,
    total_height: u16,
) {
    let max_visible = max_visible_items(total_height, cli);

    let msg = if cli.keybinds.matches_exit(key.code, key.modifiers) {
        Message::Exit
    } else if cli.keybinds.matches_select(key.code, key.modifiers) {
        Message::Select
    } else if cli.keybinds.matches_up(key.code, key.modifiers) {
        Message::MoveUp
    } else if cli.keybinds.matches_down(key.code, key.modifiers) {
        Message::MoveDown
    } else if cli.keybinds.matches_left(key.code, key.modifiers) {
        Message::MoveFirst
    } else if cli.keybinds.matches_right(key.code, key.modifiers) {
        Message::MoveLast
    } else if cli.keybinds.matches_backspace(key.code, key.modifiers) {
        Message::Backspace
    } else if cli.keybinds.matches_pin(key.code, key.modifiers) {
        toggle_selected_pin(state, db);
        refresh_info(state, cli);
        return;
    } else {
        match key.code {
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                Message::CharInput(c)
            }
            KeyCode::Home => Message::MoveFirst,
            KeyCode::End => Message::MoveLast,
            KeyCode::Tab => Message::MoveDown,
            KeyCode::BackTab => Message::MoveUp,
            _ => Message::Tick,
        }
    };

    if key.modifiers.contains(KeyModifiers::CONTROL)
        && key.code == KeyCode::Char('c')
        && !cli.keybinds.matches_exit(key.code, key.modifiers)
    {
        state.should_exit = true;
    }

    crate::core::state::update(state, msg, cli.hard_stop, max_visible);
    refresh_info(state, cli);
}

fn handle_mouse_event(state: &mut State, mouse_event: MouseEvent, cli: &Opts, total_height: u16) {
    let metrics = list_metrics(total_height, cli);

    let msg = match mouse_event.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(index) = metrics.app_index_for_row(mouse_event.row, state) {
                crate::core::state::update(
                    state,
                    Message::SelectIndex(index),
                    cli.hard_stop,
                    metrics.max_visible,
                );
                Message::Select
            } else {
                Message::Tick
            }
        }
        MouseEventKind::Moved => metrics
            .app_index_for_row(mouse_event.row, state)
            .map(Message::SelectIndex)
            .unwrap_or(Message::Tick),
        MouseEventKind::ScrollDown => {
            if metrics.contains_row(mouse_event.row)
                && !state.shown.is_empty()
                && state.scroll_offset + metrics.max_visible < state.shown.len()
            {
                state.scroll_offset += 1;
                metrics.snap_selection_to_mouse(state, mouse_event.row);
                refresh_info(state, cli);
            }
            Message::Tick
        }
        MouseEventKind::ScrollUp => {
            if metrics.contains_row(mouse_event.row)
                && !state.shown.is_empty()
                && state.scroll_offset > 0
            {
                state.scroll_offset -= 1;
                metrics.snap_selection_to_mouse(state, mouse_event.row);
                refresh_info(state, cli);
            }
            Message::Tick
        }
        _ => Message::Tick,
    };

    if !matches!(msg, Message::Tick) {
        if crate::cli::DEBUG_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            crate::core::debug_logger::log_event(&format!("State update via Mouse: {:?}", msg));
        }

        crate::core::state::update(state, msg, cli.hard_stop, metrics.max_visible);
        refresh_info(state, cli);
    }
}

fn toggle_selected_pin(state: &mut State, db: &std::sync::Arc<redb::Database>) {
    let Some(index) = state.selected else {
        return;
    };
    let Some(app) = state.shown.get(index).cloned() else {
        return;
    };

    let Ok(is_pinned) = crate::core::database::toggle_pin(db, &app.name) else {
        return;
    };

    for entry in &mut state.apps {
        if entry.name == app.name {
            entry.pinned = is_pinned;
        }
    }

    let frecency_data = crate::core::database::load_frecency(db);
    state.pin_timestamps = crate::core::database::load_pin_timestamps(db);
    crate::core::ranking::sort_by_ranking(
        &mut state.apps,
        &frecency_data,
        state.ranking_mode,
        state.pinned_order_mode,
        &state.pin_timestamps,
        current_unix_seconds(),
    );
    state.filter();
}

fn refresh_info(state: &mut State, cli: &Opts) {
    state.update_info(
        cli.highlight_color,
        cli.fancy_mode,
        cli.verbose.unwrap_or(0),
    );
}

fn max_visible_items(total_height: u16, cli: &Opts) -> usize {
    let title_height =
        crate::ui::effective_title_height(total_height, cli.title_panel_height_percent);
    let input_height = cli.input_panel_height;
    let apps_panel_height = total_height.saturating_sub(title_height + input_height);
    apps_panel_height.saturating_sub(2) as usize
}

fn list_metrics(total_height: u16, cli: &Opts) -> ListMetrics {
    let title_height =
        crate::ui::effective_title_height(total_height, cli.title_panel_height_percent);
    let input_height = cli.input_panel_height;
    let title_panel_position = cli
        .title_panel_position
        .unwrap_or(crate::ui::PanelPosition::Top);

    let (apps_panel_start, apps_panel_height) = match title_panel_position {
        crate::ui::PanelPosition::Top => (
            title_height,
            total_height.saturating_sub(title_height + input_height),
        ),
        crate::ui::PanelPosition::Middle | crate::ui::PanelPosition::Bottom => {
            (0, total_height.saturating_sub(title_height + input_height))
        }
    };

    ListMetrics {
        list_content_start: apps_panel_start + 1,
        max_visible: apps_panel_height.saturating_sub(2) as usize,
    }
}

struct ListMetrics {
    list_content_start: u16,
    max_visible: usize,
}

impl ListMetrics {
    fn contains_row(&self, row: u16) -> bool {
        row >= self.list_content_start && row < self.list_content_start + self.max_visible as u16
    }

    fn app_index_for_row(&self, row: u16, state: &State) -> Option<usize> {
        if !self.contains_row(row) {
            return None;
        }

        let row_in_content = row - self.list_content_start;
        let index = state.scroll_offset + row_in_content as usize;
        (index < state.shown.len()).then_some(index)
    }

    fn snap_selection_to_mouse(&self, state: &mut State, row: u16) {
        let row_in_content = row.saturating_sub(self.list_content_start);
        let index = state.scroll_offset + row_in_content as usize;
        if index < state.shown.len() {
            state.selected = Some(index);
        }
    }
}
