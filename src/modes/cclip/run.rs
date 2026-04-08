//! Cclip mode - main event loop and TUI
//!
//! This module contains the async run function that implements the clipboard
//! history browser with TUI interface.

use crate::cli::Opts;
use crate::ui::{DmenuUI, InputEvent as Event, TagMode};
use crossterm::event::{MouseButton, MouseEventKind};
use eyre::{Result, WrapErr};
use futures::FutureExt;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Alignment;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap};
use scopeguard::defer;
use std::collections::HashSet;
use std::io;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

use super::commands::{handle_noninteractive_mode, load_history, validate_environment};
use super::items::{build_items, reload_and_restore};
use super::state::CclipOptions;

fn copy_selected_and_exit(
    ui: &mut DmenuUI,
    index: usize,
    terminal: &mut Terminal<CrosstermBackend<io::Stderr>>,
    disable_mouse: bool,
) -> Result<bool> {
    if index >= ui.shown.len() {
        return Ok(false);
    }

    let original_line = &ui.shown[index].original_line;
    match super::CclipItem::from_line(original_line.clone()) {
        Ok(cclip_item) => {
            if let Err(error) = cclip_item.copy_to_clipboard() {
                ui.set_temp_message(format!("Copy failed: {}", error));
                return Ok(false);
            }

            terminal.show_cursor().wrap_err("Failed to show cursor")?;
            let _ = crate::ui::terminal::shutdown_terminal(disable_mouse);
            Ok(true)
        }
        Err(error) => {
            ui.set_temp_message(format!("Parse failed: {}", error));
            Ok(false)
        }
    }
}

fn reload_visible_history(
    ui: &mut DmenuUI,
    cli: &Opts,
    tag_metadata_formatter: &super::TagMetadataFormatter,
    show_line_numbers: bool,
    show_tag_color_names: bool,
    max_visible: usize,
) {
    let updated_items_res = if let Some(ref tag_name) = cli.cclip_tag {
        super::scan::get_clipboard_history_by_tag(tag_name)
    } else {
        super::scan::get_clipboard_history()
    };

    if let Ok(updated_items) = updated_items_res {
        reload_and_restore(
            ui,
            updated_items,
            tag_metadata_formatter,
            show_line_numbers,
            show_tag_color_names,
            max_visible,
        );
    }
}

