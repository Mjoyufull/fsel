//! Bounded TTY capability queries before input starts; stdin/stdout remain pipeline data.

use ratatui_image::picker::Picker;

pub(crate) fn query_terminal(fallback: Picker) -> Picker {
    #[cfg(unix)]
    {
        query_tty(&fallback).unwrap_or(fallback)
    }
    #[cfg(not(unix))]
    fallback
}

#[cfg(unix)]
fn query_tty(fallback: &Picker) -> std::io::Result<Picker> {
    use ratatui_image::picker::ProtocolType;
    use ratatui_image::picker::cap_parser::{Parser, QueryStdioOptions, Response};
    use rustix::event::{PollFd, PollFlags, Timespec, poll};
    use std::fs::OpenOptions;
    use std::io::{Read, Write};
    use std::time::{Duration, Instant};

    let mut tty = OpenOptions::new().read(true).write(true).open("/dev/tty")?;
    let mut options = QueryStdioOptions::default();
    let blacklist = ["WEZTERM_EXECUTABLE", "KONSOLE_VERSION"]
        .iter()
        .any(|name| std::env::var(name).is_ok_and(|value| !value.is_empty()));
    if blacklist {
        options.blacklist_protocols = vec![ProtocolType::Kitty, ProtocolType::Sixel];
    }
    let tmux = std::env::var_os("TMUX").is_some();
    tty.write_all(Parser::query(tmux, options).as_bytes())?;
    tty.flush()?;
    let deadline = Instant::now() + Duration::from_millis(500);
    let mut parser = Parser::new();
    let mut protocol = None;
    let mut font = fallback.font_size();
    if let Ok(size) = crossterm::terminal::window_size()
        && size.columns > 0
        && size.rows > 0
        && size.width >= size.columns
        && size.height >= size.rows
    {
        font = ratatui_image::FontSize::new(size.width / size.columns, size.height / size.rows);
    }
    'query: while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        let timeout = Timespec {
            tv_sec: 0,
            tv_nsec: remaining.as_nanos().min(500_000_000) as _,
        };
        let mut fds = [PollFd::new(&tty, PollFlags::IN)];
        if poll(&mut fds, Some(&timeout))? == 0 {
            break;
        }
        let mut bytes = [0u8; 128];
        let count = tty.read(&mut bytes)?;
        if count == 0 {
            break;
        }
        for byte in &bytes[..count] {
            for response in parser.push(char::from(*byte)) {
                match response {
                    Response::Kitty if !blacklist => protocol = Some(ProtocolType::Kitty),
                    Response::Sixel if !blacklist && protocol.is_none() => {
                        protocol = Some(ProtocolType::Sixel)
                    }
                    Response::CellSize(Some((width, height))) if width > 0 && height > 0 => {
                        font = ratatui_image::FontSize::new(width, height);
                    }
                    Response::Status => break 'query,
                    _ => {}
                }
            }
        }
    }
    Ok(picker_with_font(
        font,
        protocol.unwrap_or(fallback.protocol_type()),
    ))
}

#[cfg(unix)]
#[expect(
    deprecated,
    reason = "ratatui-image's only constructor for an externally queried cell size"
)]
fn picker_with_font(
    font: ratatui_image::FontSize,
    protocol: ratatui_image::picker::ProtocolType,
) -> Picker {
    let mut picker = Picker::from_fontsize(font);
    picker.set_protocol_type(protocol);
    picker
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use ratatui_image::{FontSize, picker::ProtocolType};

    #[test]
    fn queried_sixel_and_cell_size_are_preserved() {
        let picker = picker_with_font(FontSize::new(9, 18), ProtocolType::Sixel);
        assert_eq!(picker.protocol_type(), ProtocolType::Sixel);
        assert_eq!(picker.font_size().width, 9);
        assert_eq!(picker.font_size().height, 18);
    }
}
