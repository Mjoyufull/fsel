use super::image::ImageRuntime;
use super::items::reload_visible_history;
use super::state::CclipOptions;
use super::tags;
use crate::cli::Opts;
use crate::ui::{AsyncInput, DmenuUI, InputEvent as Event, TagMode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use eyre::{Result, WrapErr};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::collections::HashMap;
use std::io;

use super::{TagMetadata, TagMetadataFormatter};

pub(super) enum LoopControl {
    Continue,
    Exit,
}

pub(super) struct EventOutcome {
    pub(super) control: LoopControl,
    pub(super) needs_redraw: bool,
}

pub(super) struct EventContext<'a, 'ui> {
    pub(super) ui: &'a mut DmenuUI<'ui>,
    pub(super) terminal: &'a mut Terminal<CrosstermBackend<io::Stderr>>,
    pub(super) cli: &'a Opts,
    pub(super) options: &'a CclipOptions,
    pub(super) db: &'a std::sync::Arc<redb::Database>,
    pub(super) tag_metadata_map: &'a mut HashMap<String, TagMetadata>,
    pub(super) tag_metadata_formatter: &'a mut TagMetadataFormatter,
    pub(super) image_runtime: &'a mut ImageRuntime,
    pub(super) max_visible: usize,
}

pub(super) async fn handle_event(
    mut ctx: EventContext<'_, '_>,
    event: Event<KeyEvent>,
    input: &mut AsyncInput,
) -> Result<EventOutcome> {
    match event {
        Event::Input(key) => handle_key_event(&mut ctx, input, key).await,
        Event::Mouse(mouse_event) => handle_mouse_event(&mut ctx, mouse_event),
        Event::Tick => Ok(EventOutcome {
            control: LoopControl::Continue,
            needs_redraw: ctx.ui.temp_message.is_some(),
        }),
        Event::Render => Ok(EventOutcome {
            control: LoopControl::Continue,
            needs_redraw: true,
        }),
    }
}

async fn handle_key_event(
    ctx: &mut EventContext<'_, '_>,
    input: &mut AsyncInput,
    key: KeyEvent,
) -> Result<EventOutcome> {
    let needs_redraw = true;

    match (key.code, key.modifiers) {
        (code, mods) if ctx.cli.keybinds.matches_image_preview(code, mods) => {
            ctx.image_runtime
                .show_fullscreen_preview(ctx.terminal, input)
                .await?;
        }
        (code, mods) if ctx.cli.keybinds.matches_tag(code, mods) => {
            tags::begin_tag_creation(ctx.ui, ctx.image_runtime, ctx.terminal)?;
        }
        (KeyCode::Char('t'), KeyModifiers::ALT) => {
            tags::begin_tag_removal(ctx.ui);
        }
        (code, mods) if ctx.cli.keybinds.matches_cclip_delete(code, mods) => {
            if matches!(ctx.ui.tag_mode, TagMode::Normal) {
                delete_selected_item(ctx)?;
            }
        }
        (KeyCode::Esc, _)
        | (KeyCode::Char('q'), KeyModifiers::CONTROL)
        | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            if ctx.ui.tag_mode != TagMode::Normal {
                ctx.ui.tag_mode = TagMode::Normal;
            } else {
                return Ok(EventOutcome {
                    control: LoopControl::Exit,
                    needs_redraw,
                });
            }
        }
        (KeyCode::Enter, _) | (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
            if matches!(ctx.ui.tag_mode, TagMode::Normal) {
                if copy_selected_and_exit(ctx)? {
                    return Ok(EventOutcome {
                        control: LoopControl::Exit,
                        needs_redraw,
                    });
                }
            } else {
                tags::submit_tag_mode(tags::TagSubmitContext {
                    ui: ctx.ui,
                    cli: ctx.cli,
                    db: ctx.db,
                    tag_metadata_map: ctx.tag_metadata_map,
                    tag_metadata_formatter: ctx.tag_metadata_formatter,
                    show_line_numbers: ctx.options.show_line_numbers,
                    show_tag_color_names: ctx.options.show_tag_color_names,
                    max_visible: ctx.max_visible,
                });
            }
        }
        (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
            push_char(ctx.ui, c);
        }
        (KeyCode::Backspace, _) => {
            pop_char(ctx.ui);
        }
        (KeyCode::Left, _) => {
            if matches!(ctx.ui.tag_mode, TagMode::Normal) {
                move_to_first(ctx.ui);
            }
        }
        (KeyCode::Right, _) => {
            if matches!(ctx.ui.tag_mode, TagMode::Normal) {
                move_to_last(
                    ctx.ui,
                    ctx.options.max_visible_items(ctx.terminal.size()?.height),
                );
            }
        }
        (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
            handle_down(ctx)?;
        }
        (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
            handle_up(ctx)?;
        }
        _ => {}
    }

    Ok(EventOutcome {
        control: LoopControl::Continue,
        needs_redraw,
    })
}

