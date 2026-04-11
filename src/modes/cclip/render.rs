use super::TagMetadataFormatter;
use super::image::ImageRuntime;
use super::state::CclipOptions;
use crate::ui::DmenuUI;
use eyre::Result;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap,
};
use std::io;
use std::io::Write;

pub(super) fn draw(
    terminal: &mut Terminal<CrosstermBackend<io::Stderr>>,
    ui: &mut DmenuUI<'_>,
    options: &CclipOptions,
    tag_metadata_formatter: &TagMetadataFormatter,
    list_state: &mut ListState,
    image_runtime: &mut ImageRuntime,
) -> Result<usize> {
    if options.term_is_foot {
        let mut stderr = std::io::stderr();
        let _ = stderr.write_all(b"\x1b[?2026h");
        let _ = stderr.flush();
    }

    let mut max_visible = 0usize;
    let mut render_error = Ok(());
    let needs_sixel_clear = image_runtime.needs_terminal_clear();
    let force_buffer_sync = image_runtime.consume_buffer_sync();

    terminal.draw(|frame| {
        let content_height = options.content_height(frame.area().height);
        let show_content_panel = content_height > 0;
        let layout = options.split_layout(frame.area());
        let chunks = layout.chunks;
        let content_panel_index = layout.content_panel_index;
        let items_panel_index = layout.items_panel_index;
        let input_panel_index = layout.input_panel_index;
        let content_panel_width = chunks[content_panel_index].width;
        let content_panel_height = chunks[content_panel_index].height.saturating_sub(2);

        let preview_enabled = image_runtime.preview_enabled();
        match &ui.tag_mode {
            crate::ui::TagMode::Normal => ui.info_with_image_support(
                options.highlight_color,
                preview_enabled,
                options.hide_image_message,
                content_panel_width,
                content_panel_height,
            ),
            _ => ui.info_with_image_support(
                options.highlight_color,
                false,
                options.hide_image_message,
                content_panel_width,
                content_panel_height,
            ),
        }

        let border_type = if options.rounded_borders {
            BorderType::Rounded
        } else {
            BorderType::Plain
        };

        let content_block = panel_block(
            " Clipboard Preview ",
            border_type,
            options.header_title_color,
            options.main_border_color,
        );
        let content_paragraph = Paragraph::new(ui.text.clone())
            .block(content_block)
            .style(Style::default().fg(options.main_text_color))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false })
            .scroll((0, 0));

        max_visible = chunks[items_panel_index].height.saturating_sub(2) as usize;
        let visible_items = ui
            .shown
            .iter()
            .skip(ui.scroll_offset)
            .take(max_visible)
            .map(|item| item.to_list_item(Some(tag_metadata_formatter)))
            .collect::<Vec<ListItem>>();

        let items_list = List::new(visible_items)
            .block(panel_block(
                " Clipboard History ",
                border_type,
                options.header_title_color,
                options.items_border_color,
            ))
            .style(Style::default().fg(options.items_text_color))
            .highlight_style(highlight_style(
                ui,
                tag_metadata_formatter,
                options.highlight_color,
            ))
            .highlight_symbol("> ");

        let visible_selection = ui.selected.and_then(|selected| {
            if selected >= ui.scroll_offset && selected < ui.scroll_offset + max_visible {
                Some(selected - ui.scroll_offset)
            } else {
                None
            }
        });
        list_state.select(visible_selection);

        let (input_line, input_title) = input_line_and_title(ui, options);
        let input_paragraph = Paragraph::new(input_line)
            .block(panel_block(
                input_title,
                border_type,
                options.header_title_color,
                options.input_border_color,
            ))
            .style(Style::default().fg(options.input_text_color))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false });

        let is_kitty = matches!(options.graphics_adapter, crate::ui::GraphicsAdapter::Kitty);
        if is_kitty || needs_sixel_clear || force_buffer_sync {
            frame.render_widget(Clear, chunks[content_panel_index]);
            frame.render_widget(Clear, chunks[items_panel_index]);
            frame.render_widget(Clear, chunks[input_panel_index]);
        }

        let image_rendered =
            if show_content_panel && preview_enabled && image_runtime.current_is_image() {
                render_inline_image(
                    frame,
                    image_runtime,
                    chunks[content_panel_index],
                    &mut render_error,
                )
            } else {
                false
            };

        if show_content_panel {
            if image_rendered {
                frame.render_widget(
                    panel_block(
                        " Clipboard Preview ",
                        border_type,
                        options.header_title_color,
                        options.main_border_color,
                    ),
                    chunks[content_panel_index],
                );
            } else {
                frame.render_widget(content_paragraph, chunks[content_panel_index]);
            }
        }

        frame.render_stateful_widget(items_list, chunks[items_panel_index], list_state);
        frame.render_widget(input_paragraph, chunks[input_panel_index]);
    })?;
    render_error?;

    if options.term_is_foot {
        let mut stderr = std::io::stderr();
        let _ = stderr.write_all(b"\x1b[?2026l");
        let _ = stderr.flush();
    }

    image_runtime.finish_draw();
    Ok(max_visible)
}

