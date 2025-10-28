//! Stdin parsing for dmenu mode

use std::io::{self, BufRead};
use is_terminal::IsTerminal;
use crate::common::Item;

/// Check if stdin is being piped to us
pub fn is_stdin_piped() -> bool {
    !io::stdin().is_terminal()
}

/// Read all lines from stdin into a vector
pub fn read_stdin_lines() -> io::Result<Vec<String>> {
    let stdin = io::stdin();
    let lines: Result<Vec<String>, io::Error> = stdin.lock().lines().collect();
    lines
}

/// Read null-separated input from stdin
pub fn read_stdin_null_separated() -> io::Result<Vec<String>> {
    use std::io::Read;
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;

    let lines: Vec<String> = buffer
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

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
