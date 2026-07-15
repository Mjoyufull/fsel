use crate::ui::{DmenuUI, GraphicsAdapter, ImageManager};
use eyre::Result;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use std::process::Stdio;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

const MAX_PREVIEW_BYTES: u64 = 32 * 1024 * 1024;

pub(super) struct PreviewRuntime {
    command_template: Option<String>,
    content: PreviewContent,
    image_manager: ImageManager,
    active_request: Option<JoinHandle<()>>,
    current_signature: Option<PreviewSignature>,
    generation: u64,
    result_tx: mpsc::UnboundedSender<PreviewResult>,
    result_rx: mpsc::UnboundedReceiver<PreviewResult>,
}

#[derive(Debug, PartialEq, Eq)]
struct PreviewSignature {
    selected: usize,
    item: String,
    query: String,
}

enum PreviewContent {
    Empty,
    Loading,
    Text(String),
    Image(String),
}

pub(super) struct PreviewResult {
    generation: u64,
    output: Result<CommandOutput, String>,
}

struct CommandOutput {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: String,
    success: bool,
    truncated: bool,
}

impl PreviewRuntime {
    pub(super) fn new(command_template: Option<String>, adapter: GraphicsAdapter) -> Self {
        let (result_tx, result_rx) = mpsc::unbounded_channel();
        Self {
            command_template,
            content: PreviewContent::Empty,
            image_manager: ImageManager::new(adapter.picker()),
            active_request: None,
            current_signature: None,
            generation: 0,
            result_tx,
            result_rx,
        }
    }

    pub(super) fn is_enabled(&self) -> bool {
        self.command_template.is_some()
    }

    pub(super) fn title(&self) -> &'static str {
        if self.is_enabled() {
            " Preview "
        } else {
            " Content "
        }
    }

    pub(super) fn text_lines(&self) -> Option<Vec<Line<'static>>> {
        match &self.content {
            PreviewContent::Empty => Some(Vec::new()),
            PreviewContent::Loading => Some(vec![Line::from("Loading preview…")]),
            PreviewContent::Text(text) => Some(
                text.lines()
                    .map(|line| Line::from(line.to_string()))
                    .collect(),
            ),
            PreviewContent::Image(_) => None,
        }
    }

    pub(super) fn request_if_changed(&mut self, ui: &DmenuUI<'_>) {
        let Some(command_template) = self.command_template.as_deref() else {
            return;
        };
        let Some(selected) = ui.selected else {
            self.clear_request();
            return;
        };
        let Some(item) = ui.shown.get(selected) else {
            self.clear_request();
            return;
        };

        let signature = PreviewSignature {
            selected,
            item: item.original_line.clone(),
            query: ui.query.clone(),
        };
        if self.current_signature.as_ref() == Some(&signature) {
            return;
        }

        if let Some(task) = self.active_request.take() {
            task.abort();
        }
        self.generation = self.generation.wrapping_add(1);
        self.content = PreviewContent::Loading;

        let generation = self.generation;
        let command = expand_preview_command(
            command_template,
            &signature.item,
            &signature.query,
            signature.selected,
        );
        let result_tx = self.result_tx.clone();
        self.active_request = Some(tokio::spawn(async move {
            let output = run_preview_command(&command).await;
            let _ = result_tx.send(PreviewResult { generation, output });
        }));
        self.current_signature = Some(signature);
    }

    pub(super) async fn next_result(&mut self) -> Option<PreviewResult> {
        self.result_rx.recv().await
    }

    pub(super) async fn apply_result(&mut self, result: PreviewResult) {
        if result.generation != self.generation {
            return;
        }
        self.active_request = None;

        let output = match result.output {
            Ok(output) => output,
            Err(error) => {
                self.content = PreviewContent::Text(error);
                return;
            }
        };

        if !output.success {
            let stderr = output_text(&output.stderr);
            self.content = PreviewContent::Text(if stderr.trim().is_empty() {
                format!("Preview command exited with {}", output.status)
            } else {
                stderr
            });
            return;
        }

        let image_key = format!("dmenu-preview-{}", result.generation);
        if image::guess_format(&output.stdout).is_ok()
            && self
                .image_manager
                .load_image_bytes(&image_key, output.stdout.clone())
                .await
                .is_ok()
        {
            self.content = PreviewContent::Image(image_key);
            return;
        }

        let mut text = output_text(&output.stdout);
        if output.truncated {
            text.push_str("\n\n[preview output truncated]");
        }
        self.content = PreviewContent::Text(text);
    }

    pub(super) fn render_image(&mut self, frame: &mut Frame, area: Rect) -> Result<bool> {
        let PreviewContent::Image(key) = &self.content else {
            return Ok(false);
        };
        self.image_manager.render_cached(frame, key, area)
    }

    fn clear_request(&mut self) {
        if let Some(task) = self.active_request.take() {
            task.abort();
        }
        self.generation = self.generation.wrapping_add(1);
        self.current_signature = None;
        self.content = PreviewContent::Empty;
    }
}