fn handle_mouse_event(
    ctx: &mut EventContext<'_, '_>,
    mouse_event: MouseEvent,
) -> Result<EventOutcome> {
    let mouse_row = mouse_event.row;
    let (items_panel_start, items_panel_height) =
        ctx.options.items_panel_bounds(ctx.terminal.size()?.height);
    let items_content_start = items_panel_start + 1;
    let max_visible_rows = items_panel_height.saturating_sub(2);
    let items_content_end = items_content_start + max_visible_rows;

    let update_selection_for_mouse_pos = |ui: &mut DmenuUI<'_>| {
        if !ui.shown.is_empty() && mouse_row >= items_content_start && mouse_row < items_content_end
        {
            let row_in_content = mouse_row - items_content_start;
            let hovered_item_index = ui.scroll_offset + row_in_content as usize;
            if hovered_item_index < ui.shown.len() {
                ui.selected = Some(hovered_item_index);
            }
        }
    };

    match mouse_event.kind {
        MouseEventKind::Moved => {
            if matches!(ctx.ui.tag_mode, TagMode::Normal) {
                update_selection_for_mouse_pos(ctx.ui);
            }
        }
        MouseEventKind::Down(MouseButton::Left) => {
            if !matches!(ctx.ui.tag_mode, TagMode::Normal) {
                return Ok(EventOutcome {
                    control: LoopControl::Continue,
                    needs_redraw: true,
                });
            }

            if mouse_row >= items_content_start
                && mouse_row < items_content_end
                && !ctx.ui.shown.is_empty()
            {
                let row_in_content = mouse_row - items_content_start;
                let clicked_item_index = ctx.ui.scroll_offset + row_in_content as usize;
                if clicked_item_index < ctx.ui.shown.len()
                    && copy_selected_and_exit_at(ctx, clicked_item_index)?
                {
                    return Ok(EventOutcome {
                        control: LoopControl::Exit,
                        needs_redraw: true,
                    });
                }
            }
        }
        MouseEventKind::ScrollUp => {
            if matches!(ctx.ui.tag_mode, TagMode::Normal)
                && !ctx.ui.shown.is_empty()
                && ctx.ui.scroll_offset > 0
            {
                ctx.ui.scroll_offset -= 1;
                update_selection_for_mouse_pos(ctx.ui);
            }
        }
        MouseEventKind::ScrollDown => {
            if matches!(ctx.ui.tag_mode, TagMode::Normal) && !ctx.ui.shown.is_empty() {
                let max_visible = max_visible_rows as usize;
                if ctx.ui.scroll_offset + max_visible < ctx.ui.shown.len() {
                    ctx.ui.scroll_offset += 1;
                    update_selection_for_mouse_pos(ctx.ui);
                }
            }
        }
        _ => {}
    }

    Ok(EventOutcome {
        control: LoopControl::Continue,
        needs_redraw: true,
    })
}

fn delete_selected_item(ctx: &mut EventContext<'_, '_>) -> Result<()> {
    if let Some(selected) = ctx.ui.selected
        && selected < ctx.ui.shown.len()
    {
        let item = &ctx.ui.shown[selected];
        if let Some(rowid) = ctx.ui.get_cclip_rowid(item) {
            match super::select::delete_item(&rowid) {
                Ok(()) => {
                    ctx.ui.set_temp_message(format!("Deleted entry {}", rowid));
                    reload_visible_history(
                        ctx.ui,
                        ctx.cli,
                        ctx.tag_metadata_formatter,
                        ctx.options.show_line_numbers,
                        ctx.options.show_tag_color_names,
                        ctx.max_visible,
                    );
                }
                Err(error) => {
                    ctx.ui
                        .set_temp_message(format!("Failed to delete entry {}: {}", rowid, error));
                }
            }
        }
    }

    Ok(())
}

fn copy_selected_and_exit(ctx: &mut EventContext<'_, '_>) -> Result<bool> {
    let Some(selected) = ctx.ui.selected else {
        return Ok(false);
    };
    copy_selected_and_exit_at(ctx, selected)
}