fn render_inline_image(
    frame: &mut ratatui::Frame,
    image_runtime: &mut ImageRuntime,
    content_chunk: Rect,
    render_error: &mut Result<()>,
) -> bool {
    let image_area = Rect {
        x: content_chunk.x + 1,
        y: content_chunk.y + 1,
        width: content_chunk.width.saturating_sub(2),
        height: content_chunk.height.saturating_sub(2),
    };

    match image_runtime.render_inline_image(frame, image_area) {
        Ok(rendered) => rendered,
        Err(error) => {
            *render_error = Err(error);
            false
        }
    }
}

fn panel_block(
    title: &'static str,
    border_type: BorderType,
    title_color: ratatui::style::Color,
    border_color: ratatui::style::Color,
) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            title,
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(title_color),
        ))
        .border_type(border_type)
        .border_style(Style::default().fg(border_color))
}

fn highlight_style(
    ui: &DmenuUI<'_>,
    formatter: &TagMetadataFormatter,
    highlight_color: ratatui::style::Color,
) -> Style {
    let tag_color = ui.selected.and_then(|selected| {
        if selected < ui.shown.len() {
            ui.shown[selected]
                .tags
                .as_ref()
                .and_then(|tags| tags.first())
                .and_then(|tag| formatter.get_color(tag))
        } else {
            None
        }
    });

    Style::default()
        .fg(tag_color.unwrap_or(highlight_color))
        .add_modifier(Modifier::BOLD)
}

fn input_line_and_title(ui: &DmenuUI<'_>, options: &CclipOptions) -> (Line<'static>, &'static str) {
    match &ui.tag_mode {
        crate::ui::TagMode::PromptingTagName { input, .. } => (
            Line::from(vec![
                Span::styled("Tag: ", Style::default().fg(options.highlight_color)),
                Span::styled(input.clone(), Style::default().fg(options.input_text_color)),
                Span::styled(
                    options.cursor.clone(),
                    Style::default().fg(options.highlight_color),
                ),
            ]),
            " Tag Name ",
        ),
        crate::ui::TagMode::PromptingTagEmoji { input, .. } => (
            Line::from(vec![
                Span::styled("Emoji: ", Style::default().fg(options.highlight_color)),
                Span::styled(input.clone(), Style::default().fg(options.input_text_color)),
                Span::styled(
                    options.cursor.clone(),
                    Style::default().fg(options.highlight_color),
                ),
                Span::styled(
                    " (or blank)",
                    Style::default()
                        .fg(options.input_text_color)
                        .add_modifier(Modifier::DIM),
                ),
            ]),
            " Tag Emoji ",
        ),
        crate::ui::TagMode::PromptingTagColor { input, .. } => (
            Line::from(vec![
                Span::styled("Color: ", Style::default().fg(options.highlight_color)),
                Span::styled(input.clone(), Style::default().fg(options.input_text_color)),
                Span::styled(
                    options.cursor.clone(),
                    Style::default().fg(options.highlight_color),
                ),
                Span::styled(
                    " (hex/name or blank)",
                    Style::default()
                        .fg(options.input_text_color)
                        .add_modifier(Modifier::DIM),
                ),
            ]),
            " Tag Color ",
        ),
        crate::ui::TagMode::RemovingTag { input, .. } => (
            Line::from(vec![
                Span::styled("Remove: ", Style::default().fg(options.highlight_color)),
                Span::styled(input.clone(), Style::default().fg(options.input_text_color)),
                Span::styled(
                    options.cursor.clone(),
                    Style::default().fg(options.highlight_color),
                ),
                Span::styled(
                    " (blank = all)",
                    Style::default()
                        .fg(options.input_text_color)
                        .add_modifier(Modifier::DIM),
                ),
            ]),
            " Remove Tag ",
        ),
        crate::ui::TagMode::Normal => (
            Line::from(vec![
                Span::styled("(", Style::default().fg(options.input_text_color)),
                Span::styled(
                    ui.selected.map_or(0, |selected| selected + 1).to_string(),
                    Style::default().fg(options.highlight_color),
                ),
                Span::styled("/", Style::default().fg(options.input_text_color)),
                Span::styled(
                    ui.shown.len().to_string(),
                    Style::default().fg(options.input_text_color),
                ),
                Span::styled(") ", Style::default().fg(options.input_text_color)),
                Span::styled(">", Style::default().fg(options.highlight_color)),
                Span::styled("> ", Style::default().fg(options.input_text_color)),
                Span::styled(
                    ui.query.clone(),
                    Style::default().fg(options.input_text_color),
                ),
                Span::styled(
                    options.cursor.clone(),
                    Style::default().fg(options.highlight_color),
                ),
            ]),
            " Filter ",
        ),
    }
}
