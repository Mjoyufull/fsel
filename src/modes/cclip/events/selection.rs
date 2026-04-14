use super::super::items::reload_visible_history;
use super::EventContext;
use eyre::{Result, WrapErr};

pub(super) fn delete_selected_item(ctx: &mut EventContext<'_, '_>) -> Result<()> {
    if let Some(selected) = ctx.ui.selected
        && selected < ctx.ui.shown.len()
    {
        let item = &ctx.ui.shown[selected];
        if let Some(rowid) = ctx.ui.get_cclip_rowid(item) {
            match super::super::select::delete_item(&rowid) {
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

pub(super) fn copy_selected_and_exit(ctx: &mut EventContext<'_, '_>) -> Result<bool> {
    let Some(selected) = ctx.ui.selected else {
        return Ok(false);
    };
    copy_selected_and_exit_at(ctx, selected)
}

pub(super) fn copy_selected_and_exit_at(
    ctx: &mut EventContext<'_, '_>,
    index: usize,
) -> Result<bool> {
    if index >= ctx.ui.shown.len() {
        return Ok(false);
    }

    let original_line = &ctx.ui.shown[index].original_line;
    match super::super::CclipItem::from_line(original_line.clone()) {
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

pub(super) fn move_to_first(ui: &mut crate::ui::DmenuUI<'_>) {
    if !ui.shown.is_empty() {
        ui.selected = Some(0);
        ui.scroll_offset = 0;
    }
}

pub(super) fn move_to_last(ui: &mut crate::ui::DmenuUI<'_>, max_visible: usize) {
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

pub(super) fn keep_selection_visible(ui: &mut crate::ui::DmenuUI<'_>, max_visible: usize) {
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

#[cfg(test)]
mod tests {
    use super::{keep_selection_visible, move_to_first, move_to_last};
    use crate::common::Item;
    use crate::ui::DmenuUI;

    fn test_ui(count: usize) -> DmenuUI<'static> {
        let items = (0..count)
            .map(|index| {
                Item::new_simple(
                    format!("line-{index}"),
                    format!("Item {index}"),
                    index.saturating_add(1),
                )
            })
            .collect();
        let mut ui = DmenuUI::new(items, false, false);
        ui.filter();
        ui
    }

    #[test]
    fn move_to_last_scrolls_tail_into_view() {
        let mut ui = test_ui(10);

        move_to_last(&mut ui, 4);

        assert_eq!(ui.selected, Some(9));
        assert_eq!(ui.scroll_offset, 6);
    }

    #[test]
    fn move_to_first_resets_selection_and_scroll() {
        let mut ui = test_ui(5);
        ui.selected = Some(4);
        ui.scroll_offset = 3;

        move_to_first(&mut ui);

        assert_eq!(ui.selected, Some(0));
        assert_eq!(ui.scroll_offset, 0);
    }

    #[test]
    fn keep_selection_visible_scrolls_when_selection_moves_below_window() {
        let mut ui = test_ui(10);
        ui.selected = Some(7);
        ui.scroll_offset = 2;

        keep_selection_visible(&mut ui, 4);

        assert_eq!(ui.scroll_offset, 4);
    }
}
