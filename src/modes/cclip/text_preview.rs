//! Scrollable fullscreen text for the selected clipboard entry.

use super::events::EventContext;
use crate::ui::{AsyncInput, InputEvent, Keybinds};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEventKind};
use eyre::Result;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;

pub(super) async fn show(ctx: &mut EventContext<'_, '_>, input: &mut AsyncInput) -> Result<()> {
    let Some(item) = ctx
        .ui
        .selected
        .and_then(|index| ctx.ui.shown.get(index))
        .cloned()
    else {
        return Ok(());
    };
    let mime = item.original_line.split('\t').nth(1).unwrap_or("");
    if !super::html::is_textual_mime(mime) {
        return Ok(());
    }

    ctx.image_runtime.clear_inline_image();
    let result = run(ctx, input, &item).await;
    crate::ui::terminal::clear_fullscreen(ctx.terminal)?;
    ctx.image_runtime.request_buffer_sync();
    result
}

async fn run(
    ctx: &mut EventContext<'_, '_>,
    input: &mut AsyncInput,
    item: &crate::common::Item,
) -> Result<()> {
    let mut pager = Pager::default();
    loop {
        let content = ctx.ui.get_cclip_content_for_display(item);
        let clean = clean_text(&content);
        let lines: Vec<&str> = clean.split('\n').collect();
        let mut page_height = 1;
        ctx.terminal.draw(|frame| {
            page_height = draw(frame, &mut pager, &lines, ctx.options);
        })?;
        tokio::select! {
            _ = ctx.ui.wait_for_cclip_content(), if ctx.ui.has_cclip_content_activity() => {}
            event = input.next() => match event {
                Some(InputEvent::Input(key)) => {
                    if pager.handle_key(key, &ctx.options.keybinds, lines.len(), page_height) {
                        break;
                    }
                }
                Some(InputEvent::Mouse(mouse)) => match mouse.kind {
                    MouseEventKind::ScrollDown => pager.top = pager.top.saturating_add(3),
                    MouseEventKind::ScrollUp => pager.top = pager.top.saturating_sub(3),
                    _ => {}
                },
                None => break,
                _ => {}
            }
        }
    }
    Ok(())
}

fn draw(
    frame: &mut ratatui::Frame,
    pager: &mut Pager,
    lines: &[&str],
    options: &super::state::CclipOptions,
) -> usize {
    let area = frame.area();
    let body = Rect {
        height: area.height.saturating_sub(1),
        ..area
    };
    let page_height = usize::from(body.height).max(1);
    pager.clamp(lines.len(), page_height);
    let text = lines
        .iter()
        .skip(pager.top)
        .take(page_height)
        .copied()
        .collect::<Vec<_>>()
        .join("\n");
    frame.render_widget(
        Paragraph::new(text).scroll((0, pager.left)).style(
            Style::default()
                .fg(options.main_text_color)
                .bg(options.main_background_color),
        ),
        body,
    );
    if area.height > 0 {
        let status = format!(
            " {}-{}/{}  j/k: scroll  Space/b: page  g/G: ends  ←/→: pan  q: back ",
            pager.top + 1,
            (pager.top + page_height).min(lines.len()),
            lines.len(),
        );
        frame.render_widget(
            Paragraph::new(status).style(Style::default().fg(options.highlight_color)),
            Rect::new(area.x, area.y + area.height - 1, area.width, 1),
        );
    }
    page_height
}

fn clean_text(content: &str) -> String {
    strip_ansi_escapes::strip_str(content.replace('\t', "    "))
        .chars()
        .filter(|character| *character == '\n' || !character.is_control())
        .collect()
}

#[derive(Default)]
struct Pager {
    top: usize,
    left: u16,
}

impl Pager {
    fn clamp(&mut self, lines: usize, height: usize) {
        self.top = self.top.min(lines.saturating_sub(height));
    }

    /// Return true when the key closes the preview without changing selection.
    fn handle_key(&mut self, key: KeyEvent, keys: &Keybinds, lines: usize, height: usize) -> bool {
        if keys.matches_exit(key.code, key.modifiers)
            || keys.matches_image_preview(key.code, key.modifiers)
            || key.code == KeyCode::Esc
            || key.code == KeyCode::Char('q')
            || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
        {
            return true;
        }
        if keys.matches_down(key.code, key.modifiers) {
            self.top = self.top.saturating_add(1);
        } else if keys.matches_up(key.code, key.modifiers) {
            self.top = self.top.saturating_sub(1);
        } else {
            match key.code {
                KeyCode::Down | KeyCode::Char('j') => self.top = self.top.saturating_add(1),
                KeyCode::Up | KeyCode::Char('k') => self.top = self.top.saturating_sub(1),
                KeyCode::PageDown | KeyCode::Char(' ') | KeyCode::Char('f') => {
                    self.top = self.top.saturating_add(height);
                }
                KeyCode::PageUp | KeyCode::Char('b') => self.top = self.top.saturating_sub(height),
                KeyCode::Home | KeyCode::Char('g') => self.top = 0,
                KeyCode::End | KeyCode::Char('G') => self.top = lines.saturating_sub(height),
                KeyCode::Right | KeyCode::Char('l') => self.left = self.left.saturating_add(4),
                KeyCode::Left | KeyCode::Char('h') => self.left = self.left.saturating_sub(4),
                _ => {}
            }
        }
        self.clamp(lines, height);
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_text_preserves_lines_and_indentation_without_terminal_escapes() {
        let content = format!("\x1b[31mheading\x1b[0m\n\t{}\nend\0", "x".repeat(6000));
        assert_eq!(
            clean_text(&content),
            format!("heading\n    {}\nend", "x".repeat(6000))
        );
    }

    #[test]
    fn paging_clamps_at_ends_and_after_resize() {
        let mut pager = Pager::default();
        let keys = Keybinds::default();
        for (code, expected) in [
            (KeyCode::PageDown, 20),
            (KeyCode::End, 75),
            (KeyCode::Down, 75),
            (KeyCode::PageUp, 55),
            (KeyCode::Home, 0),
        ] {
            assert!(!pager.handle_key(KeyEvent::new(code, KeyModifiers::NONE), &keys, 95, 20));
            assert_eq!(pager.top, expected);
        }
        pager.top = 75;
        pager.clamp(95, 90);
        assert_eq!(pager.top, 5);
        pager.clamp(0, 1);
        assert_eq!(pager.top, 0);
    }

    #[test]
    fn configured_preview_key_returns_without_scrolling() {
        let keys: Keybinds = toml::from_str("image_preview = [{ key = 'p', modifiers = 'alt' }]")
            .expect("valid keybinds");
        let mut pager = Pager { top: 8, left: 4 };
        assert!(pager.handle_key(
            KeyEvent::new(KeyCode::Char('p'), KeyModifiers::ALT),
            &keys,
            50,
            10
        ));
        assert_eq!(pager.top, 8);
        assert_eq!(pager.left, 4);
    }
}
