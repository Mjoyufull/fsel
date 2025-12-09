// Selection, copying, and tagging functionality

use super::CclipItem;
use eyre::{eyre, Result};
use std::process::{Command, Stdio};

impl CclipItem {
    /// Copy this item back to the clipboard (Wayland)
    fn copy_to_clipboard_wayland(&self) -> Result<()> {
        let mut cclip_child = Command::new("cclip")
            .args(["get", &self.rowid])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let mut wl_copy_child = Command::new("wl-copy")
            .args(["-t", &self.mime_type])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        // pipe cclip output to wl-copy
        if let (Some(cclip_stdout), Some(wl_copy_stdin)) =
            (cclip_child.stdout.take(), wl_copy_child.stdin.take())
        {
            std::thread::spawn(move || {
                let mut cclip_stdout = cclip_stdout;
                let mut wl_copy_stdin = wl_copy_stdin;
                std::io::copy(&mut cclip_stdout, &mut wl_copy_stdin).ok();
            });
        }

        let cclip_status = cclip_child.wait()?;
        let wl_copy_status = wl_copy_child.wait()?;

        if !cclip_status.success() {
            return Err(eyre!("cclip get failed"));
        }

        if !wl_copy_status.success() {
            return Err(eyre!("wl-copy failed"));
        }

        Ok(())
    }

    /// Copy this item back to the clipboard (X11)
    fn copy_to_clipboard_x11(&self) -> Result<()> {
        // try xclip first, then xsel as fallback
        let x11_tools = ["xclip", "xsel"];

        for tool in &x11_tools {
            if !Command::new(tool)
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            {
                continue;
            }

            let mut cclip_child = Command::new("cclip")
                .args(["get", &self.rowid])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()?;

            let args = match *tool {
                "xclip" => vec!["-selection", "clipboard", "-t", &self.mime_type],
                "xsel" => vec!["--clipboard", "--input"],
                _ => unreachable!(),
            };

            let mut x11_child = Command::new(tool)
                .args(&args)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;

            // pipe cclip output to x11 tool
            if let (Some(cclip_stdout), Some(x11_stdin)) =
                (cclip_child.stdout.take(), x11_child.stdin.take())
            {
                std::thread::spawn(move || {
                    let mut cclip_stdout = cclip_stdout;
                    let mut x11_stdin = x11_stdin;
                    std::io::copy(&mut cclip_stdout, &mut x11_stdin).ok();
                });
            }

            let cclip_status = cclip_child.wait()?;
            let x11_status = x11_child.wait()?;

            if !cclip_status.success() {
                return Err(eyre!("cclip get failed"));
            }

            if !x11_status.success() {
                continue; // try next tool
            }

            return Ok(());
        }

        Err(eyre!("no X11 clipboard tool found (tried xclip, xsel)"))
    }

    /// Copy this item back to the clipboard (auto-detect Wayland/X11)
    pub fn copy_to_clipboard(&self) -> Result<()> {
        // check if we're on wayland
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            self.copy_to_clipboard_wayland()
        } else {
            self.copy_to_clipboard_x11()
        }
    }
}

/// Tag a cclip item using cclip's tag command
pub fn tag_item(rowid: &str, tag: &str) -> Result<()> {
    let output = Command::new("cclip").args(["tag", rowid, tag]).output()?;

    if !output.status.success() {
        return Err(eyre!(
            "Failed to tag item: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

/// Remove tag from a cclip item. If `tag` is `None`, all tags are removed.
pub fn untag_item(rowid: &str, tag: Option<&str>) -> Result<()> {
    let mut args = vec!["tag", "-d", rowid];
    if let Some(tag) = tag {
        args.push(tag);
    }

    let output = Command::new("cclip").args(&args).output()?;

    if !output.status.success() {
        return Err(eyre!(
            "Failed to remove tag: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

/// Wipe all tags from all cclip entries (cclip 3.2.0+)
pub fn wipe_all_tags() -> Result<()> {
    let output = Command::new("cclip").args(["tags", "wipe"]).output()?;

    if !output.status.success() {
        return Err(eyre!(
            "Failed to wipe tags: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

/// Delete a specific tag from cclip (cclip 3.2.0+)
#[allow(dead_code)]
pub fn delete_tag(tag: &str) -> Result<()> {
    let output = Command::new("cclip").args(["tags", "delete", tag]).output()?;

    if !output.status.success() {
        return Err(eyre!(
            "Failed to delete tag '{}': {}",
            tag,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

