//! Cclip mode - main event loop and TUI
//!
//! This module contains the async run function that implements the clipboard
//! history browser with TUI interface.

use crate::cli::Opts;
use crate::common::Item;
use crate::ui::{DmenuUI, InputConfig, InputEvent as Event, TagMode};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, MouseButton, MouseEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use eyre::{eyre, Result, WrapErr};
use futures::FutureExt;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;
use scopeguard::defer;
use std::collections::HashSet;
use std::io;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Run cclip mode - async TUI event loop for clipboard history
pub async fn run(cli: &Opts) -> Result<()> {
    use crossterm::event::{KeyCode, KeyModifiers};

    // Check if cclip is available
    if !super::scan::check_cclip_available() {
        return Err(eyre!(
            "cclip is not available. Please install cclip and ensure it's in your PATH."
        ));
    }

    // Check if cclip database is accessible
    super::scan::check_cclip_database().wrap_err("cclip database check failed")?;

    // Handle clear tags mode (fsel metadata only)
    if cli.cclip_clear_tags {
        let (db, _) = crate::core::database::open_history_db()?;

        // Clear tag metadata from fsel database
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(super::TAG_METADATA_TABLE)?;
            let _ = table.remove("tag_metadata");
        }
        write_txn.commit()?;

        println!("Cleared all tag metadata from fsel database");
        println!();
        println!("Note: To wipe tags from cclip entries too, use:");
        println!("  fsel --cclip --tag wipe");
        return Ok(());
    }

    // Handle wipe tags mode (cclip + fsel metadata)
    if cli.cclip_wipe_tags {
        // First wipe cclip tags
        super::select::wipe_all_tags().wrap_err("Failed to wipe cclip tags")?;
        println!("Wiped all tags from cclip entries");

        // Also clear fsel metadata
        let (db, _) = crate::core::database::open_history_db()?;
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(super::TAG_METADATA_TABLE)?;
            let _ = table.remove("tag_metadata");
        }
        write_txn.commit()?;
        println!("Cleared all tag metadata from fsel database");

        return Ok(());
    }

    // Handle tag list mode
    if cli.cclip_tag_list {
        let tags = super::scan::get_all_tags().wrap_err("Failed to get tags from cclip")?;

        if tags.is_empty() {
            println!("No tags found");
            return Ok(());
        }

        // If specific tag requested, show items in that tag
        if let Some(ref tag_name) = cli.cclip_tag {
            println!("Items tagged with '{}':", tag_name);
            let items = super::scan::get_clipboard_history_by_tag(tag_name)
                .wrap_err("Failed to get items by tag")?;

            if items.is_empty() {
                println!("  (no items)");
            } else {
                for item in items {
                    if cli.verbose.unwrap_or(0) >= 2 {
                        // Verbose: show full details
                        println!("  [{}] {} - {}", item.rowid, item.mime_type, item.preview);
                    } else {
                        // Normal: just preview
                        println!("  {}", item.preview);
                    }
                }
            }
        } else {
            // Just list tag names
            println!("Available tags:");
            for tag in tags {
                if cli.verbose.unwrap_or(0) >= 2 {
                    // Verbose: show item count
                    let items = super::scan::get_clipboard_history_by_tag(&tag).unwrap_or_default();
                    println!("  {} ({} items)", tag, items.len());
                } else {
                    println!("  {}", tag);
                }
            }
        }
        return Ok(());
    }

    // Get clipboard history from cclip (filtered by tag if specified)
    let cclip_items = if let Some(ref tag_name) = cli.cclip_tag {
        super::scan::get_clipboard_history_by_tag(tag_name).wrap_err(format!(
            "Failed to get clipboard history for tag '{}'",
            tag_name
        ))?
    } else {
        super::scan::get_clipboard_history()
            .wrap_err("Failed to get clipboard history from cclip")?
    };

    if cclip_items.is_empty() {
        if let Some(tag_name) = &cli.cclip_tag {
            println!("No clipboard items with tag '{}'", tag_name);
        } else {
            println!("No clipboard history available");
        }
        return Ok(());
    }

    // Get show_line_numbers setting early for item conversion
    let show_line_numbers = cli
        .cclip_show_line_numbers
        .or(Some(cli.dmenu_show_line_numbers))
        .unwrap_or(false);

    // Load tag metadata for proper tag coloring
    let (db, _) = crate::core::database::open_history_db()?;
    let mut tag_metadata_map = super::load_tag_metadata(&db);
    let mut tag_metadata_formatter = super::TagMetadataFormatter::new(tag_metadata_map.clone());

    // Get show_tag_color_names setting (defaults to false)
    let show_tag_color_names = cli.cclip_show_tag_color_names.unwrap_or(false);

    // Convert to DmenuItems with tag metadata formatting
    let items: Vec<Item> = cclip_items
        .into_iter()
        .enumerate()
        .map(|(idx, cclip_item)| {
            // Use numbered display name if show_line_numbers is enabled
            // Show color names based on CLI/config setting
            let display_name = if show_line_numbers {
                // Use database rowid (shows actual DB ID) with tag formatting
                cclip_item.get_display_name_with_number_formatter_options(
                    Some(&tag_metadata_formatter),
                    show_tag_color_names,
                )
            } else {
                cclip_item.get_display_name_with_formatter_options(
                    Some(&tag_metadata_formatter),
                    show_tag_color_names,
                )
            };

            let mut item =
                Item::new_simple(cclip_item.original_line.clone(), display_name, idx + 1);
            item.tags = Some(cclip_item.tags.clone());
            item
        })
        .collect();

    // Setup terminal
    enable_raw_mode().wrap_err("Failed to enable raw mode")?;
    io::stderr()
        .execute(EnterAlternateScreen)
        .wrap_err("Failed to enter alternate screen")?;

    // Get effective disable_mouse setting with cclip -> dmenu -> regular inheritance
    let disable_mouse = cli
        .cclip_disable_mouse
        .or(cli.dmenu_disable_mouse)
        .unwrap_or(cli.disable_mouse);
    if !disable_mouse {
        io::stderr()
            .execute(EnableMouseCapture)
            .wrap_err("Failed to enable mouse capture")?;
    }

    // Ensure cleanup on exit
    defer! {
        if !disable_mouse {
            let _ = io::stderr().execute(DisableMouseCapture);
        }
        let _ = io::stderr().execute(LeaveAlternateScreen);
        let _ = disable_raw_mode();
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

    // Input handler - use Null key to prevent Escape from killing the input thread
    // (Escape is handled manually in the main loop for tag mode cancellation)
    let input = InputConfig {
        exit_key: KeyCode::Null,
        disable_mouse,
        ..InputConfig::default()
    }
    .init();

    // Create dmenu UI using cclip settings with inheritance
    // Wrapping should be enabled by default for proper text display
    let wrap_long_lines = cli.cclip_wrap_long_lines.unwrap_or(true);
    let mut ui = DmenuUI::new(items, wrap_long_lines, show_line_numbers);

    // Pre-fill search if -ss was provided
    if let Some(ref search) = cli.search_string {
        ui.query = search.clone();
    }

    ui.filter(); // Initial filter to show all items (or filtered by search_string)

    // Wrap failed_rowids for thread-safe background loading
    let failed_rowids = Arc::new(Mutex::new(HashSet::<String>::new()));

    // Ensure we have a valid selection if there are items
    if !ui.shown.is_empty() && ui.selected.is_none() {
        ui.selected = Some(0);
    }

    // Determine image preview enablement and cache capabilities
    let mut image_preview_enabled = cli.cclip_image_preview.unwrap_or(false);
    let mut cached_is_sixel = false;
    if let Some(ref manager) = image_manager {
        let manager_lock = manager.lock().await;
        if cli.cclip_image_preview.is_none() {
            image_preview_enabled = manager_lock.supports_graphics();
        }
        cached_is_sixel = manager_lock.is_sixel();
    }

    // Show initialization warnings/errors only if image preview is intended
    if picker.is_none() && image_preview_enabled {
        ui.set_temp_message(
            "image_preview enabled but terminal graphics detection failed (using half-block fallback)".to_string(),
        );
    }

    // Get effective colors with cclip -> dmenu -> regular inheritance
    let get_cclip_color =
        |cclip_opt: Option<ratatui::style::Color>,
         dmenu_opt: Option<ratatui::style::Color>,
         default: ratatui::style::Color| { cclip_opt.or(dmenu_opt).unwrap_or(default) };
    let get_cclip_bool = |cclip_opt: Option<bool>, dmenu_opt: Option<bool>, default: bool| {
        cclip_opt.or(dmenu_opt).unwrap_or(default)
    };
    let get_cclip_u16 = |cclip_opt: Option<u16>, dmenu_opt: Option<u16>, default: u16| {
        cclip_opt.or(dmenu_opt).unwrap_or(default)
    };
    let get_cclip_panel_position =
        |cclip_opt: Option<crate::cli::PanelPosition>,
         dmenu_opt: Option<crate::cli::PanelPosition>,
         default: crate::cli::PanelPosition| { cclip_opt.or(dmenu_opt).unwrap_or(default) };

    // Update info with image support
    // Calculate layout to get actual chunk width

    // Get hide image message setting
    let hide_image_message = cli.cclip_hide_inline_image_message.unwrap_or(false);

    // List state for ratatui
    let mut list_state = ListState::default();

    // Track previous image state for conditional clearing
    let mut previous_was_image = false;
    // Flag to force Ratatui buffer sync after clearing in tag mode
    let mut force_sixel_sync = false;

    // For Foot: use DEC Private Mode 2026 (synchronized updates) to prevent mid-frame tearing
    let term_is_foot = std::env::var("TERM")
        .unwrap_or_default()
        .starts_with("foot");

    // Get effective cursor string with inheritance
    let cursor = cli
        .cclip_cursor
        .as_ref()
        .or(cli.dmenu_cursor.as_ref())
        .unwrap_or(&cli.cursor);

    // Pre-detect graphics adapter for performance
    let graphics_adapter = crate::ui::GraphicsAdapter::detect(picker.as_ref());

    // Track visible height for scroll management
    let mut max_visible = 0;

    // Main TUI loop
    loop {
        // Clear expired temporary messages
        ui.clear_expired_message();

        // Note: Layout and UI content calculation moved INSIDE the draw loop
        // This ensures wrapping calculations use the SAME dimensions as rendering

        // Check if current item is an image (only when not in tag mode)
        let mut current_is_image = false;
        let mut current_rowid_opt = None;
        if image_preview_enabled && matches!(ui.tag_mode, TagMode::Normal) {
            if let Some(selected) = ui.selected {
                if selected < ui.shown.len() {
                    let item = &ui.shown[selected];
                    if ui.is_cclip_image_item(item) {
                        current_is_image = true;
                        current_rowid_opt = ui.get_cclip_rowid(item);
                    }
                }
            }
        }

        // Handle image loading if it changed
        if image_preview_enabled {
            if current_is_image {
                if let (Some(rowid), Some(manager)) = (&current_rowid_opt, &mut image_manager) {
                    let mut is_loading = false;
                    let mut already_loaded = false;
                    {
                        let state = crate::ui::DISPLAY_STATE.lock().await;
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
                        let is_failed = failed_lock.lock().await.contains(rowid);

                        if !is_failed {
                            // Set state to loading
                            {
                                let mut state = crate::ui::DISPLAY_STATE.lock().await;
                                *state = crate::ui::DisplayState::Loading(rowid.clone());
                            }

                            // Spawn background loading task
                            let manager_clone = manager.clone();
                            let rowid_clone = rowid.clone();
                            tokio::spawn(async move {
                                let result = AssertUnwindSafe(async {
                                    let mut manager_lock = manager_clone.lock().await;
                                    manager_lock.load_cclip_image(&rowid_clone).await
                                })
                                .catch_unwind()
                                .await;

                                match result {
                                    Ok(Ok(_)) => {
                                        failed_lock.lock().await.remove(&rowid_clone);
                                        let mut state = crate::ui::DISPLAY_STATE.lock().await;
                                        *state = crate::ui::DisplayState::Image(rowid_clone);
                                    }
                                    Ok(Err(e)) => {
                                        failed_lock.lock().await.insert(rowid_clone.clone());
                                        if let Ok(mut manager_lock) = manager_clone.try_lock() {
                                            manager_lock.clear();
                                        }
                                        let mut state = crate::ui::DISPLAY_STATE.lock().await;
                                        *state = crate::ui::DisplayState::Failed(e.to_string());
                                    }
                                    Err(_) => {
                                        failed_lock.lock().await.insert(rowid_clone.clone());
                                        if let Ok(mut manager_lock) = manager_clone.try_lock() {
                                            manager_lock.clear();
                                        }
                                        let mut state = crate::ui::DISPLAY_STATE.lock().await;
                                        *state = crate::ui::DisplayState::Failed(
                                            "Task panicked during image load".to_string(),
                                        );
                                    }
                                }
                            });
                        }
                    }
                }
            } else if previous_was_image {
                // Clear the image manager if we transitioned away from an image
                if let Some(manager) = &mut image_manager {
                    if let Ok(mut manager_lock) = manager.try_lock() {
                        manager_lock.clear();
                    }
                }
                if let Ok(mut failed) = failed_rowids.try_lock() {
                    failed.clear();
                }
                {
                    let mut state = crate::ui::DISPLAY_STATE.lock().await;
                    *state = crate::ui::DisplayState::Empty;
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

        terminal.draw(|f| {
            // Get effective colors and settings for cclip mode with inheritance
            let highlight_color = get_cclip_color(
                cli.cclip_highlight_color,
                cli.dmenu_highlight_color,
                cli.highlight_color,
            );
            let main_border_color = get_cclip_color(
                cli.cclip_main_border_color,
                cli.dmenu_main_border_color,
                cli.main_border_color,
            );
            let items_border_color = get_cclip_color(
                cli.cclip_items_border_color,
                cli.dmenu_items_border_color,
                cli.apps_border_color,
            );
            let input_border_color = get_cclip_color(
                cli.cclip_input_border_color,
                cli.dmenu_input_border_color,
                cli.input_border_color,
            );
            let main_text_color = get_cclip_color(
                cli.cclip_main_text_color,
                cli.dmenu_main_text_color,
                cli.main_text_color,
            );
            let items_text_color = get_cclip_color(
                cli.cclip_items_text_color,
                cli.dmenu_items_text_color,
                cli.apps_text_color,
            );
            let input_text_color = get_cclip_color(
                cli.cclip_input_text_color,
                cli.dmenu_input_text_color,
                cli.input_text_color,
            );
            let header_title_color = get_cclip_color(
                cli.cclip_header_title_color,
                cli.dmenu_header_title_color,
                cli.header_title_color,
            );
            let rounded_borders = get_cclip_bool(
                cli.cclip_rounded_borders,
                cli.dmenu_rounded_borders,
                cli.rounded_borders,
            );
            let content_panel_height = get_cclip_u16(
                cli.cclip_title_panel_height_percent,
                cli.dmenu_title_panel_height_percent,
                cli.title_panel_height_percent,
            );
            let input_panel_height = get_cclip_u16(
                cli.cclip_input_panel_height,
                cli.dmenu_input_panel_height,
                cli.input_panel_height,
            );

            // Use pre-detected graphics adapter
            let graphics = graphics_adapter;

            // Layout calculation
            let total_height = f.area().height;
            let content_height =
                (total_height as f32 * content_panel_height as f32 / 100.0).round() as u16;

            // Get content panel position (defaults to Top if not set, with cclip -> dmenu -> regular inheritance)
            let content_panel_position = get_cclip_panel_position(
                cli.cclip_title_panel_position,
                cli.dmenu_title_panel_position,
                cli.title_panel_position
                    .unwrap_or(crate::cli::PanelPosition::Top),
            );

            // Split the window into three parts based on content panel position
            let (chunks, content_panel_index, items_panel_index, input_panel_index) =
                match content_panel_position {
                    crate::cli::PanelPosition::Top => {
                        // Top: content, items, input (original layout)
                        let layout = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Length(content_height.max(3)),
                                Constraint::Min(1),
                                Constraint::Length(input_panel_height),
                            ])
                            .split(f.area());
                        (layout, 0, 1, 2)
                    }
                    crate::cli::PanelPosition::Middle => {
                        // Middle: items, content, input
                        let layout = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Min(1),
                                Constraint::Length(content_height.max(3)),
                                Constraint::Length(input_panel_height),
                            ])
                            .split(f.area());
                        (layout, 1, 0, 2)
                    }
                    crate::cli::PanelPosition::Bottom => {
                        // Bottom: items, input, content
                        let layout = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Min(1),                        // Items panel (remaining space)
                                Constraint::Length(input_panel_height),    // Input panel
                                Constraint::Length(content_height.max(3)), // Content panel at bottom
                            ])
                            .split(f.area());
                        (layout, 2, 0, 1)
                    }
                };

            // NOW calculate UI content using the ACTUAL chunks that will be used for rendering
            // This ensures wrapping calculations match the actual render area
            let content_panel_width = chunks[content_panel_index].width;
            let content_panel_height = chunks[content_panel_index].height.saturating_sub(2);

            match &ui.tag_mode {
                TagMode::Normal => {
                    ui.info_with_image_support(
                        get_cclip_color(
                            cli.cclip_highlight_color,
                            cli.dmenu_highlight_color,
                            cli.highlight_color,
                        ),
                        image_preview_enabled,
                        hide_image_message,
                        content_panel_width,
                        content_panel_height,
                    );
                }
                _ => {
                    // While in tag modes, suspend inline image preview completely
                    ui.info_with_image_support(
                        get_cclip_color(
                            cli.cclip_highlight_color,
                            cli.dmenu_highlight_color,
                            cli.highlight_color,
                        ),
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

            // Render image if enabled and we have a manager
            let mut image_rendered = false;
            if image_preview_enabled && current_is_image {
                if let Some(manager) = &mut image_manager {
                    if let Ok(mut manager_lock) = manager.try_lock() {
                        // Calculate image area INSIDE the content panel borders
                        let content_chunk = chunks[content_panel_index];
                        let image_area = ratatui::layout::Rect {
                            x: content_chunk.x + 1,
                            y: content_chunk.y + 1,
                            width: content_chunk.width.saturating_sub(2),
                            height: content_chunk.height.saturating_sub(2),
                        };
                        manager_lock.render(f, image_area);
                        image_rendered = true;
                    }
                }
            }

            // Render all components in their dynamic positions
            // If image was rendered, we use a simpler block for the content panel to avoid drawing text over/under image
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

            f.render_stateful_widget(items_list, chunks[items_panel_index], &mut list_state);
            f.render_widget(input_paragraph, chunks[input_panel_index]);
        })?;

        if term_is_foot {
            let mut stderr = std::io::stderr();
            let _ = std::io::Write::write_all(&mut stderr, b"\x1b[?2026l");
            let _ = std::io::Write::flush(&mut stderr);
        }

        // Note: Post-draw clearing removed - using Clear widget inside draw loop instead
        // Clear widget ensures all widget areas are cleaned before rendering new content

        // Handle input events with full navigation and clipboard copying
        match input.next()? {
            Event::Input(key) => {
                match (key.code, key.modifiers) {
                    // Fullscreen image preview keybind
                    (code, mods) if cli.keybinds.matches_image_preview(code, mods) => {
                        if current_is_image {
                            if let (Some(_rowid), Some(manager)) =
                                (&current_rowid_opt, &mut image_manager)
                            {
                                // Fullscreen modal loop with bounded error tolerance
                                let mut consecutive_errors: u8 = 0;
                                loop {
                                    terminal.draw(|f| {
                                        if let Ok(mut manager_lock) = manager.try_lock() {
                                            manager_lock.render(f, f.area());
                                        }
                                    })?;

                                    match input.next() {
                                        Ok(Event::Input(key_event)) => {
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
                                        Ok(_) => {
                                            consecutive_errors = 0;
                                        }
                                        Err(_) => {
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
                                    let mut state = crate::ui::DISPLAY_STATE.lock().await;
                                    *state = crate::ui::DisplayState::Image(rowid.clone());
                                }
                            }
                        }
                    }
                    // Tag keybind (Ctrl+T)
                    (code, mods) if cli.keybinds.matches_tag(code, mods) => {
                        // Clear any displayed image when entering tag mode
                        if let Some(manager) = &mut image_manager {
                            if let Ok(mut manager_lock) = manager.try_lock() {
                                manager_lock.clear();
                            }
                        }
                        let _ = terminal.clear();
                        force_sixel_sync = true;
                        // Reset display state
                        {
                            let mut state = crate::ui::DISPLAY_STATE.lock().await;
                            *state = crate::ui::DisplayState::Empty;
                        }

                        if let Some(selected_idx) = ui.selected {
                            if !ui.shown.is_empty() && selected_idx < ui.shown.len() {
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
                    }
                    // Untag keybind (Alt+T)
                    (KeyCode::Char('t'), KeyModifiers::ALT) => {
                        if let Some(selected_idx) = ui.selected {
                            if selected_idx < ui.shown.len() {
                                let item = &ui.shown[selected_idx];
                                let selected_item = Some(item.original_line.clone());
                                match super::CclipItem::from_line(item.original_line.clone()) {
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
                    }
                    // Delete entry (Alt+Delete)
                    (code, mods) if cli.keybinds.matches_cclip_delete(code, mods) => {
                        if ui.tag_mode == TagMode::Normal {
                            if let Some(selected) = ui.selected {
                                if selected < ui.shown.len() {
                                    let item = &ui.shown[selected];
                                    if let Some(rowid) = ui.get_cclip_rowid(item) {
                                        match super::select::delete_item(&rowid) {
                                            Ok(()) => {
                                                ui.set_temp_message(format!(
                                                    "Deleted entry {}",
                                                    rowid
                                                ));

                                                // Reload clipboard history (respecting tag filter if active)
                                                let updated_items_res =
                                                    if let Some(ref tag_name) = cli.cclip_tag {
                                                        super::scan::get_clipboard_history_by_tag(
                                                            tag_name,
                                                        )
                                                    } else {
                                                        super::scan::get_clipboard_history()
                                                    };

                                                if let Ok(updated_items) = updated_items_res {
                                                    reload_and_restore(
                                                        &mut ui,
                                                        updated_items,
                                                        &tag_metadata_formatter,
                                                        show_line_numbers,
                                                        show_tag_color_names,
                                                        max_visible,
                                                    );
                                                }
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
                                    if let Ok(cclip_item) =
                                        super::CclipItem::from_line(item_line.clone())
                                    {
                                        cclip_item.tags.contains(&tag_name)
                                    } else {
                                        false
                                    }
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
                                    if let Ok(cclip_item) =
                                        super::CclipItem::from_line(item_line.clone())
                                    {
                                        cclip_item.tags.contains(&tag_name)
                                    } else {
                                        false
                                    }
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
                                            if let Err(e) =
                                                super::select::tag_item(rowid, &tag_name)
                                            {
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
                                        let _ = super::save_tag_metadata(&db, &tag_metadata_map);
                                        tag_metadata_formatter = super::TagMetadataFormatter::new(
                                            tag_metadata_map.clone(),
                                        );

                                        // Reload clipboard history to get updated tags
                                        let updated_items_res =
                                            if let Some(ref tag_name) = cli.cclip_tag {
                                                super::scan::get_clipboard_history_by_tag(tag_name)
                                            } else {
                                                super::scan::get_clipboard_history()
                                            };

                                        if let Ok(updated_items) = updated_items_res {
                                            reload_and_restore(
                                                &mut ui,
                                                updated_items,
                                                &tag_metadata_formatter,
                                                show_line_numbers,
                                                show_tag_color_names,
                                                max_visible,
                                            );
                                        }
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

                                        if let Err(e) =
                                            super::select::untag_item(rowid, tag_to_remove)
                                        {
                                            ui.set_temp_message(format!(
                                                "Failed to remove tag: {}",
                                                e
                                            ));
                                        } else {
                                            // Reload clipboard history to get updated tags
                                            let updated_items_res = if let Some(ref tag_name) =
                                                cli.cclip_tag
                                            {
                                                super::scan::get_clipboard_history_by_tag(tag_name)
                                            } else {
                                                super::scan::get_clipboard_history()
                                            };

                                            if let Ok(updated_items) = updated_items_res {
                                                reload_and_restore(
                                                    &mut ui,
                                                    updated_items,
                                                    &tag_metadata_formatter,
                                                    show_line_numbers,
                                                    show_tag_color_names,
                                                    max_visible,
                                                );
                                            }
                                        }
                                    }
                                }

                                ui.tag_mode = TagMode::Normal;
                                continue;
                            }
                            TagMode::Normal => {
                                // Normal mode: copy to clipboard
                                if let Some(selected) = ui.selected {
                                    if selected < ui.shown.len() {
                                        let original_line = &ui.shown[selected].original_line;
                                        match super::CclipItem::from_line(original_line.clone()) {
                                            Ok(cclip_item) => {
                                                if let Err(e) = cclip_item.copy_to_clipboard() {
                                                    ui.set_temp_message(format!(
                                                        "Copy failed: {}",
                                                        e
                                                    ));
                                                    continue;
                                                }

                                                // clean up terminal completely and exit
                                                terminal
                                                    .show_cursor()
                                                    .wrap_err("Failed to show cursor")?;
                                                drop(terminal);
                                                if !disable_mouse {
                                                    let _ =
                                                        io::stderr().execute(DisableMouseCapture);
                                                }
                                                let _ = io::stderr().execute(LeaveAlternateScreen);
                                                let _ = disable_raw_mode();
                                                return Ok(());
                                            }
                                            Err(e) => {
                                                ui.set_temp_message(format!("Parse failed: {}", e));
                                                continue;
                                            }
                                        }
                                    }
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

                            // Scroll to show last item
                            let total_height = terminal.size()?.height;
                            let content_panel_height = get_cclip_u16(
                                cli.cclip_title_panel_height_percent,
                                cli.dmenu_title_panel_height_percent,
                                cli.title_panel_height_percent,
                            );
                            let input_panel_height = get_cclip_u16(
                                cli.cclip_input_panel_height,
                                cli.dmenu_input_panel_height,
                                cli.input_panel_height,
                            );

                            // Use same calculation as rendering code
                            let content_height = (total_height as f32 * content_panel_height as f32
                                / 100.0)
                                .round() as u16;
                            let content_height = content_height.max(3);
                            let items_panel_height =
                                total_height - content_height - input_panel_height;
                            let max_visible = items_panel_height.saturating_sub(2) as usize;

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
                            let hard_stop = get_cclip_bool(
                                cli.cclip_hard_stop,
                                cli.dmenu_hard_stop,
                                cli.hard_stop,
                            );
                            ui.selected = if selected < ui.shown.len() - 1 {
                                Some(selected + 1)
                            } else if !hard_stop {
                                Some(0)
                            } else {
                                Some(selected)
                            };

                            // Auto-scroll to keep selection visible
                            if let Some(new_selected) = ui.selected {
                                let total_height = terminal.size()?.height;
                                let content_panel_height = get_cclip_u16(
                                    cli.cclip_title_panel_height_percent,
                                    cli.dmenu_title_panel_height_percent,
                                    cli.title_panel_height_percent,
                                );
                                let input_panel_height = get_cclip_u16(
                                    cli.cclip_input_panel_height,
                                    cli.dmenu_input_panel_height,
                                    cli.input_panel_height,
                                );

                                // Use same calculation as rendering code
                                let content_height =
                                    (total_height as f32 * content_panel_height as f32 / 100.0)
                                        .round() as u16;
                                let content_height = content_height.max(3);
                                let items_panel_height =
                                    total_height - content_height - input_panel_height;
                                let max_visible = items_panel_height.saturating_sub(2) as usize; // -2 for borders

                                // Scroll down if selection is below visible area
                                if new_selected >= ui.scroll_offset + max_visible {
                                    ui.scroll_offset = new_selected.saturating_sub(max_visible - 1);
                                }
                                // Scroll up if selection is above visible area (happens when wrapping to top)
                                else if new_selected < ui.scroll_offset {
                                    ui.scroll_offset = new_selected;
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
                            let hard_stop = get_cclip_bool(
                                cli.cclip_hard_stop,
                                cli.dmenu_hard_stop,
                                cli.hard_stop,
                            );
                            ui.selected = if selected > 0 {
                                Some(selected - 1)
                            } else if !hard_stop {
                                Some(ui.shown.len() - 1)
                            } else {
                                Some(selected)
                            };

                            // Auto-scroll to keep selection visible
                            if let Some(new_selected) = ui.selected {
                                let total_height = terminal.size()?.height;
                                let content_panel_height = get_cclip_u16(
                                    cli.cclip_title_panel_height_percent,
                                    cli.dmenu_title_panel_height_percent,
                                    cli.title_panel_height_percent,
                                );
                                let input_panel_height = get_cclip_u16(
                                    cli.cclip_input_panel_height,
                                    cli.dmenu_input_panel_height,
                                    cli.input_panel_height,
                                );

                                // Use same calculation as rendering code
                                let content_height =
                                    (total_height as f32 * content_panel_height as f32 / 100.0)
                                        .round() as u16;
                                let content_height = content_height.max(3);
                                let items_panel_height =
                                    total_height - content_height - input_panel_height;
                                let max_visible = items_panel_height.saturating_sub(2) as usize; // -2 for borders

                                // Scroll up if selection is above visible area
                                if new_selected < ui.scroll_offset {
                                    ui.scroll_offset = new_selected;
                                }
                                // Scroll down if selection is below visible area (happens when wrapping to bottom)
                                else if new_selected >= ui.scroll_offset + max_visible {
                                    ui.scroll_offset = new_selected.saturating_sub(max_visible - 1);
                                }
                            }
                        }
                    }
                    _ => {}
                }
                // Content update now happens at the start of the loop before drawing
            }
            Event::Mouse(mouse_event) => {
                // Mouse handling (similar to dmenu mode)
                let mouse_row = mouse_event.row;
                let total_height = terminal.size()?.height;
                let content_panel_height = get_cclip_u16(
                    cli.cclip_title_panel_height_percent,
                    cli.dmenu_title_panel_height_percent,
                    cli.title_panel_height_percent,
                );
                let input_panel_height = get_cclip_u16(
                    cli.cclip_input_panel_height,
                    cli.dmenu_input_panel_height,
                    cli.input_panel_height,
                );

                // Use same calculation as rendering code
                let content_height =
                    (total_height as f32 * content_panel_height as f32 / 100.0).round() as u16;
                let content_height = content_height.max(3);
                let items_panel_height = total_height - content_height - input_panel_height;

                // Get content panel position to calculate items panel position
                let content_panel_position = get_cclip_panel_position(
                    cli.cclip_title_panel_position,
                    cli.dmenu_title_panel_position,
                    cli.title_panel_position
                        .unwrap_or(crate::cli::PanelPosition::Top),
                );

                // Calculate items panel coordinates based on layout
                let (items_panel_start, items_panel_height) = match content_panel_position {
                    crate::cli::PanelPosition::Top => {
                        // Top: content, items, input - items start after content
                        (content_height, items_panel_height)
                    }
                    crate::cli::PanelPosition::Middle => {
                        // Middle: items, content, input - items start at top
                        (0, items_panel_height)
                    }
                    crate::cli::PanelPosition::Bottom => {
                        // Bottom: items, input, content - items start at top
                        (0, items_panel_height)
                    }
                };

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
                                let original_line = &ui.shown[clicked_item_index].original_line;
                                match super::CclipItem::from_line(original_line.clone()) {
                                    Ok(cclip_item) => {
                                        if let Err(e) = cclip_item.copy_to_clipboard() {
                                            ui.set_temp_message(format!("Copy failed: {}", e));
                                            continue;
                                        }

                                        // clean up terminal completely and exit
                                        terminal.show_cursor().wrap_err("Failed to show cursor")?;
                                        drop(terminal);
                                        if !disable_mouse {
                                            let _ = io::stderr().execute(DisableMouseCapture);
                                        }
                                        let _ = io::stderr().execute(LeaveAlternateScreen);
                                        let _ = disable_raw_mode();
                                        return Ok(());
                                    }
                                    Err(e) => {
                                        ui.set_temp_message(format!("Parse failed: {}", e));
                                        continue;
                                    }
                                }
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
            Event::Tick => {}
            Event::Render => {} // Handled by draw loop
        }

        // Update state for next iteration
        previous_was_image = current_is_image;
    }
}
/// Helper to reload clipboard items and restore selection/scroll
fn reload_and_restore(
    ui: &mut DmenuUI,
    updated_items: Vec<super::CclipItem>,
    tag_metadata_formatter: &super::TagMetadataFormatter,
    show_line_numbers: bool,
    show_tag_color_names: bool,
    visible_height: usize,
) {
    // Recreate items
    let new_items: Vec<Item> = updated_items
        .into_iter()
        .enumerate()
        .map(|(idx, cclip_item)| {
            let display_name = if show_line_numbers {
                cclip_item.get_display_name_with_number_formatter_options(
                    Some(tag_metadata_formatter),
                    show_tag_color_names,
                )
            } else {
                cclip_item.get_display_name_with_formatter_options(
                    Some(tag_metadata_formatter),
                    show_tag_color_names,
                )
            };

            let mut item =
                Item::new_simple(cclip_item.original_line.clone(), display_name, idx + 1);
            item.tags = Some(cclip_item.tags.clone());
            item
        })
        .collect();

    // Preserve current selection by rowid (first field in original_line)
    let selected_rowid = ui
        .selected
        .and_then(|idx| ui.shown.get(idx))
        .and_then(|item| item.original_line.split('\t').next().map(|s| s.to_string()));

    // Update UI with new items
    ui.hidden = new_items;
    ui.shown.clear();
    ui.filter();

    // Restore selection by finding the same rowid
    if let Some(ref rowid) = selected_rowid {
        if let Some(pos) = ui
            .shown
            .iter()
            .position(|item| item.original_line.split('\t').next() == Some(rowid.as_str()))
        {
            ui.selected = Some(pos);
            // Adjust scroll to keep selection visible
            if pos < ui.scroll_offset {
                ui.scroll_offset = pos;
            } else if pos >= ui.scroll_offset + visible_height {
                ui.scroll_offset = pos + 1 - visible_height;
            }
        } else if !ui.shown.is_empty() {
            ui.selected = Some(0);
            ui.scroll_offset = 0;
        }
    } else if !ui.shown.is_empty() && ui.selected.is_none() {
        // If nothing was selected before but we have items now, select first
        ui.selected = Some(0);
        ui.scroll_offset = 0;
    }

    // Final boundary check for scroll offset
    let max_scroll = ui.shown.len().saturating_sub(visible_height);
    if ui.scroll_offset > max_scroll {
        ui.scroll_offset = max_scroll;
    }
}
