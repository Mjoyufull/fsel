use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap,
};

use crate::ui::DmenuUI;

use super::options::DmenuOptions;

pub(super) fn draw_frame(
    frame: &mut Frame,
    ui: &mut DmenuUI,
    list_state: &mut ListState,
    options: &DmenuOptions,
) {
    let layout = options.split_layout(frame.area());
    let chunks = layout.chunks;
    let content_panel_index = layout.content_panel_index;
    let items_panel_index = layout.items_panel_index;
    let input_panel_index = layout.input_panel_index;
    let show_content_panel = options.content_height(frame.area().height) > 0;

    let border_type = if options.rounded_borders {
        BorderType::Rounded
    } else {
        BorderType::Plain
    };

    let content_block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Content ",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(options.header_title_color),
        ))
        .border_type(border_type)
        .border_style(Style::default().fg(options.main_border_color));

    ui.info_with_image_support(
        options.highlight_color,
        false,
        false,
        chunks[content_panel_index].width,
        chunks[content_panel_index].height.saturating_sub(2),
    );

    let content_paragraph = Paragraph::new(ui.text.clone())
        .block(content_block)
        .style(Style::default().fg(options.main_text_color))
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Left);

    let items_panel_height = chunks[items_panel_index].height;
    let max_visible = items_panel_height.saturating_sub(2) as usize;
    let visible_items = ui
        .shown
        .iter()
        .skip(ui.scroll_offset)
        .take(max_visible)
        .map(ListItem::from)
        .collect::<Vec<ListItem>>();

    let items_list = List::new(visible_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    " Items ",
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(options.header_title_color),
                ))
                .border_type(border_type)
                .border_style(Style::default().fg(options.items_border_color)),
        )
        .style(Style::default().fg(options.items_text_color))
        .highlight_style(
            Style::default()
                .fg(options.highlight_color)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let visible_selection = ui.selected.and_then(|selected| {
        if selected >= ui.scroll_offset && selected < ui.scroll_offset + max_visible {
            Some(selected - ui.scroll_offset)
        } else {
            None
        }
    });
    list_state.select(visible_selection);

    let input_paragraph = Paragraph::new(Line::from(vec![
        Span::styled("(", Style::default().fg(options.input_text_color)),
        Span::styled(
            (ui.selected.map_or(0, |index| index + 1)).to_string(),
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
            options.display_query(&ui.query),
            Style::default().fg(options.input_text_color),
        ),
        Span::styled(
            &options.cursor,
            Style::default().fg(options.highlight_color),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(
                options.input_title(),
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(options.header_title_color),
            ))
            .border_type(border_type)
            .border_style(Style::default().fg(options.input_border_color)),
    )
    .style(Style::default().fg(options.input_text_color))
    .alignment(Alignment::Left)
    .wrap(Wrap { trim: false });

    if matches!(options.graphics_adapter, crate::ui::GraphicsAdapter::Kitty) {
        frame.render_widget(Clear, chunks[content_panel_index]);
        frame.render_widget(Clear, chunks[items_panel_index]);
        frame.render_widget(Clear, chunks[input_panel_index]);
    }

    if show_content_panel && (!options.hide_before_typing || !ui.query.is_empty()) {
        frame.render_widget(content_paragraph, chunks[content_panel_index]);
    }
    if !options.prompt_only && (!options.hide_before_typing || !ui.query.is_empty()) {
        frame.render_stateful_widget(items_list, chunks[items_panel_index], list_state);
    }
    frame.render_widget(input_paragraph, chunks[input_panel_index]);
}