fn copy_selected_and_exit_at(ctx: &mut EventContext<'_, '_>, index: usize) -> Result<bool> {
    if index >= ctx.ui.shown.len() {
        return Ok(false);
    }

    let original_line = &ctx.ui.shown[index].original_line;
    match super::CclipItem::from_line(original_line.clone()) {
        Ok(cclip_item) => {
            if let Err(error) = cclip_item.copy_to_clipboard() {
                ctx.ui.set_temp_message(format!("Copy failed: {}", error));
                return Ok(false);
            }

            ctx.terminal
                .show_cursor()
                .wrap_err("Failed to show cursor")?;
            let _ = crate::ui::terminal::shutdown_terminal(ctx.options.disable_mouse);
            Ok(true)
        }
        Err(error) => {
            ctx.ui.set_temp_message(format!("Parse failed: {}", error));
            Ok(false)
        }
    }
}

fn push_char(ui: &mut DmenuUI<'_>, character: char) {
    match &mut ui.tag_mode {
        TagMode::PromptingTagName { input, .. }
        | TagMode::PromptingTagColor { input, .. }
        | TagMode::PromptingTagEmoji { input, .. }
        | TagMode::RemovingTag { input, .. } => input.push(character),
        TagMode::Normal => {
            ui.query.push(character);
            ui.filter();
        }
    }
}

fn pop_char(ui: &mut DmenuUI<'_>) {
    match &mut ui.tag_mode {
        TagMode::PromptingTagName { input, .. }
        | TagMode::PromptingTagColor { input, .. }
        | TagMode::PromptingTagEmoji { input, .. }
        | TagMode::RemovingTag { input, .. } => {
            input.pop();
        }
        TagMode::Normal => {
            ui.query.pop();
            ui.filter();
        }
    }
}

fn move_to_first(ui: &mut DmenuUI<'_>) {
    if !ui.shown.is_empty() {
        ui.selected = Some(0);
        ui.scroll_offset = 0;
    }
}

fn move_to_last(ui: &mut DmenuUI<'_>, max_visible: usize) {
    if ui.shown.is_empty() {
        return;
    }

    let last_index = ui.shown.len() - 1;
    ui.selected = Some(last_index);
    if max_visible > 0 && ui.shown.len() > max_visible {
        ui.scroll_offset = ui.shown.len().saturating_sub(max_visible);
    } else {
        ui.scroll_offset = 0;
    }
}

fn handle_down(ctx: &mut EventContext<'_, '_>) -> Result<()> {
    match &ctx.ui.tag_mode {
        TagMode::PromptingTagName { .. } => {
            ctx.ui.cycle_tag_creation_selection(1);
            return Ok(());
        }
        TagMode::RemovingTag { .. } => {
            ctx.ui.cycle_removal_selection(1);
            return Ok(());
        }
        TagMode::PromptingTagEmoji { .. } | TagMode::PromptingTagColor { .. } => {
            return Ok(());
        }
        TagMode::Normal => {}
    }

    if let Some(selected) = ctx.ui.selected {
        ctx.ui.selected = if ctx.ui.shown.is_empty() {
            Some(selected)
        } else if selected + 1 < ctx.ui.shown.len() {
            Some(selected + 1)
        } else if !ctx.options.hard_stop {
            Some(0)
        } else {
            Some(selected)
        };
        keep_selection_visible(
            ctx.ui,
            ctx.options.max_visible_items(ctx.terminal.size()?.height),
        );
    }

    Ok(())
}

fn handle_up(ctx: &mut EventContext<'_, '_>) -> Result<()> {
    match &ctx.ui.tag_mode {
        TagMode::PromptingTagName { .. } => {
            ctx.ui.cycle_tag_creation_selection(-1);
            return Ok(());
        }
        TagMode::RemovingTag { .. } => {
            ctx.ui.cycle_removal_selection(-1);
            return Ok(());
        }
        TagMode::PromptingTagEmoji { .. } | TagMode::PromptingTagColor { .. } => {
            return Ok(());
        }
        TagMode::Normal => {}
    }

    if let Some(selected) = ctx.ui.selected {
        ctx.ui.selected = if selected > 0 {
            Some(selected - 1)
        } else if !ctx.options.hard_stop && !ctx.ui.shown.is_empty() {
            Some(ctx.ui.shown.len() - 1)
        } else {
            Some(selected)
        };
        keep_selection_visible(
            ctx.ui,
            ctx.options.max_visible_items(ctx.terminal.size()?.height),
        );
    }

    Ok(())
}

fn keep_selection_visible(ui: &mut DmenuUI<'_>, max_visible: usize) {
    if let Some(new_selected) = ui.selected {
        if max_visible == 0 {
            ui.scroll_offset = 0;
        } else if new_selected >= ui.scroll_offset + max_visible {
            ui.scroll_offset = new_selected.saturating_sub(max_visible - 1);
        } else if new_selected < ui.scroll_offset {
            ui.scroll_offset = new_selected;
        }
    }
}
