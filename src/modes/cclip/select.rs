// Selection, copying, and tagging functionality

use super::CclipItem;
use eyre::{Result, eyre};
use std::io;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const CLIPBOARD_PROVIDER_STARTUP_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Debug, Eq, PartialEq)]
enum ClipboardProviderState {
    Exited,
    StillRunning,
}

impl CclipItem {
    /// Copy this item back to the clipboard (Wayland)
    fn copy_to_clipboard_wayland(&self) -> Result<()> {
        if command_is_available("wl-copy")
            && let Ok(()) = self.copy_to_clipboard_wayland_with_wl_copy()
        {
            return Ok(());
        }

        let mut child = Command::new("cclip")
            .args(["copy", &self.rowid])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        wait_for_clipboard_provider_start(
            &mut child,
            "cclip copy",
            CLIPBOARD_PROVIDER_STARTUP_TIMEOUT,
        )?;

        Ok(())
    }

    fn copy_to_clipboard_wayland_with_wl_copy(&self) -> Result<()> {
        let mut cclip_child = Command::new("cclip")
            .args(["get", &self.rowid])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let cclip_stdout = cclip_child
            .stdout
            .take()
            .ok_or_else(|| eyre!("failed to capture cclip stdout"))?;

        let mut wl_copy_child = Command::new("wl-copy")
            .args(["--type", &self.mime_type])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        let wl_copy_stdin = wl_copy_child
            .stdin
            .take()
            .ok_or_else(|| eyre!("failed to open wl-copy stdin"))?;

        let pipe_handle = std::thread::spawn(move || {
            let mut source = cclip_stdout;
            let mut sink = wl_copy_stdin;
            io::copy(&mut source, &mut sink)
        });

        let cclip_output = cclip_child.wait_with_output()?;
        let copied_bytes = pipe_handle
            .join()
            .map_err(|_| eyre!("clipboard pipe thread panicked"))??;
        wait_for_clipboard_provider_start(
            &mut wl_copy_child,
            "wl-copy",
            CLIPBOARD_PROVIDER_STARTUP_TIMEOUT,
        )?;

        if !cclip_output.status.success() {
            return Err(eyre!(
                "cclip get failed: {}",
                String::from_utf8_lossy(&cclip_output.stderr)
            ));
        }

        if copied_bytes == 0 {
            return Err(eyre!("cclip get returned no data"));
        }

        Ok(())
    }

    /// Copy this item back to the clipboard.
    pub fn copy_to_clipboard(&self) -> Result<()> {
        if std::env::var("WAYLAND_DISPLAY").is_err() {
            return Err(eyre!("cclip mode requires a Wayland session"));
        }

        self.copy_to_clipboard_wayland()
    }
}

fn wait_for_clipboard_provider_start(
    child: &mut Child,
    command: &str,
    timeout: Duration,
) -> Result<ClipboardProviderState> {
    let deadline = Instant::now() + timeout;

    loop {
        if let Some(status) = child.try_wait()? {
            if status.success() {
                return Ok(ClipboardProviderState::Exited);
            }
            return Err(eyre!("{} failed", command));
        }

        if Instant::now() >= deadline {
            return Ok(ClipboardProviderState::StillRunning);
        }

        std::thread::sleep(Duration::from_millis(10));
    }
}

fn command_is_available(command: &str) -> bool {
    Command::new(command)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
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

/// Delete a specific cclip item by rowid
pub fn delete_item(rowid: &str) -> Result<()> {
    let output = Command::new("cclip").args(["delete", rowid]).output()?;

    if !output.status.success() {
        return Err(eyre!(
            "Failed to delete item: {}",
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
    let output = Command::new("cclip")
        .args(["tags", "delete", tag])
        .output()?;

    if !output.status.success() {
        return Err(eyre!(
            "Failed to delete tag '{}': {}",
            tag,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ClipboardProviderState, wait_for_clipboard_provider_start};
    use std::process::{Command, Stdio};
    use std::time::Duration;

    #[test]
    fn provider_start_wait_returns_while_clipboard_owner_stays_running() {
        let mut child = Command::new("sh")
            .args(["-c", "sleep 1"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("test process should spawn");

        let state = wait_for_clipboard_provider_start(
            &mut child,
            "test-provider",
            Duration::from_millis(20),
        )
        .expect("running provider should be accepted");

        assert_eq!(state, ClipboardProviderState::StillRunning);
        child.kill().expect("test process should be killable");
        let _ = child.wait();
    }

    #[test]
    fn provider_start_wait_rejects_fast_failures() {
        let mut child = Command::new("sh")
            .args(["-c", "exit 7"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("test process should spawn");

        let result =
            wait_for_clipboard_provider_start(&mut child, "test-provider", Duration::from_secs(1));

        assert!(result.is_err());
    }
}
