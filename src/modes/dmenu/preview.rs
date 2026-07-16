use crate::ui::{DmenuUI, GraphicsAdapter, ImageManager};
use eyre::Result;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui_image::protocol::StatefulProtocol;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

const MAX_PREVIEW_BYTES: u64 = 32 * 1024 * 1024;

pub(super) struct PreviewRuntime {
    command_template: Option<String>,
    content: PreviewContent,
    adapter: GraphicsAdapter,
    image_manager: ImageManager,
    active_request: Option<JoinHandle<()>>,
    decode_tx: mpsc::UnboundedSender<()>,
    decode_request: Arc<Mutex<Option<DecodeRequest>>>,
    _decode_worker: std::thread::JoinHandle<()>,
    current_signature: Option<PreviewSignature>,
    generation: u64,
    previous_was_image: bool,
    result_tx: mpsc::UnboundedSender<PreviewResult>,
    result_rx: mpsc::UnboundedReceiver<PreviewResult>,
}

#[derive(Debug, PartialEq, Eq)]
struct PreviewSignature {
    selected: usize,
    input_ordinal: usize,
    item: String,
    query: String,
}

enum PreviewContent {
    Empty,
    Loading,
    Text(String),
    Image(String),
}

struct DecodeRequest {
    generation: u64,
    key: String,
    bytes: Vec<u8>,
}

pub(super) enum PreviewResult {
    Command {
        generation: u64,
        output: Result<CommandOutput, String>,
    },
    Image {
        generation: u64,
        key: String,
        protocol: Result<Box<StatefulProtocol>, String>,
    },
}

pub(super) struct CommandOutput {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: String,
    success: bool,
    truncated: bool,
    stdout_truncated: bool,
}

