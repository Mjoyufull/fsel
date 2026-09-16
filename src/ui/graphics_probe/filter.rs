//! Remove only replies to our queries; retain keyboard bytes without decoding them.

use ratatui_image::picker::cap_parser::{Parser, Response};

#[derive(Default)]
pub(super) struct ReplyFilter {
    pending: Vec<u8>,
}

impl ReplyFilter {
    pub(super) fn push(&mut self, byte: u8, input: &mut Vec<u8>) -> Vec<Response> {
        if self.pending.is_empty() && byte != 27 {
            input.push(byte);
            return Vec::new();
        }
        if byte == 27 && !self.pending.starts_with(b"\x1b_Gi=31;") {
            input.append(&mut self.pending);
        }
        self.pending.push(byte);
        let prefixes: [&[u8]; 4] = [b"\x1b[?", b"\x1b[6;", b"\x1b[0n", b"\x1b_Gi=31;"];
        if prefixes
            .iter()
            .any(|prefix| prefix.starts_with(&self.pending))
        {
            if self.pending == b"\x1b[0n" {
                return self.complete();
            }
            return Vec::new();
        }
        let kitty = self.pending.starts_with(prefixes[3]);
        if kitty && self.pending.ends_with(b"\x1b\\") {
            return self.complete();
        }
        let device = self.pending.starts_with(prefixes[0]);
        let size = self.pending.starts_with(prefixes[1]);
        if (device || size) && (0x40..=0x7e).contains(&byte) {
            if (device && byte == b'c') || (size && byte == b't') {
                return self.complete();
            }
        } else if (kitty || device || size) && self.pending.len() < 4096 {
            return Vec::new();
        }
        input.append(&mut self.pending);
        Vec::new()
    }

    fn complete(&mut self) -> Vec<Response> {
        let mut parser = Parser::new();
        self.pending
            .drain(..)
            .flat_map(|byte| parser.push(char::from(byte)))
            .collect()
    }

    pub(super) fn release_ambiguous_escape(&mut self, input: &mut Vec<u8>) {
        if self.pending.len() <= 2 {
            input.append(&mut self.pending);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn responses_do_not_consume_surrounding_keyboard_bytes() {
        let mut filter = ReplyFilter::default();
        let mut input = Vec::new();
        let mut replies = Vec::new();
        for byte in b"f\x1b[?62;4c\xc3\xa9\x1b[6;18;9t\x1b[1;5D\x1b_Gi=31;OK\x1b\\\x1b[0nrest\r" {
            replies.extend(filter.push(*byte, &mut input));
        }
        assert_eq!(input, b"f\xc3\xa9\x1b[1;5Drest\r");
        assert!(replies.contains(&Response::Sixel));
        assert!(replies.contains(&Response::Kitty));
        assert!(replies.contains(&Response::Status));
    }

    #[test]
    fn partial_keys_and_modified_page_down_remain_byte_exact() {
        let mut filter = ReplyFilter::default();
        let mut input = Vec::new();
        for chunk in [b"\xc3".as_slice(), b"\xa9\x1b[", b"6;2~\x1b", b"[A"] {
            for byte in chunk {
                filter.push(*byte, &mut input);
            }
        }
        assert_eq!(input, b"\xc3\xa9\x1b[6;2~\x1b[A");
    }
}
