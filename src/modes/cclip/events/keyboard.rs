use super::selection::{
    copy_selected_and_exit, delete_selected_item, keep_selection_visible, move_to_first,
    move_to_last,
};
use super::{EventContext, EventOutcome, LoopControl};
use crate::ui::{AsyncInput, TagMode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use eyre::Result;

pub(super) async fn handle_key_event(
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
            super::super::tags::begin_tag_creation(ctx.ui, ctx.image_runtime, ctx.terminal)?;
        }
        (KeyCode::Char('t'), KeyModifiers::ALT) => {
            super::super::tags::begin_tag_removal(ctx.ui);
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
                super::super::tags::submit_tag_mode(super::super::tags::TagSubmitContext {
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

fn push_char(ui: &mut crate::ui::DmenuUI<'_>, character: char) {
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

fn pop_char(ui: &mut crate::ui::DmenuUI<'_>) {
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
