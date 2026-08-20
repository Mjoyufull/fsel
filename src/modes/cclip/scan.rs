// Clipboard database scanning functions

use super::CclipItem;
use eyre::{Result, eyre};
use std::process::{Command, Output, Stdio};

const HISTORY_FIELD_SETS: [&str; 4] = [
    "rowid,mime_type,preview,data_size,timestamp,tag",
    "rowid,mime_type,preview,data_size,timestamp",
    "rowid,mime_type,preview,tag",
    "rowid,mime_type,preview",
];
const TAGGED_HISTORY_FIELD_SETS: [&str; 2] = [
    "rowid,mime_type,preview,data_size,timestamp,tag",
    "rowid,mime_type,preview,tag",
];

/// Get clipboard history from cclip
pub fn get_clipboard_history() -> Result<Vec<CclipItem>> {
    let output = run_cclip_list(&[], &HISTORY_FIELD_SETS)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(eyre!("cclip list failed: {}", stderr));
    }

    let stdout = String::from_utf8(output.stdout)?;
    let mut items = Vec::new();

    for line in stdout.lines() {
        if !line.trim().is_empty() {
            let parsed_item = CclipItem::from_line(line.to_string());
            match parsed_item {
                Ok(item) => items.push(item),
                Err(e) => eprintln!("Warning: Failed to parse cclip line: {}", e),
            }
        }
    }

    Ok(items)
}

/// Get clipboard items filtered by tag
pub fn get_clipboard_history_by_tag(tag: &str) -> Result<Vec<CclipItem>> {
    let output = run_cclip_list(&["-T", tag], &TAGGED_HISTORY_FIELD_SETS)?;

    if !output.status.success() {
        return Err(eyre!("Failed to get clipboard history"));
    }

    let items: Result<Vec<CclipItem>> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| CclipItem::from_line(line.to_string()))
        .collect();

    items
}

fn run_cclip_list(extra_args: &[&str], field_sets: &[&str]) -> Result<Output> {
    let mut last_output = None;

    for fields in field_sets {
        let output = Command::new("cclip")
            .arg("list")
            .args(extra_args)
            .arg(fields)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output()?;
        let unsupported_field = String::from_utf8_lossy(&output.stderr).contains("invalid field:");

        if output.status.success() || !unsupported_field {
            return Ok(output);
        }
        last_output = Some(output);
    }

    last_output.ok_or_else(|| eyre!("No cclip field sets configured"))
}

/// Get all unique tags from cclip database
pub fn get_all_tags() -> Result<Vec<String>> {
    let output = Command::new("cclip").arg("tags").output()?;

    if !output.status.success() {
        return Err(eyre!(
            "Failed to list tags: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let tags: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
        .collect();

    Ok(tags)
}

/// Check if cclip is available on the system
pub fn check_cclip_available() -> bool {
    Command::new("cclip")
        .arg("-h")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Check if cclip database exists and has entries
pub fn check_cclip_database() -> Result<()> {
    let output = Command::new("cclip")
        .args(["list", "rowid"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .wait_with_output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("unable to open database file") {
            return Err(eyre!(
                "cclip database not found. Make sure cclipd is running and has stored some clipboard history."
            ));
        } else {
            return Err(eyre!("cclip error: {}", stderr));
        }
    }

    Ok(())
}
