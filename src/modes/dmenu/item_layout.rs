//! Dmenu item rectangles shared by rendering, scrolling, and mouse selection.

use crate::common::Item;
use crate::ui::DmenuUI;
use ratatui::layout::{Position, Rect};

pub(super) struct ItemLayout {
    slots: Vec<Rect>,
}

impl ItemLayout {
    pub(super) fn new(
        area: Rect,
        ui: &mut DmenuUI<'_>,
        horizontal: bool,
        reverse: bool,
        width: u16,
    ) -> Self {
        let limit = if horizontal {
            area.width / width.max(1)
        } else {
            area.height
        };
        if area.is_empty() || limit == 0 {
            return Self { slots: Vec::new() };
        }
        let extent = |item: &Item| {
            if horizontal {
                1
            } else {
                u16::try_from(item.display_text.lines().count().max(1))
                    .unwrap_or(u16::MAX)
                    .min(limit)
            }
        };
        if let Some(selected) = ui.selected.filter(|index| *index < ui.shown.len()) {
            ui.scroll_offset = ui.scroll_offset.min(selected);
            let mut start = selected;
            let mut used = extent(&ui.shown[selected]);
            while start > ui.scroll_offset {
                let previous = extent(&ui.shown[start - 1]);
                if previous > limit.saturating_sub(used) {
                    break;
                }
                used += previous;
                start -= 1;
            }
            ui.scroll_offset = start;
        }
        let mut slots = Vec::new();
        let mut used = 0u16;
        for item in ui.shown.iter().skip(ui.scroll_offset) {
            let size = extent(item);
            if size > limit.saturating_sub(used) {
                break;
            }
            let physical = if reverse { limit - used - size } else { used };
            let height =
                u16::try_from(item.display_text.lines().count().max(1)).unwrap_or(u16::MAX);
            slots.push(if horizontal {
                Rect::new(
                    area.x + physical * width,
                    area.y,
                    width,
                    height.min(area.height),
                )
            } else {
                Rect::new(area.x, area.y + physical, area.width, size)
            });
            used += size;
        }
        Self { slots }
    }

    pub(super) fn capacity(&self) -> usize {
        self.slots.len()
    }

    pub(super) fn slot(&self, index: usize) -> Rect {
        self.slots.get(index).copied().unwrap_or_default()
    }

    pub(super) fn hit(&self, column: u16, row: u16) -> Option<usize> {
        self.slots
            .iter()
            .position(|slot| slot.contains(Position::new(column, row)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> DmenuUI<'static> {
        DmenuUI::new(
            ["one\ntwo\nthree", "short", "last\nline"]
                .into_iter()
                .enumerate()
                .map(|(index, text)| Item::new_simple(text.into(), text.into(), index + 1))
                .collect(),
            false,
            false,
        )
    }

    #[test]
    fn multiline_items_reserve_each_line_and_mouse_hits_match() {
        for reverse in [false, true] {
            let mut ui = fixture();
            let layout = ItemLayout::new(Rect::new(2, 4, 30, 8), &mut ui, false, reverse, 10);
            assert_eq!(layout.capacity(), 3);
            for (index, height) in [3, 1, 2].into_iter().enumerate() {
                let slot = layout.slot(index);
                assert_eq!(slot.height, height);
                for row in slot.y..slot.bottom() {
                    assert_eq!(layout.hit(slot.x, row), Some(index));
                }
            }
            assert!(layout.slot(0).intersection(layout.slot(1)).is_empty());
        }
    }

    #[test]
    fn selecting_last_item_scrolls_by_rendered_height() {
        let mut ui = fixture();
        ui.selected = Some(2);
        let layout = ItemLayout::new(Rect::new(0, 0, 30, 4), &mut ui, false, false, 10);
        assert_eq!(ui.scroll_offset, 1);
        assert_eq!(layout.capacity(), 2);
        assert_eq!(layout.slot(1).height, 2);
    }

    #[test]
    fn horizontal_items_use_their_multiline_height() {
        let mut ui = fixture();
        let layout = ItemLayout::new(Rect::new(0, 0, 30, 8), &mut ui, true, false, 10);
        assert_eq!(layout.slot(0), Rect::new(0, 0, 10, 3));
        assert_eq!(layout.hit(1, 2), Some(0));
        assert_eq!(layout.hit(11, 2), None);
    }

    #[test]
    fn oversized_item_is_visible_without_hiding_selection() {
        let mut ui = fixture();
        let layout = ItemLayout::new(Rect::new(0, 0, 30, 2), &mut ui, false, false, 10);
        assert_eq!(layout.capacity(), 1);
        assert_eq!(layout.slot(0).height, 2);
        assert_eq!(ui.scroll_offset, 0);
    }
}