impl PreviewRuntime {
    pub(super) fn new(command_template: Option<String>, adapter: GraphicsAdapter) -> Self {
        let (result_tx, result_rx) = mpsc::unbounded_channel();
        let (decode_tx, mut decode_rx) = mpsc::unbounded_channel::<()>();
        let decode_request = Arc::new(Mutex::new(None::<DecodeRequest>));
        let worker_request = Arc::clone(&decode_request);
        let picker = adapter.picker();
        let decode_result_tx = result_tx.clone();
        let decode_worker = std::thread::spawn(move || {
            while decode_rx.blocking_recv().is_some() {
                while decode_rx.try_recv().is_ok() {}
                let request = worker_request
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .take();
                let Some(request) = request else {
                    continue;
                };
                let protocol =
                    ImageManager::prepare_image_bytes_blocking(picker.clone(), request.bytes)
                        .map(Box::new)
                        .map_err(|error| format!("Failed to decode preview image: {error}"));
                if decode_result_tx
                    .send(PreviewResult::Image {
                        generation: request.generation,
                        key: request.key,
                        protocol,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            command_template,
            content: PreviewContent::Empty,
            adapter,
            image_manager: ImageManager::new(adapter.picker()),
            active_request: None,
            decode_tx,
            decode_request,
            _decode_worker: decode_worker,
            current_signature: None,
            generation: 0,
            previous_was_image: false,
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
            input_ordinal: item.line_number.saturating_sub(1),
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
        let command = match expand_preview_command(command_template) {
            Ok(command) => command,
            Err(error) => {
                self.content = PreviewContent::Text(error);
                self.current_signature = Some(signature);
                return;
            }
        };
        let item = signature.item.clone();
        let query = signature.query.clone();
        let input_ordinal = signature.input_ordinal;
        let result_tx = self.result_tx.clone();
        self.active_request = Some(tokio::spawn(async move {
            let output = run_preview_command(&command, &item, &query, input_ordinal).await;
            let _ = result_tx.send(PreviewResult::Command { generation, output });
        }));
        self.current_signature = Some(signature);
    }

    pub(super) async fn next_result(&mut self) -> Option<PreviewResult> {
        self.result_rx.recv().await
    }

    pub(super) fn apply_result(&mut self, result: PreviewResult) {
        let PreviewResult::Command { generation, output } = result else {
            return self.apply_image_result(result);
        };
        if generation != self.generation {
            return;
        }
        self.active_request = None;

        let output = match output {
            Ok(output) => output,
            Err(error) => {
                self.content = PreviewContent::Text(error);
                return;
            }
        };

        if !output.success && !should_show_truncated_stdout(&output) {
            let stderr = output_text(&output.stderr);
            let mut text = if stderr.trim().is_empty() {
                format!("Preview command exited with {}", output.status)
            } else {
                stderr
            };
            append_truncation_notice(&mut text, output.truncated);
            self.content = PreviewContent::Text(text);
            return;
        }

        let image_key = format!("dmenu-preview-{generation}");
        if image::guess_format(&output.stdout).is_ok() {
            let request = DecodeRequest {
                generation,
                key: image_key,
                bytes: output.stdout,
            };
            *self
                .decode_request
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(request);
            let _ = self.decode_tx.send(());
            return;
        }

        let mut text = output_text(&output.stdout);
        append_truncation_notice(&mut text, output.truncated);
        self.content = PreviewContent::Text(text);
    }

    fn apply_image_result(&mut self, result: PreviewResult) {
        let PreviewResult::Image {
            generation,
            key,
            protocol,
        } = result
        else {
            return;
        };
        if generation != self.generation {
            return;
        }
        self.active_request = None;
        match protocol {
            Ok(protocol) => {
                self.image_manager.insert_protocol(key.clone(), *protocol);
                self.content = PreviewContent::Image(key);
            }
            Err(error) => self.content = PreviewContent::Text(error),
        }
    }

    pub(super) fn needs_terminal_clear(&self) -> bool {
        matches!(self.adapter, GraphicsAdapter::Sixel)
            && self.previous_was_image != matches!(&self.content, PreviewContent::Image(_))
    }

    pub(super) fn finish_draw(&mut self) {
        self.previous_was_image = matches!(&self.content, PreviewContent::Image(_));
    }

    pub(super) fn render_image(&mut self, frame: &mut Frame, area: Rect) -> Result<bool> {
        let PreviewContent::Image(key) = &self.content else {
            return Ok(false);
        };
        let key = key.clone();
        if self.image_manager.render_cached(frame, &key, area)? {
            Ok(true)
        } else {
            self.content = PreviewContent::Text("Failed to render preview image".to_string());
            Ok(false)
        }
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

fn expand_preview_command(template: &str) -> Result<String, String> {
    if template.contains("<<") {
        return Err("Preview command heredocs are not supported".to_string());
    }

    let mut command = String::with_capacity(template.len() + 16);
    let mut remaining = template;
    let mut quote = ShellQuote::Unquoted;
    let mut escaped = false;

    while !remaining.is_empty() {
        if !escaped && let Some(rest) = remaining.strip_prefix("{}") {
            require_expandable_placeholder(quote)?;
            command.push_str("\"$FSEL_PREVIEW_ITEM\"");
            remaining = rest;
        } else if !escaped && let Some(rest) = remaining.strip_prefix("{q}") {
            require_expandable_placeholder(quote)?;
            command.push_str("\"$FSEL_PREVIEW_QUERY\"");
            remaining = rest;
        } else if !escaped && let Some(rest) = remaining.strip_prefix("{n}") {
            require_expandable_placeholder(quote)?;
            command.push_str("\"$FSEL_PREVIEW_ORDINAL\"");
            remaining = rest;
        } else {
            let Some(character) = remaining.chars().next() else {
                break;
            };
            command.push(character);
            remaining = &remaining[character.len_utf8()..];
            if escaped {
                escaped = false;
                continue;
            }
            match (quote, character) {
                (ShellQuote::Unquoted, '\\') | (ShellQuote::Double, '\\') => escaped = true,
                (ShellQuote::Unquoted, '\'') => quote = ShellQuote::Single,
                (ShellQuote::Unquoted, '"') => quote = ShellQuote::Double,
                (ShellQuote::Single, '\'') | (ShellQuote::Double, '"') => {
                    quote = ShellQuote::Unquoted;
                }
                _ => {}
            }
        }
    }

    Ok(command)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ShellQuote {
    Unquoted,
    Single,
    Double,
}

fn require_expandable_placeholder(quote: ShellQuote) -> Result<(), String> {
    if quote != ShellQuote::Single {
        Ok(())
    } else {
        Err("Preview placeholders must not appear inside single quotes".to_string())
    }
}

async fn run_preview_command(
    command: &str,
    item: &str,
    query: &str,
    input_ordinal: usize,
) -> Result<CommandOutput, String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let mut process = tokio::process::Command::new(shell);
    process
        .args(["-c", command])
        .env("FSEL_PREVIEW_ITEM", item)
        .env("FSEL_PREVIEW_QUERY", query)
        .env("FSEL_PREVIEW_ORDINAL", input_ordinal.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    process.process_group(0);

    let mut child = process
        .spawn()
        .map_err(|error| format!("Failed to start preview command: {error}"))?;
    #[cfg(unix)]
    let mut process_group = ProcessGroupGuard::new(child.id());

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture preview stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture preview stderr".to_string())?;

    let stdout_task = tokio::spawn(read_limited(stdout));
    let stderr_task = tokio::spawn(read_limited(stderr));
    let status = child.wait().await;
    #[cfg(unix)]
    process_group.terminate();
    let status = status.map_err(|error| format!("Preview command failed: {error}"))?;
    let (stdout_result, stderr_result) = tokio::join!(stdout_task, stderr_task);
    let (stdout, stdout_truncated) = stdout_result
        .map_err(|error| format!("Preview output reader failed: {error}"))?
        .map_err(|error| format!("Failed to read preview output: {error}"))?;
    let (stderr, stderr_truncated) = stderr_result
        .map_err(|error| format!("Preview error reader failed: {error}"))?
        .map_err(|error| format!("Failed to read preview error output: {error}"))?;

    Ok(CommandOutput {
        stdout,
        stderr,
        status: status.to_string(),
        success: status.success(),
        truncated: stdout_truncated || stderr_truncated,
        stdout_truncated,
    })
}

#[cfg(unix)]
struct ProcessGroupGuard {
    pgid: Option<rustix::process::Pid>,
}

#[cfg(unix)]
impl ProcessGroupGuard {
    fn new(pid: Option<u32>) -> Self {
        Self {
            pgid: pid
                .and_then(|pid| i32::try_from(pid).ok())
                .and_then(rustix::process::Pid::from_raw),
        }
    }

    fn terminate(&mut self) {
        if let Some(pgid) = self.pgid.take() {
            let _ = rustix::process::kill_process_group(pgid, rustix::process::Signal::KILL);
        }
    }
}

#[cfg(unix)]
impl Drop for ProcessGroupGuard {
    fn drop(&mut self) {
        self.terminate();
    }
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

fn append_truncation_notice(text: &mut String, truncated: bool) {
    if truncated {
        text.push_str("\n\n[preview output truncated]");
    }
}

fn should_show_truncated_stdout(output: &CommandOutput) -> bool {
    output.stdout_truncated && !output.stdout.is_empty()
}

#[cfg(test)]
mod tests {
    use super::{
        CommandOutput, append_truncation_notice, expand_preview_command, run_preview_command,
        should_show_truncated_stdout,
    };

    #[test]
    fn command_expansion_uses_environment_variables() {
        let command = expand_preview_command("printf '%s %s %s' {} {q} {n}")
            .expect("unquoted placeholders should expand");

        assert_eq!(
            command,
            "printf '%s %s %s' \"$FSEL_PREVIEW_ITEM\" \"$FSEL_PREVIEW_QUERY\" \"$FSEL_PREVIEW_ORDINAL\""
        );
    }

    #[test]
    fn nested_double_quotes_never_interpolate_row_source() {
        let command = expand_preview_command("echo \"$(printf \"{}\")\"")
            .expect("positional parameters make double-quoted expansion safe");

        assert_eq!(command, "echo \"$(printf \"\"$FSEL_PREVIEW_ITEM\"\")\"");
    }

    #[test]
    fn single_quoted_placeholders_are_rejected() {
        let result = expand_preview_command("printf %s '{}'");

        assert!(result.is_err());
    }

    #[test]
    fn heredoc_preview_commands_are_rejected() {
        let result = expand_preview_command("sh <<EOF\necho {}\nEOF");

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn selected_item_is_not_reparsed_as_shell_source() {
        let command =
            expand_preview_command("printf '%s' {}").expect("unquoted placeholder should expand");
        let payload = "$(printf injected >&2)";

        let output = run_preview_command(&command, payload, "", 0)
            .await
            .expect("preview command should run");

        assert!(output.success);
        assert_eq!(output.stdout, payload.as_bytes());
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn truncation_notice_is_added_to_failed_command_diagnostics() {
        let mut text = "command failed".to_string();

        append_truncation_notice(&mut text, true);

        assert_eq!(text, "command failed\n\n[preview output truncated]");
    }

    #[test]
    fn truncated_stdout_survives_a_producer_sigpipe_failure() {
        let output = CommandOutput {
            stdout: b"captured prefix".to_vec(),
            stderr: Vec::new(),
            status: "signal: 13 (SIGPIPE)".to_string(),
            success: false,
            truncated: true,
            stdout_truncated: true,
        };

        assert!(should_show_truncated_stdout(&output));
    }
}
