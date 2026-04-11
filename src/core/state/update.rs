use super::{Message, State};

/// Update function - pure state transition.
pub fn update(state: &mut State, msg: Message, hard_stop: bool, max_visible: usize) {
    match msg {
        Message::SelectIndex(index) => select_index(state, index),
        Message::CharInput(character) => update_query(state, Some(character), "User typed"),
        Message::Backspace => update_query(state, None, "User pressed backspace"),
        Message::MoveUp => move_up(state, hard_stop, max_visible),
        Message::MoveDown => move_down(state, hard_stop, max_visible),
        Message::MoveFirst => move_first(state),
        Message::MoveLast => move_last(state, max_visible),
        Message::Select => state.should_launch = true,
        Message::Exit => {
            state.selected = None;
            state.should_exit = true;
        }
        Message::Tick => {}
    }
}

fn select_index(state: &mut State, index: usize) {
    if index < state.shown.len() {
        let app_name = state.shown.get(index).map(|app| app.name.as_str());
        state.selected = Some(index);
        if crate::cli::DEBUG_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            crate::core::debug_logger::log_selection_change(
                state.selected,
                app_name,
                state.scroll_offset,
            );
        }
    }
}

fn update_query(state: &mut State, character: Option<char>, reason: &str) {
    let old_query = state.query.clone();
    match character {
        Some(character) => state.query.push(character),
        None => {
            state.query.pop();
        }
    }

    if crate::cli::DEBUG_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        let message = match character {
            Some(character) => format!("{reason} '{}'", character),
            None => reason.to_string(),
        };
        crate::core::debug_logger::log_query_change(&old_query, &state.query, &message);
    }

    state.filter();
}

fn move_up(state: &mut State, hard_stop: bool, max_visible: usize) {
    if let Some(selected) = state.selected {
        state.selected = if selected > 0 {
            Some(selected - 1)
        } else if !hard_stop && !state.shown.is_empty() {
            Some(state.shown.len() - 1)
        } else {
            Some(selected)
        };
        keep_selection_visible(state, max_visible);
    }
}

fn move_down(state: &mut State, hard_stop: bool, max_visible: usize) {
    if let Some(selected) = state.selected {
        state.selected = if selected < state.shown.len().saturating_sub(1) {
            Some(selected + 1)
        } else if !hard_stop {
            Some(0)
        } else {
            Some(selected)
        };
        keep_selection_visible(state, max_visible);
    }
}

fn move_first(state: &mut State) {
    if !state.shown.is_empty() {
        state.selected = Some(0);
        state.scroll_offset = 0;
    }
}

fn move_last(state: &mut State, max_visible: usize) {
    if !state.shown.is_empty() {
        let last = state.shown.len() - 1;
        state.selected = Some(last);
        if max_visible > 0 && last >= max_visible {
            state.scroll_offset = last - max_visible + 1;
        }
    }
}

fn keep_selection_visible(state: &mut State, max_visible: usize) {
    if let Some(selected) = state.selected {
        if selected < state.scroll_offset {
            state.scroll_offset = selected;
        } else if max_visible > 0 && selected >= state.scroll_offset + max_visible {
            state.scroll_offset = selected.saturating_sub(max_visible - 1);
        }
    }
}