impl Drop for PreviewRuntime {
    fn drop(&mut self) {
        if let Some(task) = self.active_request.take() {
            task.abort();
        }
    }
}

fn expand_preview_command(template: &str, item: &str, query: &str, selected: usize) -> String {
    let item = shell_quote(item);
    let query = shell_quote(query);
    let selected = selected.to_string();
    let mut command = String::with_capacity(template.len() + item.len() + query.len());
    let mut remaining = template;

    while !remaining.is_empty() {
        if let Some(rest) = remaining.strip_prefix("{}") {
            command.push_str(&item);
            remaining = rest;
        } else if let Some(rest) = remaining.strip_prefix("{q}") {
            command.push_str(&query);
            remaining = rest;
        } else if let Some(rest) = remaining.strip_prefix("{n}") {
            command.push_str(&selected);
            remaining = rest;
        } else {
            let Some(character) = remaining.chars().next() else {
                break;
            };
            command.push(character);
            remaining = &remaining[character.len_utf8()..];
        }
    }

    command
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

async fn run_preview_command(command: &str) -> Result<CommandOutput, String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let mut child = tokio::process::Command::new(shell)
        .args(["-c", command])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| format!("Failed to start preview command: {error}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture preview stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture preview stderr".to_string())?;

    let (status, stdout_result, stderr_result) =
        tokio::join!(child.wait(), read_limited(stdout), read_limited(stderr),);
    let status = status.map_err(|error| format!("Preview command failed: {error}"))?;
    let (stdout, stdout_truncated) =
        stdout_result.map_err(|error| format!("Failed to read preview output: {error}"))?;
    let (stderr, stderr_truncated) =
        stderr_result.map_err(|error| format!("Failed to read preview error output: {error}"))?;

    Ok(CommandOutput {
        stdout,
        stderr,
        status: status.to_string(),
        success: status.success(),
        truncated: stdout_truncated || stderr_truncated,
    })
}

async fn read_limited(reader: impl AsyncRead + Unpin) -> std::io::Result<(Vec<u8>, bool)> {
    let mut bytes = Vec::new();
    reader
        .take(MAX_PREVIEW_BYTES + 1)
        .read_to_end(&mut bytes)
        .await?;
    let truncated = bytes.len() as u64 > MAX_PREVIEW_BYTES;
    if truncated {
        bytes.truncate(MAX_PREVIEW_BYTES as usize);
    }
    Ok((bytes, truncated))
}

fn output_text(bytes: &[u8]) -> String {
    let stripped = strip_ansi_escapes::strip(bytes);
    String::from_utf8_lossy(&stripped).trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::{expand_preview_command, shell_quote};

    #[test]
    fn command_expansion_quotes_selected_item_and_query() {
        let command =
            expand_preview_command("printf '%s %s %s' {} {q} {n}", "it's here", "two words", 4);

        assert_eq!(command, "printf '%s %s %s' 'it'\"'\"'s here' 'two words' 4");
    }

    #[test]
    fn shell_quote_preserves_empty_arguments() {
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn command_expansion_does_not_reexpand_placeholder_text_inside_values() {
        let command = expand_preview_command("printf '%s %s' {} {q}", "{q}", "{}", 0);

        assert_eq!(command, "printf '%s %s' '{q}' '{}'");
    }
}
