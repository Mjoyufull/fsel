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

        if should_report_command_failure(&output) {
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
        if let Some(message) = truncated_image_message(&output) {
            self.content = PreviewContent::Text(message);
            return;
        }
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
    let mut command = String::with_capacity(template.len() + 16);
    let mut remaining = template;
    let mut quote = ShellQuote::Unquoted;
    let mut escaped = false;
    let mut substitutions = Vec::<CommandSubstitution>::new();

    while !remaining.is_empty() {
        let arithmetic_context = substitutions
            .last()
            .is_some_and(|substitution| substitution.arithmetic);
        if !escaped
            && quote == ShellQuote::Unquoted
            && !arithmetic_context
            && remaining.starts_with("<<")
        {
            return Err("Preview command heredocs are not supported".to_string());
        } else if !escaped && let Some(rest) = remaining.strip_prefix("{}") {
            append_placeholder(&mut command, quote, "FSEL_PREVIEW_ITEM")?;
            remaining = rest;
        } else if !escaped && let Some(rest) = remaining.strip_prefix("{q}") {
            append_placeholder(&mut command, quote, "FSEL_PREVIEW_QUERY")?;
            remaining = rest;
        } else if !escaped && let Some(rest) = remaining.strip_prefix("{n}") {
            if arithmetic_context {
                command.push_str("FSEL_PREVIEW_ORDINAL");
            } else {
                append_placeholder(&mut command, quote, "FSEL_PREVIEW_ORDINAL")?;
            }
            remaining = rest;
        } else if !escaped
            && quote != ShellQuote::Single
            && let Some(rest) = remaining.strip_prefix("$(")
        {
            command.push_str("$(");
            if let Some(parent) = substitutions.last_mut() {
                parent.command_position = false;
            }
            substitutions.push(CommandSubstitution {
                outer_quote: quote,
                nested_parentheses: 0,
                case_patterns: Vec::new(),
                arithmetic: remaining.starts_with("$(("),
                command_position: true,
            });
            quote = ShellQuote::Unquoted;
            remaining = rest;
        } else if !escaped
            && quote == ShellQuote::Unquoted
            && substitutions.last().is_some_and(|substitution| {
                substitution.command_position && !substitution.arithmetic
            })
            && let Some(keyword) = shell_control_keyword(template, remaining)
        {
            command.push_str(keyword);
            remaining = &remaining[keyword.len()..];
            let substitution = substitutions
                .last_mut()
                .expect("substitution stack is non-empty");
            match keyword {
                "case" => {
                    substitution.case_patterns.push(true);
                    substitution.command_position = false;
                }
                "esac" => {
                    substitution.case_patterns.pop();
                    substitution.command_position = false;
                }
                "then" | "do" | "else" | "elif" => {
                    substitution.command_position = true;
                }
                _ => unreachable!("all recognized control keywords are handled"),
            }
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
                (ShellQuote::Unquoted, '(') if !substitutions.is_empty() => {
                    substitutions
                        .last_mut()
                        .expect("substitution stack is non-empty")
                        .nested_parentheses += 1;
                }
                (ShellQuote::Unquoted, ')') if !substitutions.is_empty() => {
                    let substitution = substitutions
                        .last_mut()
                        .expect("substitution stack is non-empty");
                    if substitution.case_patterns.last() == Some(&true) {
                        // A case pattern terminator belongs to the case grammar,
                        // not to the surrounding command substitution.
                        if let Some(in_pattern) = substitution.case_patterns.last_mut() {
                            *in_pattern = false;
                        }
                        substitution.command_position = true;
                        continue;
                    } else if substitution.nested_parentheses == 0 {
                        quote = substitutions
                            .pop()
                            .expect("substitution stack is non-empty")
                            .outer_quote;
                    } else {
                        substitution.nested_parentheses -= 1;
                    }
                }
                _ => {}
            }
            if quote == ShellQuote::Unquoted
                && let Some(substitution) = substitutions.last_mut()
            {
                match character {
                    ';' | '|' | '&' | '\n' => {
                        substitution.command_position = true;
                        if character == ';'
                            && remaining.starts_with(';')
                            && let Some(in_pattern) = substitution.case_patterns.last_mut()
                        {
                            *in_pattern = true;
                        }
                    }
                    '(' | '{' if substitution.command_position => {
                        substitution.command_position = true;
                    }
                    character if character.is_whitespace() => {}
                    _ => substitution.command_position = false,
                }
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

struct CommandSubstitution {
    outer_quote: ShellQuote,
    nested_parentheses: usize,
    case_patterns: Vec<bool>,
    arithmetic: bool,
    command_position: bool,
}

fn shell_control_keyword(template: &str, remaining: &str) -> Option<&'static str> {
    ["case", "esac", "then", "do", "else", "elif"]
        .into_iter()
        .find(|keyword| starts_shell_keyword(template, remaining, keyword))
}

fn starts_shell_keyword(template: &str, remaining: &str, keyword: &str) -> bool {
    let Some(after_keyword) = remaining.strip_prefix(keyword) else {
        return false;
    };
    let offset = template.len().saturating_sub(remaining.len());
    let previous = template[..offset].chars().next_back();
    let next = after_keyword.chars().next();
    previous.is_none_or(is_shell_word_boundary) && next.is_none_or(is_shell_word_boundary)
}

fn is_shell_word_boundary(character: char) -> bool {
    !character.is_alphanumeric() && character != '_'
}

fn append_placeholder(
    command: &mut String,
    quote: ShellQuote,
    variable: &str,
) -> Result<(), String> {
    match quote {
        ShellQuote::Unquoted => command.push_str(&format!("\"${variable}\"")),
        // Empty adjacent quotes terminate the variable name without changing the
        // surrounding double-quoted context. This works in POSIX shells and fish.
        ShellQuote::Double => command.push_str(&format!("${variable}\"\"")),
        ShellQuote::Single => {
            return Err("Preview placeholders must not appear inside single quotes".to_string());
        }
    }
    Ok(())
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

    let (limit_tx, mut limit_rx) = mpsc::unbounded_channel();
    let stdout_task = tokio::spawn(read_limited(stdout, limit_tx.clone()));
    let stderr_task = tokio::spawn(read_limited(stderr, limit_tx.clone()));
    drop(limit_tx);
    let status = tokio::select! {
        status = child.wait() => status,
        Some(()) = limit_rx.recv() => {
            #[cfg(unix)]
            process_group.terminate();
            let _ = child.start_kill();
            child.wait().await
        }
    };
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

async fn read_limited(
    reader: impl AsyncRead + Unpin,
    limit_tx: mpsc::UnboundedSender<()>,
) -> std::io::Result<(Vec<u8>, bool)> {
    read_limited_to(reader, MAX_PREVIEW_BYTES, limit_tx).await
}

async fn read_limited_to(
    mut reader: impl AsyncRead + Unpin,
    limit: u64,
    limit_tx: mpsc::UnboundedSender<()>,
) -> std::io::Result<(Vec<u8>, bool)> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 8192];
    let mut truncated = false;
    loop {
        let read = reader.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        let remaining = limit.saturating_sub(bytes.len() as u64) as usize;
        bytes.extend_from_slice(&buffer[..read.min(remaining)]);
        if read > remaining && !truncated {
            truncated = true;
            let _ = limit_tx.send(());
        }
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

fn truncated_image_message(output: &CommandOutput) -> Option<String> {
    (output.truncated && image::guess_format(&output.stdout).is_ok()).then(|| {
        format!(
            "Preview image exceeds the {} MiB output limit",
            MAX_PREVIEW_BYTES / (1024 * 1024)
        )
    })
}

fn should_report_command_failure(output: &CommandOutput) -> bool {
    !output.success && output.stdout.is_empty()
}

#[cfg(test)]
mod tests {
    use super::{
        CommandOutput, append_truncation_notice, expand_preview_command, read_limited_to,
        run_preview_command, should_report_command_failure, truncated_image_message,
    };
    use tokio::io::AsyncWriteExt;

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
    fn nested_double_quotes_preserve_shell_context() {
        let command = expand_preview_command("echo \"$(printf \"{}\")\"")
            .expect("nested double-quoted placeholders should expand safely");

        assert_eq!(command, "echo \"$(printf \"$FSEL_PREVIEW_ITEM\"\"\")\"");
    }

    #[test]
    fn nested_single_quoted_placeholders_are_rejected() {
        let result = expand_preview_command("echo \"$(printf '{}')\"");

        assert!(result.is_err());
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

    #[test]
    fn quoted_shift_operators_are_not_treated_as_heredocs() {
        let command = expand_preview_command("python -c 'print(1 << 8)' {}")
            .expect("quoted shift operators are ordinary command data");

        assert!(command.contains("print(1 << 8)"));
    }

    #[test]
    fn arithmetic_shift_operators_are_not_treated_as_heredocs() {
        let command = expand_preview_command("printf '%s' $((1 << 8)) {}")
            .expect("arithmetic shifts are not heredocs");

        assert!(command.contains("$((1 << 8))"));
    }

    #[tokio::test]
    async fn arithmetic_ordinal_placeholder_uses_an_unquoted_variable() {
        let command = expand_preview_command("printf '%s' $(({n}+1))")
            .expect("ordinal placeholders should expand in arithmetic contexts");

        assert_eq!(command, "printf '%s' $((FSEL_PREVIEW_ORDINAL+1))");
        let output = tokio::process::Command::new("/bin/sh")
            .args(["-c", &command])
            .env("FSEL_PREVIEW_ORDINAL", "4")
            .output()
            .await
            .expect("POSIX preview command should run");

        assert!(output.status.success());
        assert_eq!(output.stdout, b"5");
    }

    #[test]
    fn case_patterns_do_not_close_command_substitutions() {
        let command = expand_preview_command("echo \"$(case x in x) printf '%s' \"{}\";; esac)\"")
            .expect("case pattern terminators belong to the case clause");

        assert!(command.contains("$FSEL_PREVIEW_ITEM"));
    }

    #[tokio::test]
    async fn case_as_an_argument_does_not_change_shell_context() {
        let command = expand_preview_command("printf '<%s>' \"$(printf case){}\"")
            .expect("an ordinary case argument is not case grammar");
        let payload = "two words*.txt";

        let output = run_preview_command(&command, payload, "", 0)
            .await
            .expect("preview command should run");

        assert!(output.success);
        assert_eq!(output.stdout, b"<casetwo words*.txt>");
    }

    #[tokio::test]
    async fn grouped_case_preserves_placeholder_quoting() {
        let command =
            expand_preview_command("printf '<%s>' \"$( (case x in x) :;; esac); printf '%s' {})\"")
                .expect("a grouped case command should keep substitution context");
        let payload = "two words*.txt";

        let output = tokio::process::Command::new("/bin/sh")
            .args(["-c", &command])
            .env("FSEL_PREVIEW_ITEM", payload)
            .env("FSEL_PREVIEW_QUERY", "")
            .env("FSEL_PREVIEW_ORDINAL", "0")
            .output()
            .await
            .expect("POSIX preview command should run");

        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"<two words*.txt>");
    }

    #[tokio::test]
    async fn grouping_inside_case_preserves_outer_quote_context() {
        let command = expand_preview_command("printf '<%s>' \"$(case x in x) ( : );; esac){}\"")
            .expect("a group inside a case body should not close the substitution");
        let payload = "two words*.txt";

        let output = tokio::process::Command::new("/bin/sh")
            .args(["-c", &command])
            .env("FSEL_PREVIEW_ITEM", payload)
            .output()
            .await
            .expect("POSIX preview command should run");

        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"<two words*.txt>");
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

    #[tokio::test]
    async fn double_quoted_placeholder_preserves_one_argument() {
        let command = expand_preview_command("printf '<%s>' \"{}\"")
            .expect("double-quoted placeholder should expand");
        let payload = "two words*.txt";

        let output = run_preview_command(&command, payload, "", 0)
            .await
            .expect("preview command should run");

        assert!(output.success);
        assert_eq!(output.stdout, b"<two words*.txt>");
    }

    #[tokio::test]
    async fn nested_double_quoted_placeholder_expands_the_row() {
        let command = expand_preview_command("printf '<%s>' \"$(printf '%s' \"{}\")\"")
            .expect("nested double-quoted placeholder should expand");
        let payload = "two words*.txt";

        let output = run_preview_command(&command, payload, "", 0)
            .await
            .expect("preview command should run");

        assert!(output.success);
        assert_eq!(output.stdout, b"<two words*.txt>");
    }

    #[tokio::test]
    async fn limited_reader_drains_after_reporting_the_cap() {
        let (mut writer, reader) = tokio::io::duplex(32);
        let writer_task = tokio::spawn(async move {
            writer
                .write_all(b"abcdef")
                .await
                .expect("write should work");
            writer.shutdown().await.expect("shutdown should work");
        });
        let (limit_tx, mut limit_rx) = tokio::sync::mpsc::unbounded_channel();

        let (bytes, truncated) = read_limited_to(reader, 3, limit_tx)
            .await
            .expect("read should work");
        writer_task.await.expect("writer should finish");

        assert_eq!(bytes, b"abc");
        assert!(truncated);
        assert!(limit_rx.try_recv().is_ok());
    }

    #[test]
    fn truncation_notice_is_added_to_failed_command_diagnostics() {
        let mut text = "command failed".to_string();

        append_truncation_notice(&mut text, true);

        assert_eq!(text, "command failed\n\n[preview output truncated]");
    }

    #[test]
    fn truncated_images_are_reported_before_decode() {
        let output = CommandOutput {
            stdout: b"\x89PNG\r\n\x1a\npartial".to_vec(),
            stderr: Vec::new(),
            status: "signal: 9".to_string(),
            success: false,
            truncated: true,
        };

        assert_eq!(
            truncated_image_message(&output).as_deref(),
            Some("Preview image exceeds the 32 MiB output limit")
        );
    }

    #[test]
    fn nonzero_commands_keep_nonempty_stdout() {
        let output = CommandOutput {
            stdout: b"useful diff".to_vec(),
            stderr: Vec::new(),
            status: "exit status: 1".to_string(),
            success: false,
            truncated: false,
        };

        assert!(!should_report_command_failure(&output));
    }
}
