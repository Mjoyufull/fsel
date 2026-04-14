use crate::cli::Opts;
use crate::common::Item;
use crate::ui::DmenuUI;

pub(super) fn show_line_numbers(cli: &Opts) -> bool {
    cli.cclip_show_line_numbers
        .or(Some(cli.dmenu_show_line_numbers))
        .unwrap_or(false)
}

pub(super) fn build_items(
    cclip_items: Vec<super::CclipItem>,
    formatter: &super::TagMetadataFormatter,
    show_line_numbers: bool,
    show_tag_color_names: bool,
) -> Vec<Item> {
    cclip_items
        .into_iter()
        .enumerate()
        .map(|(idx, cclip_item)| {
            let display_name = if show_line_numbers {
                cclip_item.get_display_name_with_number_formatter_options(
                    Some(formatter),
                    show_tag_color_names,
                )
            } else {
                cclip_item
                    .get_display_name_with_formatter_options(Some(formatter), show_tag_color_names)
            };

            let mut item =
                Item::new_simple(cclip_item.original_line.clone(), display_name, idx + 1);
            item.tags = Some(cclip_item.tags.clone());
            item
        })
        .collect()
}

pub(super) fn reload_and_restore(
    ui: &mut DmenuUI,
    updated_items: Vec<super::CclipItem>,
    tag_metadata_formatter: &super::TagMetadataFormatter,
    show_line_numbers: bool,
    show_tag_color_names: bool,
    visible_height: usize,
) {
    let old_selection = ui.selected;
    let selected_rowid = old_selection
        .and_then(|idx| ui.shown.get(idx))
        .and_then(item_rowid)
        .map(ToOwned::to_owned);

    let new_items = build_items(
        updated_items,
        tag_metadata_formatter,
        show_line_numbers,
        show_tag_color_names,
    );
    ui.set_items(new_items);

    if let Some(ref rowid) = selected_rowid {
        if let Some(position) = ui
            .shown
            .iter()
            .position(|item| item_rowid(item) == Some(rowid.as_str()))
        {
            ui.selected = Some(position);
        } else if let Some(old_idx) = old_selection {
            ui.selected = ui.shown.len().checked_sub(1).map(|last| old_idx.min(last));
        }
    } else if !ui.shown.is_empty() && ui.selected.is_none() {
        ui.selected = Some(0);
    }

    if let Some(position) = ui.selected {
        if position < ui.scroll_offset {
            ui.scroll_offset = position;
        } else if position >= ui.scroll_offset + visible_height {
            ui.scroll_offset = position + 1 - visible_height;
        }
    } else {
        ui.scroll_offset = 0;
    }

    let max_scroll = ui.shown.len().saturating_sub(visible_height);
    if ui.scroll_offset > max_scroll {
        ui.scroll_offset = max_scroll;
    }
}

pub(super) fn reload_visible_history(
    ui: &mut DmenuUI,
    cli: &Opts,
    tag_metadata_formatter: &super::TagMetadataFormatter,
    show_line_numbers: bool,
    show_tag_color_names: bool,
    max_visible: usize,
) {
    let updated_items = if let Some(ref tag_name) = cli.cclip_tag {
        super::scan::get_clipboard_history_by_tag(tag_name)
    } else {
        super::scan::get_clipboard_history()
    };

    if let Ok(updated_items) = updated_items {
        reload_and_restore(
            ui,
            updated_items,
            tag_metadata_formatter,
            show_line_numbers,
            show_tag_color_names,
            max_visible,
        );
    }
}

fn item_rowid(item: &Item) -> Option<&str> {
    item.original_line.split('\t').next()
}

#[cfg(test)]
mod tests {
    use crate::common::Item;
    use crate::ui::DmenuUI;

    use super::reload_and_restore;

    fn cclip_item(rowid: &str, preview: &str) -> crate::modes::cclip::CclipItem {
        crate::modes::cclip::CclipItem::from_line(format!("{rowid}\ttext/plain\t{preview}\ttag"))
            .expect("valid cclip item")
    }

    #[test]
    fn reload_and_restore_keeps_matching_rowid_selected() {
        let formatter =
            crate::modes::cclip::TagMetadataFormatter::new(std::collections::HashMap::new());
        let mut ui = DmenuUI::new(
            vec![
                Item::new_simple("1\ttext/plain\tone".into(), "one".into(), 1),
                Item::new_simple("2\ttext/plain\ttwo".into(), "two".into(), 2),
            ],
            true,
            false,
        );
        ui.filter();
        ui.selected = Some(1);

        reload_and_restore(
            &mut ui,
            vec![cclip_item("2", "two"), cclip_item("3", "three")],
            &formatter,
            false,
            false,
            5,
        );

        assert_eq!(ui.selected, Some(0));
        assert_eq!(ui.shown[0].original_line.split('\t').next(), Some("2"));
    }
}
