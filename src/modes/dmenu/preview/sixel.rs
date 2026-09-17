//! Erase replaced Sixel pixels before Ratatui flushes the replacement frame.

use crossterm::{cursor, queue, style};
use ratatui::{
    buffer::{Buffer, CellDiffOption},
    layout::Rect,
};
use std::{
    hash::{Hash, Hasher},
    io::{self, Write},
};

#[derive(Default)]
pub(in crate::modes::dmenu) struct SixelDamage {
    previous: Option<(Rect, u64)>,
    current: Option<(Rect, u64)>,
}

impl SixelDamage {
    pub(in crate::modes::dmenu) fn begin_frame(&mut self) {
        self.current = None;
    }

    pub(super) fn record(&mut self, buffer: &Buffer, area: Rect) {
        if area.is_empty() {
            return;
        }
        // The anchor contains the encoded payload, including its dimensions. Comparing it
        // avoids clearing a retained image while the next command/decoder is still pending.
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        buffer[(area.x, area.y)].symbol().hash(&mut hash);
        self.current = Some((area, hash.finish()));
    }

    pub(in crate::modes::dmenu) fn erase_changed(
        &mut self,
        buffer: &mut Buffer,
        output: &mut impl Write,
    ) -> io::Result<()> {
        let old = self.previous;
        self.previous = self.current;
        if old == self.current {
            return Ok(());
        }
        let Some((area, old_hash)) = old else {
            return Ok(());
        };
        // A resized slot (or swapped identical panels) can still contain exactly the same
        // encoded image at the old position. Erasing it would remove pixels without causing
        // Ratatui to resend that unchanged image anchor.
        if let Some(cell) = buffer.cell((area.x, area.y)) {
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            cell.symbol().hash(&mut hash);
            if hash.finish() == old_hash {
                return Ok(());
            }
        }
        let area = area.intersection(buffer.area);
        if area.is_empty() {
            return Ok(());
        }

        // Writing spaces erases Sixel pixels on terminals where buffer changes or ECH alone
        // leave transparent remnants. Do not clear the screen or query its cursor position.
        queue!(
            output,
            cursor::SavePosition,
            style::ResetColor,
            style::SetAttribute(style::Attribute::Reset)
        )?;
        let spaces = " ".repeat(usize::from(area.width));
        for y in area.top()..area.bottom() {
            queue!(output, cursor::MoveTo(area.x, y), style::Print(&spaces))?;
            for x in area.left()..area.right() {
                let cell = &mut buffer[(x, y)];
                // A moved panel can expose unchanged text underneath its old area.
                // Image anchors retain ForcedWidth; their changed payload/position is diffed.
                if cell.diff_option == CellDiffOption::None {
                    cell.set_diff_option(CellDiffOption::AlwaysUpdate);
                }
            }
        }
        queue!(output, cursor::RestorePosition)?;
        output.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacement_erases_old_rectangle_before_drawing_smaller_transparent_image() {
        let mut damage = SixelDamage::default();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 10));
        let old = Rect::new(2, 3, 5, 4);
        buffer[(2, 3)].set_symbol("old sixel payload");
        damage.record(&buffer, old);
        let mut output = Vec::new();
        damage.erase_changed(&mut buffer, &mut output).unwrap();
        assert!(output.is_empty());
        damage.begin_frame();
        damage.record(&buffer, old);
        damage.erase_changed(&mut buffer, &mut output).unwrap();
        assert!(
            output.is_empty(),
            "pending replacement must keep the old image"
        );
        damage.begin_frame();
        buffer[(2, 3)].set_symbol("new transparent payload");
        damage.record(&buffer, Rect::new(2, 3, 2, 2));
        damage.erase_changed(&mut buffer, &mut output).unwrap();
        let output = String::from_utf8(output).unwrap();
        for row in 4..8 {
            assert!(output.contains(&format!("\x1b[{row};3H     ")));
        }
        assert!(!output.contains("\x1b[2J"));
        assert!(!output.contains("\x1b[6n"));
        assert_eq!(buffer[(6, 6)].diff_option, CellDiffOption::AlwaysUpdate);
        assert_eq!(buffer[(7, 6)].diff_option, CellDiffOption::None);
    }

    #[test]
    fn hidden_or_failed_image_clears_once_and_clamps_after_resize() {
        let mut damage = SixelDamage::default();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 8, 4));
        damage.previous = Some((Rect::new(6, 2, 10, 10), 1));
        let mut output = Vec::new();
        damage.erase_changed(&mut buffer, &mut output).unwrap();
        let text = String::from_utf8(output.clone()).unwrap();
        assert!(text.contains("\x1b[3;7H  "));
        assert!(text.contains("\x1b[4;7H  "));
        let length = output.len();
        damage.erase_changed(&mut buffer, &mut output).unwrap();
        assert_eq!(length, output.len());
    }

    #[test]
    fn resized_slot_does_not_erase_an_unchanged_image_anchor() {
        let mut damage = SixelDamage::default();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 10));
        buffer[(2, 3)].set_symbol("same small image");
        let mut output = Vec::new();
        damage.record(&buffer, Rect::new(2, 3, 10, 6));
        damage.erase_changed(&mut buffer, &mut output).unwrap();
        damage.begin_frame();
        damage.record(&buffer, Rect::new(2, 3, 6, 4));
        damage.erase_changed(&mut buffer, &mut output).unwrap();
        assert!(output.is_empty());
    }
}
