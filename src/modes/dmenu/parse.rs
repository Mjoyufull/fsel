//! Stdin parsing for dmenu mode

use crate::common::Item;
use is_terminal::IsTerminal;
use std::io::{self, BufRead};

/// Check if stdin is being piped to us
pub fn is_stdin_piped() -> bool {
    !io::stdin().is_terminal()
}

/// Read all lines from stdin, replacing invalid UTF-8 sequences.
pub fn read_stdin_lines() -> io::Result<Vec<String>> {
    read_separated(&mut io::stdin().lock(), b'\n')
}

/// Read null-separated input from stdin, replacing invalid UTF-8 sequences.
pub fn read_stdin_null_separated() -> io::Result<Vec<String>> {
    read_separated(&mut io::stdin().lock(), b'\0')
}

fn read_separated(reader: &mut dyn BufRead, separator: u8) -> io::Result<Vec<String>> {
    let mut lines = Vec::new();
    let mut buffer = Vec::new();
    while reader.read_until(separator, &mut buffer)? != 0 {
        if buffer.last() == Some(&separator) {
            buffer.pop();
            // Match BufRead::lines(): strip CR only as part of a CRLF ending.
            if separator == b'\n' && buffer.last() == Some(&b'\r') {
                buffer.pop();
            }
        }
        if separator != b'\0' || !buffer.is_empty() {
            lines.push(String::from_utf8_lossy(&buffer).into_owned());
        }
        buffer.clear();
    }
    Ok(lines)
}

/// Parse stdin lines into Items
pub fn parse_stdin_to_items(
    lines: Vec<String>,
    delimiter: &str,
    with_nth: Option<&Vec<usize>>,
) -> Vec<Item> {
    lines
        .into_iter()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty()) // Skip empty lines
        .map(|(idx, line)| Item::new(line, idx + 1, delimiter, with_nth))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newline_input_accepts_invalid_utf8_and_preserves_cliphist_ids() {
        let lines = read_separated(&mut &b"42\tpreview\xff\n43\tvalid\n"[..], b'\n').unwrap();
        let items = parse_stdin_to_items(lines, "\t", Some(&vec![2]));

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].original_line, "42\tpreview\u{fffd}");
        assert_eq!(items[0].display_text, "preview\u{fffd}");
        assert_eq!(items[0].get_accept_nth_output(&[1]), "42");
        assert_eq!(items[1].original_line, "43\tvalid");
    }

    #[test]
    fn newline_input_preserves_line_ending_behavior() {
        let input = "café\r\n\nembedded\rcarriage\nfinal\r";
        let lines = read_separated(&mut input.as_bytes(), b'\n').unwrap();

        assert_eq!(lines, ["café", "", "embedded\rcarriage", "final\r"]);
        assert!(read_separated(&mut &b""[..], b'\n').unwrap().is_empty());
    }

    #[test]
    fn null_input_accepts_invalid_utf8_and_skips_empty_records() {
        let mut input = &b"\0first\xff\0\0second\r\nline\0last\xfe"[..];
        let lines = read_separated(&mut input, b'\0').unwrap();

        assert_eq!(lines, ["first\u{fffd}", "second\r\nline", "last\u{fffd}"]);
        assert_eq!(
            read_separated(&mut &b"valid\0"[..], b'\0').unwrap(),
            ["valid"]
        );
        assert!(read_separated(&mut &b"\0\0"[..], b'\0').unwrap().is_empty());
        assert!(read_separated(&mut &b""[..], b'\0').unwrap().is_empty());
    }

    #[test]
    fn read_errors_are_still_propagated() {
        struct FailingReader;

        impl io::Read for FailingReader {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::other("test read failure"))
            }
        }

        for separator in *b"\n\0" {
            let mut reader = io::BufReader::new(FailingReader);
            let error = read_separated(&mut reader, separator).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::Other);
            assert_eq!(error.to_string(), "test read failure");
        }
    }
}