/// Run cclip mode - async TUI event loop for clipboard history
pub async fn run(cli: &Opts) -> Result<()> {
    use crossterm::event::{KeyCode, KeyModifiers};

    validate_environment()?;
    if handle_noninteractive_mode(cli)? {
        return Ok(());
    }

    let cclip_items = load_history(cli)?;

    if cclip_items.is_empty() {
        if let Some(tag_name) = &cli.cclip_tag {
            println!("No clipboard items with tag '{}'", tag_name);
        } else {
            println!("No clipboard history available");
        }
        return Ok(());
    }

    let options = CclipOptions::from_cli(cli);

    // Load tag metadata for proper tag coloring
    let (db, _) = crate::core::database::open_history_db()?;
    let mut tag_metadata_map = super::load_tag_metadata(&db);
    let mut tag_metadata_formatter = super::TagMetadataFormatter::new(tag_metadata_map.clone());

    let items = build_items(
        cclip_items,
        &tag_metadata_formatter,
        options.show_line_numbers,
        options.show_tag_color_names,
    );
    crate::ui::terminal::setup_terminal(options.disable_mouse)?;

    // Ensure cleanup on exit
    defer! {
        let _ = crate::ui::terminal::shutdown_terminal(options.disable_mouse);
    }

    // Initialize terminal using stderr to keep stdout clean
    let backend = CrosstermBackend::new(io::stderr());
    let mut terminal = Terminal::new(backend).wrap_err("Failed to start crossterm terminal")?;
    terminal.hide_cursor().wrap_err("Failed to hide cursor")?;
    terminal.clear().wrap_err("Failed to clear terminal")?;

    // Initialize ImageManager for ratatui-image
    // Detect terminal capabilities ONCE
    let picker = ratatui_image::picker::Picker::from_query_stdio().ok();
    let picker_fallback = || ratatui_image::picker::Picker::halfblocks();

    let mut image_manager = Some(Arc::new(Mutex::new(crate::ui::ImageManager::new(
        picker.clone().unwrap_or_else(picker_fallback),
    ))));

    // Cclip does not need a synthetic render stream. Redraw on real input, resize, and
    // image-loader completions so large image previews do not flood the event queue.
    let mut input = options.input_config().init_async();

    let mut ui = DmenuUI::new(items, options.wrap_long_lines, options.show_line_numbers);

    // Pre-fill search if -ss was provided
    if let Some(ref search) = cli.search_string {
        ui.query = search.clone();
    }

    ui.filter(); // Initial filter to show all items (or filtered by search_string)

    // Wrap failed_rowids for thread-safe background loading
    let failed_rowids = Arc::new(Mutex::new(HashSet::<String>::new()));
    let (image_redraw_tx, mut image_redraw_rx) = mpsc::unbounded_channel::<()>();

    // Ensure we have a valid selection if there are items
    if !ui.shown.is_empty() && ui.selected.is_none() {
        ui.selected = Some(0);
    }

    // Determine image preview enablement and cache capabilities
    let mut image_preview_enabled = false;
    let mut cached_is_sixel = false;
    if let Some(ref manager) = image_manager {
        let manager_lock = manager.lock().await;
        image_preview_enabled = options.image_preview_enabled(manager_lock.supports_graphics());
        cached_is_sixel = manager_lock.is_sixel();
    }

    // Show initialization warnings/errors only if image preview is intended
    if picker.is_none() && image_preview_enabled {
        ui.set_temp_message(
            "image_preview enabled but terminal graphics detection failed (using half-block fallback)".to_string(),
        );
    }

    let hide_image_message = options.hide_image_message;
    let show_line_numbers = options.show_line_numbers;
    let show_tag_color_names = options.show_tag_color_names;
    let disable_mouse = options.disable_mouse;

    // List state for ratatui
    let mut list_state = ListState::default();

    // Track previous image state for conditional clearing
    let mut previous_was_image = false;
    // Flag to force Ratatui buffer sync after clearing in tag mode
    let mut force_sixel_sync = false;

    let term_is_foot = options.term_is_foot;
    let cursor = &options.cursor;
    let graphics_adapter = options.graphics_adapter;

    // Track visible height for scroll management
    let mut max_visible = 0;
    let mut needs_redraw = true;
    let mut current_is_image = false;
    let mut current_rowid_opt = None;

    // Main TUI loop
    loop {
        if needs_redraw {
            // Clear expired temporary messages before drawing.
            ui.clear_expired_message();

            // Note: Layout and UI content calculation moved INSIDE the draw loop
            // This ensures wrapping calculations use the SAME dimensions as rendering

            // Check if current item is an image (only when not in tag mode)
            current_is_image = false;
            current_rowid_opt = None;
            if image_preview_enabled
                && matches!(ui.tag_mode, TagMode::Normal)
                && let Some(selected) = ui.selected
                && selected < ui.shown.len()
            {
                let item = &ui.shown[selected];
                if ui.is_cclip_image_item(item) {
                    current_is_image = true;
                    current_rowid_opt = ui.get_cclip_rowid(item);
                }
            }

            // Handle image loading if it changed
            if image_preview_enabled {
                if current_is_image {
                    if let (Some(rowid), Some(manager)) = (&current_rowid_opt, &mut image_manager) {
                        let mut already_loaded = false;
                        let mut is_loading = false;

                        let manager_try_lock = manager.try_lock();
                        if let Ok(mut manager_lock) = manager_try_lock
                            && manager_lock.is_cached(rowid)
                        {
                            manager_lock.set_image(rowid);
                            already_loaded = true;
                        }

                        if !already_loaded {
                            let state = crate::ui::DISPLAY_STATE
                                .lock()
                                .unwrap_or_else(|e| e.into_inner());
                            match &*state {
                                crate::ui::DisplayState::Image(id) if id == rowid => {
                                    already_loaded = true
                                }
                                crate::ui::DisplayState::Loading(id) if id == rowid => {
                                    is_loading = true
                                }
                                _ => {}
                            }
                        }

                        if !already_loaded && !is_loading {
                            let failed_lock = failed_rowids.clone();
                            let failed_guard = failed_lock.lock().await;
                            let is_failed = failed_guard.contains(rowid);
                            drop(failed_guard);

                            if !is_failed {
                                // Set state to loading
                                {
                                    let mut state = crate::ui::DISPLAY_STATE
                                        .lock()
                                        .unwrap_or_else(|e| e.into_inner());
                                    *state = crate::ui::DisplayState::Loading(rowid.clone());
                                }

                                // Spawn background loading task
                                let manager_clone = manager.clone();
                                let rowid_clone = rowid.clone();
                                let redraw_tx = image_redraw_tx.clone();
                                tokio::spawn(async move {
                                    let result = AssertUnwindSafe(async {
                                        let mut manager_lock = manager_clone.lock().await;
                                        let load_result =
                                            manager_lock.load_cclip_image(&rowid_clone).await;
                                        drop(manager_lock);
                                        load_result
                                    })
                                    .catch_unwind()
                                    .await;

                                    match result {
                                        Ok(Ok(_)) => {
                                            failed_lock.lock().await.remove(&rowid_clone);
                                            let mut state = crate::ui::DISPLAY_STATE
                                                .lock()
                                                .unwrap_or_else(|e| e.into_inner());
                                            *state =
                                                crate::ui::DisplayState::Image(rowid_clone.clone());
                                        }
                                        Ok(Err(e)) => {
                                            failed_lock.lock().await.insert(rowid_clone.clone());
                                            let manager_try_lock = manager_clone.try_lock();
                                            if let Ok(mut manager_lock) = manager_try_lock {
                                                manager_lock.clear();
                                            }
                                            let mut state = crate::ui::DISPLAY_STATE
                                                .lock()
                                                .unwrap_or_else(|e| e.into_inner());
                                            *state = crate::ui::DisplayState::Failed(e.to_string());
                                        }
                                        Err(_) => {
                                            failed_lock.lock().await.insert(rowid_clone.clone());
                                            let manager_try_lock = manager_clone.try_lock();
                                            if let Ok(mut manager_lock) = manager_try_lock {
                                                manager_lock.clear();
                                            }
                                            let mut state = crate::ui::DISPLAY_STATE
                                                .lock()
                                                .unwrap_or_else(|e| e.into_inner());
                                            *state = crate::ui::DisplayState::Failed(
                                                "Task panicked during image load".to_string(),
                                            );
                                        }
                                    }
                                    let _ = redraw_tx.send(());
                                });
                            }
                        }
                    }
                } else if previous_was_image {
                    // Clear the image manager if we transitioned away from an image
                    if let Some(manager) = &mut image_manager {
                        let manager_try_lock = manager.try_lock();
                        if let Ok(mut manager_lock) = manager_try_lock {
                            manager_lock.clear();
                        }
                    }
                    let failed_try_lock = failed_rowids.try_lock();
                    if let Ok(mut failed) = failed_try_lock {
                        failed.clear();
                    }
                }
            }

            // For Sixel/Foot: Clear when state changes (only if still using legacy clearing for some reason)
            let mut needs_sixel_clear = false;
            if image_preview_enabled {
                let is_sixel = cached_is_sixel;
                if is_sixel && (previous_was_image != current_is_image) {
                    let _ = terminal.clear();
                    needs_sixel_clear = true;
                }
            }
            if term_is_foot {
                let mut stderr = std::io::stderr();
                let _ = std::io::Write::write_all(&mut stderr, b"\x1b[?2026h");
                let _ = std::io::Write::flush(&mut stderr);
            }

            let mut render_error = Ok(());
            terminal.draw(|f| {
                let highlight_color = options.highlight_color;
                let main_border_color = options.main_border_color;
                let items_border_color = options.items_border_color;
                let input_border_color = options.input_border_color;
                let main_text_color = options.main_text_color;
                let items_text_color = options.items_text_color;
                let input_text_color = options.input_text_color;
                let header_title_color = options.header_title_color;
                let rounded_borders = options.rounded_borders;
                let graphics = graphics_adapter;

                let total_height = f.area().height;
                let content_height = options.content_height(total_height);
                let show_content_panel = content_height > 0;
                let layout = options.split_layout(f.area());
                let chunks = layout.chunks;
                let content_panel_index = layout.content_panel_index;
                let items_panel_index = layout.items_panel_index;
                let input_panel_index = layout.input_panel_index;

                // NOW calculate UI content using the ACTUAL chunks that will be used for rendering
                // This ensures wrapping calculations match the actual render area
                let content_panel_width = chunks[content_panel_index].width;
                let content_panel_height = chunks[content_panel_index].height.saturating_sub(2);

                match &ui.tag_mode {
                    TagMode::Normal => {
                        ui.info_with_image_support(
                            highlight_color,
                            image_preview_enabled,
                            hide_image_message,
                            content_panel_width,
                            content_panel_height,
                        );
                    }
                    _ => {
                        // While in tag modes, suspend inline image preview completely
                        ui.info_with_image_support(
                            highlight_color,
                            false,
                            hide_image_message,
                            content_panel_width,
                            content_panel_height,
                        );
                    }
                }

                // Border type
                let border_type = if rounded_borders {
                    BorderType::Rounded
                } else {
                    BorderType::Plain
                };

                // Content panel (shows selected item's content with potential image preview)
                let content_block = Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        " Clipboard Preview ",
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(header_title_color),
                    ))
                    .border_type(border_type)
                    .border_style(Style::default().fg(main_border_color));

                // Create paragraph WITH wrapping as safety net
                // Text is pre-wrapped manually, but Paragraph wrap prevents ANY overflow
                let content_paragraph = Paragraph::new(ui.text.clone())
                    .block(content_block)
                    .style(Style::default().fg(main_text_color))
                    .alignment(Alignment::Left)
                    .wrap(Wrap { trim: false })
                    .scroll((0, 0));

                // Items panel
                let items_panel_height = chunks[items_panel_index].height;
                max_visible = items_panel_height.saturating_sub(2) as usize;

                let visible_items = ui
                    .shown
                    .iter()
                    .skip(ui.scroll_offset)
                    .take(max_visible)
                    .map(|item| item.to_list_item(Some(&tag_metadata_formatter)))
                    .collect::<Vec<ListItem>>();

                let items_list = List::new(visible_items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(Span::styled(
                                " Clipboard History ",
                                Style::default()
                                    .add_modifier(Modifier::BOLD)
                                    .fg(header_title_color),
                            ))
                            .border_type(border_type)
                            .border_style(Style::default().fg(items_border_color)),
                    )
                    .style(Style::default().fg(items_text_color))
                    .highlight_style({
                        // Use first tag's color for highlight if available
                        let tag_color = if let Some(selected) = ui.selected {
                            if selected < ui.shown.len() {
                                ui.shown[selected]
                                    .tags
                                    .as_ref()
                                    .and_then(|tags| tags.first())
                                    .and_then(|tag| tag_metadata_formatter.get_color(tag))
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        Style::default()
                            .fg(tag_color.unwrap_or(highlight_color))
                            .add_modifier(Modifier::BOLD)
                    })
                    .highlight_symbol("> ");

                // Update list state selection
                let visible_selection = ui.selected.and_then(|sel| {
                    if sel >= ui.scroll_offset && sel < ui.scroll_offset + max_visible {
                        Some(sel - ui.scroll_offset)
                    } else {
                        None
                    }
                });
                list_state.select(visible_selection);

                // Input panel - changes based on tag mode
                let (input_line, input_title) = match &ui.tag_mode {
                    TagMode::PromptingTagName { input, .. } => (
                        Line::from(vec![
                            Span::styled("Tag: ", Style::default().fg(highlight_color)),
                            Span::styled(input, Style::default().fg(input_text_color)),
                            Span::styled(cursor, Style::default().fg(highlight_color)),
                        ]),
                        " Tag Name ",
                    ),
                    TagMode::PromptingTagEmoji { input, .. } => (
                        Line::from(vec![
                            Span::styled("Emoji: ", Style::default().fg(highlight_color)),
                            Span::styled(input, Style::default().fg(input_text_color)),
                            Span::styled(cursor, Style::default().fg(highlight_color)),
                            Span::styled(
                                " (or blank)",
                                Style::default()
                                    .fg(input_text_color)
                                    .add_modifier(Modifier::DIM),
                            ),
                        ]),
                        " Tag Emoji ",
                    ),
                    TagMode::PromptingTagColor { input, .. } => (
                        Line::from(vec![
                            Span::styled("Color: ", Style::default().fg(highlight_color)),
                            Span::styled(input, Style::default().fg(input_text_color)),
                            Span::styled(cursor, Style::default().fg(highlight_color)),
                            Span::styled(
                                " (hex/name or blank)",
                                Style::default()
                                    .fg(input_text_color)
                                    .add_modifier(Modifier::DIM),
                            ),
                        ]),
                        " Tag Color ",
                    ),
                    TagMode::RemovingTag { input, .. } => (
                        Line::from(vec![
                            Span::styled("Remove: ", Style::default().fg(highlight_color)),
                            Span::styled(input, Style::default().fg(input_text_color)),
                            Span::styled(cursor, Style::default().fg(highlight_color)),
                            Span::styled(
                                " (blank = all)",
                                Style::default()
                                    .fg(input_text_color)
                                    .add_modifier(Modifier::DIM),
                            ),
                        ]),
                        " Remove Tag ",
                    ),
                    TagMode::Normal => (
                        Line::from(vec![
                            Span::styled("(", Style::default().fg(input_text_color)),
                            Span::styled(
                                (ui.selected.map_or(0, |v| v + 1)).to_string(),
                                Style::default().fg(highlight_color),
                            ),
                            Span::styled("/", Style::default().fg(input_text_color)),
                            Span::styled(
                                ui.shown.len().to_string(),
                                Style::default().fg(input_text_color),
                            ),
                            Span::styled(") ", Style::default().fg(input_text_color)),
                            Span::styled(">", Style::default().fg(highlight_color)),
                            Span::styled("> ", Style::default().fg(input_text_color)),
                            Span::styled(&ui.query, Style::default().fg(input_text_color)),
                            Span::styled(cursor, Style::default().fg(highlight_color)),
                        ]),
                        " Filter ",
                    ),
                };

                let input_paragraph = Paragraph::new(input_line)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(Span::styled(
                                input_title,
                                Style::default()
                                    .add_modifier(Modifier::BOLD)
                                    .fg(header_title_color),
                            ))
                            .border_type(border_type)
                            .border_style(Style::default().fg(input_border_color)),
                    )
                    .style(Style::default().fg(input_text_color))
                    .alignment(Alignment::Left)
                    .wrap(Wrap { trim: false });

                // Clear widget areas
                // - Kitty: always clear all panels (Kitty needs explicit clearing)
                // - Sixel: clear ALL panels when we did terminal.clear() to sync Ratatui's buffer
                //   This prevents disappearing text because Ratatui knows all panels were cleared
                use ratatui::widgets::Clear;
                let is_kitty = matches!(graphics, crate::ui::GraphicsAdapter::Kitty);

                if is_kitty {
                    // Kitty: Use Clear widget for all panels (Kitty requires explicit clearing)
                    f.render_widget(Clear, chunks[content_panel_index]);
                    f.render_widget(Clear, chunks[items_panel_index]);
                    f.render_widget(Clear, chunks[input_panel_index]);
                } else if needs_sixel_clear || force_sixel_sync {
                    // Sixel: Clear ALL panels to sync Ratatui's buffer with terminal state
                    f.render_widget(Clear, chunks[content_panel_index]);
                    f.render_widget(Clear, chunks[items_panel_index]);
                    f.render_widget(Clear, chunks[input_panel_index]);
                    // Reset flag after using it
                    force_sixel_sync = false;
                }

                let mut image_rendered = false;
                if show_content_panel
                    && image_preview_enabled
                    && current_is_image
                    && let Some(manager) = &mut image_manager
                {
                    let manager_try_lock = manager.try_lock();
                    if let Ok(mut manager_lock) = manager_try_lock {
                        // Calculate image area INSIDE the content panel borders
                        let content_chunk = chunks[content_panel_index];
                        let image_area = ratatui::layout::Rect {
                            x: content_chunk.x + 1,
                            y: content_chunk.y + 1,
                            width: content_chunk.width.saturating_sub(2),
                            height: content_chunk.height.saturating_sub(2),
                        };
                        if let Err(e) = manager_lock.render(f, image_area) {
                            render_error = Err(e);
                        }
                        image_rendered = true;
                    }
                }

                // Render all components in their dynamic positions
                // If image was rendered, we use a simpler block for the content panel to avoid drawing text over/under image
                if show_content_panel {
                    if image_rendered {
                        let content_block = Block::default()
                            .borders(Borders::ALL)
                            .title(Span::styled(
                                " Clipboard Preview ",
                                Style::default()
                                    .add_modifier(Modifier::BOLD)
                                    .fg(header_title_color),
                            ))
                            .border_type(border_type)
                            .border_style(Style::default().fg(main_border_color));
                        f.render_widget(content_block, chunks[content_panel_index]);
                    } else {
                        f.render_widget(content_paragraph, chunks[content_panel_index]);
                    }
                }

                f.render_stateful_widget(items_list, chunks[items_panel_index], &mut list_state);
                f.render_widget(input_paragraph, chunks[input_panel_index]);
            })?;
            render_error?;

            if term_is_foot {
                let mut stderr = std::io::stderr();
                let _ = std::io::Write::write_all(&mut stderr, b"\x1b[?2026l");
                let _ = std::io::Write::flush(&mut stderr);
            }

            // Update state for next iteration only after a draw completed.
            previous_was_image = current_is_image;
        }

        // Note: Post-draw clearing removed - using Clear widget inside draw loop instead
        // Clear widget ensures all widget areas are cleaned before rendering new content

        // Handle input events with full navigation and clipboard copying
        tokio::select! {
            Some(_) = image_redraw_rx.recv() => {
                needs_redraw = true;
            }
            Some(event) = input.next() => match event {
                Event::Input(key) => {
                    needs_redraw = true;
                    match (key.code, key.modifiers) {
                        // Fullscreen image preview keybind
                        (code, mods) if cli.keybinds.matches_image_preview(code, mods) => {
                            if current_is_image
                                && let (Some(_rowid), Some(manager)) =
                                    (&current_rowid_opt, &mut image_manager)
                                {
                                    // Fullscreen modal loop with bounded error tolerance
                                    let mut consecutive_errors: u8 = 0;
                                    loop {
                                        let mut render_err = Ok(());
                                        terminal.draw(|f| {
                                            let manager_try_lock = manager.try_lock();
                                            if let Ok(mut manager_lock) = manager_try_lock
                                                && let Err(e) = manager_lock.render(f, f.area()) {
                                                    render_err = Err(e);
                                                }
                                        })?;
                                        render_err?;

                                        match input.next().await {
                                            Some(Event::Input(key_event)) => {
                                                consecutive_errors = 0;
                                                match (key_event.code, key_event.modifiers) {
                                                    (KeyCode::Esc, _)
                                                    | (KeyCode::Char('q'), _)
                                                    | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                                                        break;
                                                    }
                                                    _ => {} // Ignore other keys
                                                }
                                            }
                                            Some(_) => {
                                                consecutive_errors = 0;
                                            }
                                            None => {
                                                consecutive_errors += 1;
                                                if consecutive_errors >= 3 {
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                    terminal.clear().wrap_err("Failed to clear terminal")?;
                                    // Restore display state instead of purging to force reload
                                    if let Some(rowid) = &current_rowid_opt {
                                        let mut state = crate::ui::DISPLAY_STATE.lock().unwrap_or_else(|e| e.into_inner());
                                        *state = crate::ui::DisplayState::Image(rowid.clone());
                                    }
                                }
                        }
                        // Tag keybind (Ctrl+T)
                        (code, mods) if cli.keybinds.matches_tag(code, mods) => {
                            // Clear any displayed image when entering tag mode
                            if let Some(manager) = &mut image_manager {
                                let manager_try_lock = manager.try_lock();
                                if let Ok(mut manager_lock) = manager_try_lock {
                                    manager_lock.clear();
                                }
                            }
                            let _ = terminal.clear();
                            force_sixel_sync = true;

                            if let Some(selected_idx) = ui.selected
                                && !ui.shown.is_empty() && selected_idx < ui.shown.len() {
                                    let selected_item = ui.shown[selected_idx].original_line.clone();
                                    // Get available tags with just names (no formatting)
                                    let available_tags =
                                        super::scan::get_all_tags().unwrap_or_default();
                                    ui.tag_mode = TagMode::PromptingTagName {
                                        input: String::new(),
                                        selected_item: Some(selected_item),
                                        available_tags,
                                        selected_tag: None,
                                    };
                                }
                        }
                        // Untag keybind (Alt+T)
                        (KeyCode::Char('t'), KeyModifiers::ALT) => {
                            if let Some(selected_idx) = ui.selected
                                && selected_idx < ui.shown.len() {
                                    let item = &ui.shown[selected_idx];
                                    let selected_item = Some(item.original_line.clone());
                                    let parsed_item =
                                        super::CclipItem::from_line(item.original_line.clone());
                                    match parsed_item {
                                        Ok(cclip_item) => {
                                            if !cclip_item.tags.is_empty() {
                                                let first_tag = cclip_item.tags[0].clone();
                                                ui.tag_mode = TagMode::RemovingTag {
                                                    input: first_tag,
                                                    tags: cclip_item.tags.clone(),
                                                    selected: Some(0),
                                                    selected_item,
                                                };
                                            } else {
                                                ui.tag_mode = TagMode::RemovingTag {
                                                    input: String::new(),
                                                    tags: Vec::new(),
                                                    selected: None,
                                                    selected_item,
                                                };
                                            }
                                        }
                                        Err(e) => {
                                            ui.set_temp_message(format!("Failed to parse item: {}", e));
                                            ui.tag_mode = TagMode::RemovingTag {
                                                input: String::new(),
                                                tags: Vec::new(),
                                                selected: None,
                                                selected_item,
                                            };
                                        }
                                    }
                                }
                        }
                        // Delete entry (Alt+Delete)
                        (code, mods) if cli.keybinds.matches_cclip_delete(code, mods) => {
                            if ui.tag_mode == TagMode::Normal {
                                if let Some(selected) = ui.selected
                                    && selected < ui.shown.len() {
                                        let item = &ui.shown[selected];
                                        if let Some(rowid) = ui.get_cclip_rowid(item) {
                                            let delete_result = super::select::delete_item(&rowid);
                                            match delete_result {
                                                Ok(()) => {
                                                    ui.set_temp_message(format!(
                                                        "Deleted entry {}",
                                                        rowid
                                                    ));
                                                    reload_visible_history(
                                                        &mut ui,
                                                        cli,
                                                        &tag_metadata_formatter,
                                                        show_line_numbers,
                                                        show_tag_color_names,
                                                        max_visible,
                                                    );
                                                }
                                                Err(e) => {
                                                    ui.set_temp_message(format!(
                                                        "Failed to delete entry {}: {}",
                                                        rowid, e
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                continue;
                            }
                        }
                        // Exit on escape or Ctrl+C/Q
                        (KeyCode::Esc, _)
                        | (KeyCode::Char('q'), KeyModifiers::CONTROL)
                        | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            // Tag mode cancellation
                            if ui.tag_mode != TagMode::Normal {
                                ui.tag_mode = TagMode::Normal;
                                continue;
                            } else {
                                return Ok(()); // Exit without copying
                            }
                        }
                        // Handle Enter key (clipboard copy or tag mode progression)
                        (KeyCode::Enter, _) | (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                            match ui.tag_mode.clone() {
                                TagMode::PromptingTagName {
                                    input,
                                    selected_item,
                                    selected_tag: _,
                                    available_tags: _,
                                } => {
                                    let tag_name = input.trim().to_string();

                                    // If no name, proceed to emoji (tag can be emoji-only)
                                    if tag_name.is_empty() {
                                        ui.tag_mode = TagMode::PromptingTagEmoji {
                                            tag_name: String::new(),
                                            input: String::new(),
                                            selected_item,
                                        };
                                        continue;
                                    }

                                    // Check if tag already exists on this item - if so, enter editing mode
                                    let is_editing = if let Some(ref item_line) = selected_item {
                                        let parsed_item =
                                            super::CclipItem::from_line(item_line.clone());
                                        match parsed_item {
                                        Ok(cclip_item) => {
                                            cclip_item.tags.contains(&tag_name)
                                        } _ => {
                                            false
                                        }}
                                    } else {
                                        false
                                    };

                                    // Load metadata if tag exists (either editing or selecting existing)
                                    if tag_metadata_map.contains_key(&tag_name) {
                                        let metadata = &tag_metadata_map[&tag_name];

                                        if is_editing {
                                            // Tag already on item - enter editing mode with message
                                            ui.set_temp_message(format!(
                                                "Tag '{}' already applied (editing)",
                                                tag_name
                                            ));
                                        }

                                        ui.tag_mode = TagMode::PromptingTagEmoji {
                                            tag_name: tag_name.clone(),
                                            input: metadata.emoji.clone().unwrap_or_default(),
                                            selected_item,
                                        };
                                    } else {
                                        // New tag - start with empty emoji
                                        ui.tag_mode = TagMode::PromptingTagEmoji {
                                            tag_name,
                                            input: String::new(),
                                            selected_item,
                                        };
                                    }
                                    continue;
                                }
                                TagMode::PromptingTagEmoji {
                                    tag_name,
                                    input,
                                    selected_item,
                                } => {
                                    let emoji = if input.trim().is_empty() {
                                        None
                                    } else {
                                        Some(input.trim().to_string())
                                    };

                                    // If no name and no emoji, cancel
                                    if tag_name.is_empty() && emoji.is_none() {
                                        ui.set_temp_message(
                                            "Tag requires either a name or an emoji".to_string(),
                                        );
                                        ui.tag_mode = TagMode::Normal;
                                        continue;
                                    }

                                    // If no name, use emoji as the tag name
                                    let final_tag_name = if tag_name.is_empty() {
                                        emoji.clone().unwrap_or_default()
                                    } else {
                                        tag_name
                                    };

                                    // Load existing color if editing existing tag
                                    let existing_color = tag_metadata_map
                                        .get(&final_tag_name)
                                        .and_then(|m| m.color.clone())
                                        .unwrap_or_default();

                                    ui.tag_mode = TagMode::PromptingTagColor {
                                        tag_name: final_tag_name,
                                        emoji,
                                        input: existing_color,
                                        selected_item,
                                    };
                                    continue;
                                }
                                TagMode::PromptingTagColor {
                                    tag_name,
                                    emoji,
                                    input,
                                    selected_item,
                                } => {
                                    let color = if input.trim().is_empty() {
                                        None
                                    } else {
                                        Some(input.trim().to_string())
                                    };

                                    // Check if this is editing an existing tag (already on item)
                                    let is_editing = if let Some(ref item_line) = selected_item {
                                        let parsed_item =
                                            super::CclipItem::from_line(item_line.clone());
                                        match parsed_item {
                                        Ok(cclip_item) => {
                                            cclip_item.tags.contains(&tag_name)
                                        } _ => {
                                            false
                                        }}
                                    } else {
                                        false
                                    };

                                    // Get rowid from selected_item
                                    if let Some(ref item_line) = selected_item {
                                        let parts: Vec<&str> =
                                            item_line.splitn(4, '\t').collect::<Vec<&str>>();
                                        if !parts.is_empty() {
                                            let rowid = parts[0];

                                            // Only call tag_item if not editing (would fail if tag already exists)
                                            if !is_editing {
                                                let tag_result =
                                                    super::select::tag_item(rowid, &tag_name);
                                                if let Err(e) = tag_result {
                                                    ui.set_temp_message(format!(
                                                        "Failed to tag item: {}",
                                                        e
                                                    ));
                                                    ui.tag_mode = TagMode::Normal;
                                                    continue;
                                                }
                                            }

                                            // Save tag metadata (always update metadata even when editing)
                                            tag_metadata_map.insert(
                                                tag_name.clone(),
                                                super::TagMetadata {
                                                    name: tag_name.clone(),
                                                    color,
                                                    emoji,
                                                },
                                            );
                                            let save_tag_metadata_result =
                                                super::save_tag_metadata(&db, &tag_metadata_map);
                                            let _ = save_tag_metadata_result;
                                            tag_metadata_formatter = super::TagMetadataFormatter::new(
                                                tag_metadata_map.clone(),
                                            );
                                            reload_visible_history(
                                                &mut ui,
                                                cli,
                                                &tag_metadata_formatter,
                                                show_line_numbers,
                                                show_tag_color_names,
                                                max_visible,
                                            );
                                        }
                                    }

                                    ui.tag_mode = TagMode::Normal;
                                    continue;
                                }
                                TagMode::RemovingTag {
                                    input,
                                    selected_item,
                                    ..
                                } => {
                                    // Get rowid from selected_item
                                    if let Some(ref item_line) = selected_item {
                                        let parts: Vec<&str> =
                                            item_line.splitn(4, '\t').collect::<Vec<&str>>();
                                        if !parts.is_empty() {
                                            let rowid = parts[0];
                                            let tag_to_remove = if input.trim().is_empty() {
                                                None
                                            } else {
                                                Some(input.trim())
                                            };

                                            let untag_result =
                                                super::select::untag_item(rowid, tag_to_remove);
                                            match untag_result {
                                            Err(e) => {
                                                ui.set_temp_message(format!(
                                                    "Failed to remove tag: {}",
                                                    e
                                                ));
                                            } _ => {
                                                reload_visible_history(
                                                    &mut ui,
                                                    cli,
                                                    &tag_metadata_formatter,
                                                    show_line_numbers,
                                                    show_tag_color_names,
                                                    max_visible,
                                                );
                                            }}
                                        }
                                    }

                                    ui.tag_mode = TagMode::Normal;
                                    continue;
                                }
                                TagMode::Normal => {
                                    // Normal mode: copy to clipboard
                                    if let Some(selected) = ui.selected
                                        && selected < ui.shown.len() {
                                            if copy_selected_and_exit(
                                                &mut ui,
                                                selected,
                                                &mut terminal,
                                                disable_mouse,
                                            )? {
                                                return Ok(());
                                            }
                                            continue;
                                        }
                                }
                            }
                        }

                        // Add character to query or tag input
                        (KeyCode::Char(c), KeyModifiers::NONE)
                        | (KeyCode::Char(c), KeyModifiers::SHIFT) => match &mut ui.tag_mode {
                            TagMode::PromptingTagName { input, .. } => {
                                input.push(c);
                            }
                            TagMode::PromptingTagColor { input, .. } => {
                                input.push(c);
                            }
                            TagMode::PromptingTagEmoji { input, .. } => {
                                input.push(c);
                            }
                            TagMode::RemovingTag { input, .. } => {
                                input.push(c);
                            }
                            TagMode::Normal => {
                                ui.query.push(c);
                                ui.filter();
                            }
                        },
                        // Remove character from query or tag input
                        (KeyCode::Backspace, _) => match &mut ui.tag_mode {
                            TagMode::PromptingTagName { input, .. } => {
                                input.pop();
                            }
                            TagMode::PromptingTagColor { input, .. } => {
                                input.pop();
                            }
                            TagMode::PromptingTagEmoji { input, .. } => {
                                input.pop();
                            }
                            TagMode::RemovingTag { input, .. } => {
                                input.pop();
                            }
                            TagMode::Normal => {
                                ui.query.pop();
                                ui.filter();
                            }
                        },
                        // Navigation - Left: go to first item
                        (KeyCode::Left, _) => {
                            // Disable during tag creation
                            if !matches!(ui.tag_mode, TagMode::Normal) {
                                continue;
                            }
                            if !ui.shown.is_empty() {
                                ui.selected = Some(0);
                                ui.scroll_offset = 0;
                            }
                        }
                        // Navigation - Right: go to last item
                        (KeyCode::Right, _) => {
                            // Disable during tag creation
                            if !matches!(ui.tag_mode, TagMode::Normal) {
                                continue;
                            }
                            if !ui.shown.is_empty() {
                                let last_index = ui.shown.len() - 1;
                                ui.selected = Some(last_index);

                                let max_visible =
                                    options.max_visible_items(terminal.size()?.height);

                                if max_visible > 0 && ui.shown.len() > max_visible {
                                    ui.scroll_offset = ui.shown.len().saturating_sub(max_visible);
                                } else {
                                    ui.scroll_offset = 0;
                                }
                            }
                        }
                        // Navigation - Down: next item with scrolling (or cycle tag selection)
                        (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                            // Handle tag mode navigation
                            match &ui.tag_mode {
                                TagMode::PromptingTagName { .. } => {
                                    ui.cycle_tag_creation_selection(1);
                                    continue;
                                }
                                TagMode::RemovingTag { .. } => {
                                    ui.cycle_removal_selection(1);
                                    continue;
                                }
                                TagMode::PromptingTagEmoji { .. }
                                | TagMode::PromptingTagColor { .. } => {
                                    // Disable navigation during emoji/color input
                                    continue;
                                }
                                _ => {}
                            }

                            if let Some(selected) = ui.selected {
                                let hard_stop = options.hard_stop;
                                ui.selected = if ui.shown.is_empty() {
                                    Some(selected)
                                } else if selected + 1 < ui.shown.len() {
                                    Some(selected + 1)
                                } else if !hard_stop {
                                    Some(0)
                                } else {
                                    Some(selected)
                                };

                                // Auto-scroll to keep selection visible
                                if let Some(new_selected) = ui.selected {
                                    let max_visible =
                                        options.max_visible_items(terminal.size()?.height);

                                    if max_visible == 0 {
                                        ui.scroll_offset = 0;
                                    } else {
                                        // Scroll down if selection is below visible area
                                        if new_selected >= ui.scroll_offset + max_visible {
                                            ui.scroll_offset =
                                                new_selected.saturating_sub(max_visible - 1);
                                        }
                                        // Scroll up if selection is above visible area (happens when wrapping to top)
                                        else if new_selected < ui.scroll_offset {
                                            ui.scroll_offset = new_selected;
                                        }
                                    }
                                }
                            }
                        }
                        // Navigation - Up: previous item with scrolling (or cycle tag selection)
                        (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                            // Handle tag mode navigation
                            match &ui.tag_mode {
                                TagMode::PromptingTagName { .. } => {
                                    ui.cycle_tag_creation_selection(-1);
                                    continue;
                                }
                                TagMode::RemovingTag { .. } => {
                                    ui.cycle_removal_selection(-1);
                                    continue;
                                }
                                TagMode::PromptingTagEmoji { .. }
                                | TagMode::PromptingTagColor { .. } => {
                                    // Disable navigation during emoji/color input
                                    continue;
                                }
                                _ => {}
                            }

                            if let Some(selected) = ui.selected {
                                let hard_stop = options.hard_stop;
                                ui.selected = if selected > 0 {
                                    Some(selected - 1)
                                } else if !hard_stop && !ui.shown.is_empty() {
                                    Some(ui.shown.len() - 1)
                                } else {
                                    Some(selected)
                                };

                                // Auto-scroll to keep selection visible
                                if let Some(new_selected) = ui.selected {
                                    let max_visible =
                                        options.max_visible_items(terminal.size()?.height);

                                    if max_visible == 0 {
                                        ui.scroll_offset = 0;
                                    } else {
                                        // Scroll up if selection is above visible area
                                        if new_selected < ui.scroll_offset {
                                            ui.scroll_offset = new_selected;
                                        }
                                        // Scroll down if selection is below visible area (happens when wrapping to bottom)
                                        else if new_selected >= ui.scroll_offset + max_visible {
                                            ui.scroll_offset =
                                                new_selected.saturating_sub(max_visible - 1);
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Event::Mouse(mouse_event) => {
                    needs_redraw = true;
                    // Mouse handling (similar to dmenu mode)
                    let mouse_row = mouse_event.row;
                    let (items_panel_start, items_panel_height) =
                        options.items_panel_bounds(terminal.size()?.height);

                    let items_content_start = items_panel_start + 1; // +1 for top border
                    let max_visible_rows = items_panel_height.saturating_sub(2); // -2 for borders
                    let items_content_end = items_content_start + max_visible_rows;

                    let update_selection_for_mouse_pos = |ui: &mut DmenuUI, mouse_row: u16| {
                        if !ui.shown.is_empty()
                            && mouse_row >= items_content_start
                            && mouse_row < items_content_end
                        {
                            let row_in_content = mouse_row - items_content_start;
                            let hovered_item_index = ui.scroll_offset + row_in_content as usize;
                            if hovered_item_index < ui.shown.len() {
                                ui.selected = Some(hovered_item_index);
                                // Content update happens at start of loop
                            }
                        }
                    };

                    match mouse_event.kind {
                        MouseEventKind::Moved => {
                            // Disable mouse during tag creation
                            if matches!(ui.tag_mode, TagMode::Normal) {
                                update_selection_for_mouse_pos(&mut ui, mouse_row);
                            }
                        }
                        MouseEventKind::Down(MouseButton::Left) => {
                            // Disable mouse clicks during tag creation
                            if !matches!(ui.tag_mode, TagMode::Normal) {
                                continue;
                            }
                            if mouse_row >= items_content_start
                                && mouse_row < items_content_end
                                && !ui.shown.is_empty()
                            {
                                let row_in_content = mouse_row - items_content_start;
                                let clicked_item_index = ui.scroll_offset + row_in_content as usize;

                                if clicked_item_index < ui.shown.len() {
                                    if copy_selected_and_exit(
                                        &mut ui,
                                        clicked_item_index,
                                        &mut terminal,
                                        disable_mouse,
                                    )? {
                                        return Ok(());
                                    }
                                    continue;
                                }
                            }
                        }
                        MouseEventKind::ScrollUp => {
                            // Disable scroll during tag creation
                            if !matches!(ui.tag_mode, TagMode::Normal) {
                                continue;
                            }
                            if !ui.shown.is_empty() && ui.scroll_offset > 0 {
                                ui.scroll_offset -= 1;
                                update_selection_for_mouse_pos(&mut ui, mouse_row);
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            // Disable scroll during tag creation
                            if !matches!(ui.tag_mode, TagMode::Normal) {
                                continue;
                            }
                            if !ui.shown.is_empty() {
                                let max_visible = max_visible_rows as usize;
                                if ui.scroll_offset + max_visible < ui.shown.len() {
                                    ui.scroll_offset += 1;
                                    update_selection_for_mouse_pos(&mut ui, mouse_row);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Event::Tick => {
                    needs_redraw = ui.temp_message.is_some();
                }
                Event::Render => {
                    needs_redraw = true;
                }
            }
        }
    }
}
