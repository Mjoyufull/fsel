//! Erase replaced Sixel pixels before Ratatui flushes the replacement frame.

use crossterm::{cursor, queue, style};
use ratatui::{
    buffer::{Buffer, CellDiffOption},
    layout::Rect,
};
use std::{
    borrow::Cow,
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

    pub(super) fn record(&mut self, buffer: &mut Buffer, area: Rect) {
        if area.is_empty() {
            return;
        }
        anchor_payload(buffer, area);
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

/// Shrink `area` so a graphic drawn in it cannot reach the last row of `screen`.
///
/// Terminals leave the text cursor below a drawn graphic, and against the bottom margin
/// that is a line feed: the screen scrolls, and every panel travels up with it on each
/// frame. One row of slack gives that cursor a row that already exists. The Alacritty
/// image fork advances past the graphic whatever its payload says, so the slack is what
/// keeps an image panel at the bottom of the terminal from walking the interface away.
pub(in crate::modes::dmenu) fn off_bottom_margin(mut area: Rect, screen: Rect) -> Rect {
    let last_row = screen.bottom().saturating_sub(1);
    if area.bottom() > last_row {
        area.height = last_row.saturating_sub(area.y);
    }
    area
}

fn anchor_payload(buffer: &mut Buffer, area: Rect) {
    let cell = &mut buffer[(area.x, area.y)];
    let payload = cell.symbol();
    let tmux = payload.starts_with("\x1bPtmux;");
    let delimiter = if tmux { "\x1b\x1bP" } else { "\x1bP" };
    let Some(start) = payload.find(delimiter) else {
        return;
    };
    let escape = if tmux { "\x1b\x1b" } else { "\x1b" };
    let sixel = without_trailing_band_advance(&payload[start..]);
    // Upstream clears rows using CUD/CUU. CUD clamps at the bottom edge, so
    // its final CUU can overshoot. Re-anchor after that clear, before the DCS.
    let anchored = format!(
        "{}{escape}[{};{}H{sixel}\x1b[{};{}H",
        &payload[..start],
        u32::from(area.y) + 1,
        u32::from(area.x) + 1,
        u32::from(area.y) + 1,
        u32::from(area.x) + 2,
    );
    // Ratatui treats this escape payload as one cell. Sixel cursor advancement
    // varies by emulator; restore the position expected by the next text write.
    cell.set_symbol(&anchored);
}

/// Drop the graphics newlines that trail the final band of an encoded image.
///
/// The encoder ends every band with one, including the last, so the graphic advances a
/// six-pixel band past the height its raster attributes declare. Terminals charge that
/// band to the next text row: the image overruns the row below it, and against the
/// bottom margin every frame scrolls the screen and carries the text panels up with it.
fn without_trailing_band_advance(sixel: &str) -> Cow<'_, str> {
    // ESC [ESC] P <params> q <data> ESC \ — the parameters hold no `q`, the data no escape.
    let Some(data) = sixel
        .find('P')
        .and_then(|intro| sixel[intro..].find('q').map(|end| intro + end + 1))
    else {
        return Cow::Borrowed(sixel);
    };
    let Some(terminator) = sixel[data..].find('\x1b').map(|end| data + end) else {
        return Cow::Borrowed(sixel);
    };
    let bands = sixel[data..terminator].trim_end_matches('-');
    if bands.len() == terminator - data {
        return Cow::Borrowed(sixel);
    }
    Cow::Owned(format!("{}{bands}{}", &sixel[..data], &sixel[terminator..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoded_sixel_keeps_its_anchor_at_the_bottom_edge_across_frames() {
        use image::{DynamicImage, Rgba, RgbaImage};
        use ratatui::layout::Size;
        use ratatui::widgets::Widget;
        use ratatui_image::Image;
        use ratatui_image::protocol::{Protocol, sixel::Sixel};

        let image =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(16, 32, Rgba([20, 80, 220, 255])));
        let protocol = Protocol::Sixel(Sixel::new(image, Size::new(2, 2), false).unwrap());
        let area = Rect::new(2, 8, 2, 2);
        let mut damage = SixelDamage::default();
        let mut output = Vec::new();
        for _ in 0..2 {
            let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 10));
            Image::new(&protocol).render(area, &mut buffer);
            damage.begin_frame();
            damage.record(&mut buffer, area);
            let payload = buffer[(2, 8)].symbol();
            assert!(payload.contains("\x1b[2A\x1b[9;3H\x1bP"));
            assert!(payload.ends_with("\x1b[9;4H"));
            assert_eq!(
                buffer[(2, 8)].diff_option,
                CellDiffOption::ForcedWidth(std::num::NonZeroU16::new(1).unwrap())
            );
            damage.erase_changed(&mut buffer, &mut output).unwrap();
        }
        assert!(
            output.is_empty(),
            "unchanged frames must not erase the image"
        );
    }

    #[test]
    fn encoded_sixel_ends_on_its_last_band_instead_of_advancing_past_it() {
        use image::{DynamicImage, Rgba, RgbaImage};
        use ratatui::layout::Size;
        use ratatui::widgets::Widget;
        use ratatui_image::Image;
        use ratatui_image::protocol::{Protocol, sixel::Sixel};

        // 26 pixel rows over two cells: the height is not a multiple of the six-pixel band.
        let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(16, 26, Rgba([0, 0, 0, 255])));
        let protocol = Protocol::Sixel(Sixel::new(image, Size::new(2, 2), false).unwrap());
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 10));
        let area = Rect::new(2, 8, 2, 2);
        Image::new(&protocol).render(area, &mut buffer);
        anchor_payload(&mut buffer, area);
        let payload = buffer[(2, 8)].symbol();
        let (bands, _) = payload.split_once("\x1b\\").unwrap();
        assert!(!bands.ends_with('-'), "payload claims an extra band");
    }

    #[test]
    fn an_image_is_kept_off_the_last_row_of_the_terminal() {
        let screen = Rect::new(0, 0, 40, 20);

        assert_eq!(
            off_bottom_margin(Rect::new(2, 4, 10, 5), screen),
            Rect::new(2, 4, 10, 5),
            "an area clear of the bottom margin is left alone"
        );
        assert_eq!(
            off_bottom_margin(Rect::new(2, 14, 10, 6), screen),
            Rect::new(2, 14, 10, 5)
        );
        assert_eq!(
            off_bottom_margin(Rect::new(2, 19, 10, 1), screen),
            Rect::new(2, 19, 10, 0),
            "a slot with only the last row left draws nothing"
        );
    }

    #[test]
    fn every_trailing_band_newline_is_dropped_including_transparent_ones() {
        let sixel = "\x1bP9;1;0q\"1;1;8;26#0!8~$---\x1b\\";
        assert_eq!(
            without_trailing_band_advance(sixel),
            "\x1bP9;1;0q\"1;1;8;26#0!8~$\x1b\\"
        );
        let tmux = "\x1b\x1bP9;1;0q\"1;1;8;26#0!8~$-\x1b\\\x1b\\";
        assert_eq!(
            without_trailing_band_advance(tmux),
            "\x1b\x1bP9;1;0q\"1;1;8;26#0!8~$\x1b\\\x1b\\"
        );
    }

    #[test]
    fn a_band_aligned_payload_is_left_untouched() {
        let sixel = "\x1bP9;1;0q\"1;1;8;24#0!8~$\x1b\\";
        assert!(matches!(
            without_trailing_band_advance(sixel),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn payload_is_anchored_after_clear_and_leaves_a_one_cell_cursor_advance() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 10));
        let area = Rect::new(2, 8, 5, 2);
        buffer[(2, 8)].set_symbol("\x1b[5X\x1b[1B\x1b[5X\x1b[1B\x1b[2A\x1bPqDATA\x1b\\");
        anchor_payload(&mut buffer, area);
        let payload = buffer[(2, 8)].symbol();
        assert!(payload.contains("\x1b[2A\x1b[9;3H\x1bPqDATA"));
        assert!(payload.ends_with("\x1b\\\x1b[9;4H"));
    }

    #[test]
    fn tmux_anchor_is_inside_passthrough_and_cursor_restore_is_outside() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 10));
        buffer[(2, 3)].set_symbol("\x1bPtmux;\x1b\x1b[2A\x1b\x1bPqDATA\x1b\x1b\\\x1b\\");
        anchor_payload(&mut buffer, Rect::new(2, 3, 5, 2));
        let payload = buffer[(2, 3)].symbol();
        assert!(payload.contains("\x1b\x1b[4;3H\x1b\x1bPqDATA"));
        assert!(payload.ends_with("\x1b\\\x1b[4;4H"));
    }

    #[test]
    fn replacement_erases_old_rectangle_before_drawing_smaller_transparent_image() {
        let mut damage = SixelDamage::default();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 10));
        let old = Rect::new(2, 3, 5, 4);
        buffer[(2, 3)].set_symbol("old sixel payload");
        damage.record(&mut buffer, old);
        let mut output = Vec::new();
        damage.erase_changed(&mut buffer, &mut output).unwrap();
        assert!(output.is_empty());
        damage.begin_frame();
        damage.record(&mut buffer, old);
        damage.erase_changed(&mut buffer, &mut output).unwrap();
        assert!(
            output.is_empty(),
            "pending replacement must keep the old image"
        );
        damage.begin_frame();
        buffer[(2, 3)].set_symbol("new transparent payload");
        damage.record(&mut buffer, Rect::new(2, 3, 2, 2));
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
        damage.record(&mut buffer, Rect::new(2, 3, 10, 6));
        damage.erase_changed(&mut buffer, &mut output).unwrap();
        damage.begin_frame();
        damage.record(&mut buffer, Rect::new(2, 3, 6, 4));
        damage.erase_changed(&mut buffer, &mut output).unwrap();
        assert!(output.is_empty());
    }
}
