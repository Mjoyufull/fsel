//! Cclip mode - main event loop and TUI
//! 
//! This module contains the async run function that implements the clipboard
//! history browser with TUI interface.

use eyre::{Result, WrapErr, eyre};
use crate::cli::Opts;
use crate::ui::{DmenuUI, TagMode, InputConfig, InputEvent as Event};
use crate::common::Item;
use std::io;
use scopeguard::defer;
use crossterm::{
    event::{MouseButton, MouseEventKind, EnableMouseCapture, DisableMouseCapture},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;

/// Run cclip mode - async TUI event loop for clipboard history
pub async fn run(cli: &Opts) -> Result<()> {
    use crossterm::event::{KeyCode, KeyModifiers};

    // Check if cclip is available
    if !super::scan::check_cclip_available() {
        return Err(eyre!("cclip is not available. Please install cclip and ensure it's in your PATH."));
    }
    
    // Check if cclip database is accessible
    super::scan::check_cclip_database()
        .wrap_err("cclip database check failed")?;

    // Handle clear tags mode
    if cli.cclip_clear_tags {
        let (db, _) = crate::core::database::open_history_db()?;
        
        // Clear all tags from cclip items
        let result = std::process::Command::new("cclip")
            .args(&["clear-tags"])
            .output();
        
        match result {
            Ok(output) if output.status.success() => {
                println!("Cleared all tags from cclip items");
            }
            Ok(output) => {
                eprintln!("Warning: cclip clear-tags failed: {}", 
                    String::from_utf8_lossy(&output.stderr));
            }
            Err(e) => {
                eprintln!("Warning: Failed to run cclip clear-tags: {}", e);
            }
        }
        
        // Clear tag metadata from fsel database
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(super::TAG_METADATA_TABLE)?;
            let _ = table.remove("tag_metadata");
        }
        write_txn.commit()?;
        
        println!("Cleared all tag metadata");
        return Ok(());
    }
    
    // Handle tag list mode
    if cli.cclip_tag_list {
        let tags = super::scan::get_all_tags()
            .wrap_err("Failed to get tags from cclip")?;
        
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
                    let items = super::scan::get_clipboard_history_by_tag(&tag)
                        .unwrap_or_default();
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
        super::scan::get_clipboard_history_by_tag(tag_name)
            .wrap_err(format!("Failed to get clipboard history for tag '{}'", tag_name))?
    } else {
        super::scan::get_clipboard_history()
            .wrap_err("Failed to get clipboard history from cclip")?
    };
    
    if cclip_items.is_empty() {
        if cli.cclip_tag.is_some() {
            println!("No clipboard items with tag '{}'", cli.cclip_tag.as_ref().unwrap());
        } else {
            println!("No clipboard history available");
        }
        return Ok(());
    }

    // Get show_line_numbers setting early for item conversion
    let show_line_numbers = cli.cclip_show_line_numbers.or(Some(cli.dmenu_show_line_numbers)).unwrap_or(false);

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
                cclip_item.get_display_name_with_number_formatter_options(Some(&tag_metadata_formatter), show_tag_color_names)
            } else {
                cclip_item.get_display_name_with_formatter_options(Some(&tag_metadata_formatter), show_tag_color_names)
            };
            
            let mut item = Item::new_simple(
                cclip_item.original_line.clone(),
                display_name,
                idx + 1
            );
            item.tags = Some(cclip_item.tags.clone());
            item
        })
        .collect();

    // Setup terminal
    enable_raw_mode().wrap_err("Failed to enable raw mode")?;
    io::stderr().execute(EnterAlternateScreen).wrap_err("Failed to enter alternate screen")?;
    
    // Get effective disable_mouse setting with cclip -> dmenu -> regular inheritance
    let disable_mouse = cli.cclip_disable_mouse.or(cli.dmenu_disable_mouse).unwrap_or(cli.disable_mouse);
    if !disable_mouse {
        io::stderr().execute(EnableMouseCapture).wrap_err("Failed to enable mouse capture")?;
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

    // Input handler - use Null key to prevent Escape from killing the input thread
    // (Escape is handled manually in the main loop for tag mode cancellation)
    let input = InputConfig {
        exit_key: KeyCode::Null,
        disable_mouse: disable_mouse,
        ..InputConfig::default()
    }.init();

    // Create dmenu UI using cclip settings with inheritance
    // Default to false for cclip to avoid Foot terminal rendering issues with sixel images
    let wrap_long_lines = cli.cclip_wrap_long_lines.unwrap_or(false);
    let mut ui = DmenuUI::new(items, wrap_long_lines, show_line_numbers);
    
    // Pre-fill search if -ss was provided
    if let Some(ref search) = cli.search_string {
        ui.query = search.clone();
    }
    
    ui.filter(); // Initial filter to show all items (or filtered by search_string)
    
    // Ensure we have a valid selection if there are items
    if !ui.shown.is_empty() && ui.selected.is_none() {
        ui.selected = Some(0);
    }
    
    // Check if terminal supports graphics and chafa is available
    let chafa_available = super::preview::check_chafa_available();
    
    // enable inline preview for both Kitty and Sixel protocols
    let graphics_adapter = crate::ui::GraphicsAdapter::detect();
    let supports_graphics = !matches!(graphics_adapter, crate::ui::GraphicsAdapter::None);
    
    let image_preview_enabled = cli.cclip_image_preview.unwrap_or(chafa_available && supports_graphics);
    
    // warn if image preview is enabled but requirements aren't met
    if image_preview_enabled && !chafa_available {
        eprintln!("warning: image_preview is enabled but chafa is not installed");
        eprintln!("install chafa for image previews: https://github.com/hpjansson/chafa");
    }
    if image_preview_enabled && !supports_graphics {
        eprintln!("warning: image_preview is enabled but your terminal doesn't support graphics");
        eprintln!("supported terminals: Kitty, Foot, WezTerm, xterm (with sixel support)");
    }
    
    // Get effective colors with cclip -> dmenu -> regular inheritance
    let get_cclip_color = |cclip_opt: Option<ratatui::style::Color>, dmenu_opt: Option<ratatui::style::Color>, default: ratatui::style::Color| {
        cclip_opt.or(dmenu_opt).unwrap_or(default)
    };
    let get_cclip_bool = |cclip_opt: Option<bool>, dmenu_opt: Option<bool>, default: bool| {
        cclip_opt.or(dmenu_opt).unwrap_or(default)
    };
    let get_cclip_u16 = |cclip_opt: Option<u16>, dmenu_opt: Option<u16>, default: u16| {
        cclip_opt.or(dmenu_opt).unwrap_or(default)
    };
    let get_cclip_panel_position = |cclip_opt: Option<crate::cli::PanelPosition>, dmenu_opt: Option<crate::cli::PanelPosition>, default: crate::cli::PanelPosition| {
        cclip_opt.or(dmenu_opt).unwrap_or(default)
    };
    
    // Update info with image support
    let content_panel_width = terminal.size()?.width.saturating_sub(2); // Account for borders
    let content_panel_height = (terminal.size()?.height as f32 * get_cclip_u16(cli.cclip_title_panel_height_percent, cli.dmenu_title_panel_height_percent, cli.title_panel_height_percent) as f32 / 100.0).round() as u16;
    let content_panel_height = content_panel_height.max(3).saturating_sub(2); // Account for borders
    
    // Get hide image message setting
    let hide_image_message = cli.cclip_hide_inline_image_message.unwrap_or(false);
    
    ui.info_with_image_support(
        get_cclip_color(cli.cclip_highlight_color, cli.dmenu_highlight_color, cli.highlight_color),
        image_preview_enabled,
        hide_image_message,
        content_panel_width,
        content_panel_height
    );
    
    // List state for ratatui
    let mut list_state = ListState::default();
    
    // Track previous selection state to avoid unnecessary clearing
    let mut previous_was_image = false;
    
    // Get effective cursor string with inheritance
    let cursor = cli.cclip_cursor.as_ref()
        .or(cli.dmenu_cursor.as_ref())
        .unwrap_or(&cli.cursor);

    // Main TUI loop
    loop {
        // Clear expired temporary messages
        ui.clear_expired_message();
        
        // Update UI content BEFORE drawing to avoid race conditions with graphics clearing
        let content_panel_width = terminal.size()?.width.saturating_sub(2);
        let content_panel_height = (terminal.size()?.height as f32 * get_cclip_u16(cli.cclip_title_panel_height_percent, cli.dmenu_title_panel_height_percent, cli.title_panel_height_percent) as f32 / 100.0).round() as u16;
        let content_panel_height = content_panel_height.max(3).saturating_sub(2);
        
        // Update UI based on tag mode
        match &ui.tag_mode {
            TagMode::Normal => {
                ui.info_with_image_support(
                    get_cclip_color(cli.cclip_highlight_color, cli.dmenu_highlight_color, cli.highlight_color),
                    image_preview_enabled,
                    hide_image_message,
                    content_panel_width,
                    content_panel_height
                );
            }
            _ => {
                // Tag mode UI is handled in dmenu_ui.rs info_with_image_support
                ui.info_with_image_support(
                    get_cclip_color(cli.cclip_highlight_color, cli.dmenu_highlight_color, cli.highlight_color),
                    image_preview_enabled,
                    hide_image_message,
                    content_panel_width,
                    content_panel_height
                );
            }
        }
        
        // Check if current item is an image
        let mut current_is_image = false;
        if image_preview_enabled {
            if let Some(selected) = ui.selected {
                if selected < ui.shown.len() {
                    let item = &ui.shown[selected];
                    if ui.get_cclip_rowid(item).is_some() {
                        current_is_image = true;
                    }
                }
            }
        }
        
        // For Sixel/Foot: Clear when state changes OR when showing different image
        let needs_clear = if image_preview_enabled {
            if previous_was_image != current_is_image {
                // State changed (text↔image)
                true
            } else if current_is_image {
                // Both are images - check if it's a different image
                if let Some(selected) = ui.selected {
                    if selected < ui.shown.len() {
                        if let Some(current_rowid) = ui.get_cclip_rowid(&ui.shown[selected]) {
                            // Check if this is a different image than what's displayed
                            let is_different = if let Ok(state) = crate::ui::DISPLAY_STATE.lock() {
                                match &*state {
                                    crate::ui::DisplayState::Image(_, displayed_rowid) => {
                                        displayed_rowid != &current_rowid
                                    }
                                    _ => true
                                }
                            } else {
                                true
                            };
                            is_different
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };
        
        if needs_clear {
            let graphics = crate::ui::GraphicsAdapter::detect();
            if matches!(graphics, crate::ui::GraphicsAdapter::Sixel) {
                let _ = terminal.clear();
            }
        }
        
        terminal.draw(|f| {
            // Get effective colors and settings for cclip mode with inheritance
            let highlight_color = get_cclip_color(cli.cclip_highlight_color, cli.dmenu_highlight_color, cli.highlight_color);
            let main_border_color = get_cclip_color(cli.cclip_main_border_color, cli.dmenu_main_border_color, cli.main_border_color);
            let items_border_color = get_cclip_color(cli.cclip_items_border_color, cli.dmenu_items_border_color, cli.apps_border_color);
            let input_border_color = get_cclip_color(cli.cclip_input_border_color, cli.dmenu_input_border_color, cli.input_border_color);
            let main_text_color = get_cclip_color(cli.cclip_main_text_color, cli.dmenu_main_text_color, cli.main_text_color);
            let items_text_color = get_cclip_color(cli.cclip_items_text_color, cli.dmenu_items_text_color, cli.apps_text_color);
            let input_text_color = get_cclip_color(cli.cclip_input_text_color, cli.dmenu_input_text_color, cli.input_text_color);
            let header_title_color = get_cclip_color(cli.cclip_header_title_color, cli.dmenu_header_title_color, cli.header_title_color);
            let rounded_borders = get_cclip_bool(cli.cclip_rounded_borders, cli.dmenu_rounded_borders, cli.rounded_borders);
            let content_panel_height = get_cclip_u16(cli.cclip_title_panel_height_percent, cli.dmenu_title_panel_height_percent, cli.title_panel_height_percent);
            let input_panel_height = get_cclip_u16(cli.cclip_input_panel_height, cli.dmenu_input_panel_height, cli.input_panel_height);
            
            // Layout calculation
            let total_height = f.area().height;
            let content_height = (total_height as f32 * content_panel_height as f32 / 100.0).round() as u16;
            
            // Get content panel position (defaults to Top if not set, with cclip -> dmenu -> regular inheritance)
            let content_panel_position = get_cclip_panel_position(cli.cclip_title_panel_position, cli.dmenu_title_panel_position, cli.title_panel_position.unwrap_or(crate::cli::PanelPosition::Top));
            
            // Split the window into three parts based on content panel position
            let (chunks, content_panel_index, items_panel_index, input_panel_index) = match content_panel_position {
                crate::cli::PanelPosition::Top => {
                    // Top: content, items, input (original layout)
                    let layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(content_height.max(3)),
                            Constraint::Min(1),
                            Constraint::Length(input_panel_height),
                        ].as_ref())
                        .split(f.area());
                    (layout, 0, 1, 2)
                },
                crate::cli::PanelPosition::Middle => {
                    // Middle: items, content, input
                    let layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Min(1),
                            Constraint::Length(content_height.max(3)),
                            Constraint::Length(input_panel_height),
                        ].as_ref())
                        .split(f.area());
                    (layout, 1, 0, 2)
                },
                crate::cli::PanelPosition::Bottom => {
                    // Bottom: items, input, content
                    let layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Min(1),                         // Items panel (remaining space)
                            Constraint::Length(input_panel_height),     // Input panel
                            Constraint::Length(content_height.max(3)),  // Content panel at bottom
                        ].as_ref())
                        .split(f.area());
                    (layout, 2, 0, 1)
                }
            };
            
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
                    Style::default().add_modifier(Modifier::BOLD).fg(header_title_color),
                ))
                .border_type(border_type)
                .border_style(Style::default().fg(main_border_color));
            
            // Create paragraph - NEVER use .wrap() to prevent Foot terminal rendering issues
            // Text wrapping is handled manually in dmenu_ui.rs by splitting into multiple Lines
            let content_paragraph = Paragraph::new(ui.text.clone())
                .block(content_block)
                .style(Style::default().fg(main_text_color))
                .alignment(Alignment::Left)
                .scroll((0, 0)); // Explicitly set scroll to prevent any automatic scrolling
            
            // Items panel
            let items_panel_height = chunks[items_panel_index].height;
            let max_visible = items_panel_height.saturating_sub(2) as usize;
            
            let visible_items = ui.shown
                .iter()
                .skip(ui.scroll_offset)
                .take(max_visible)
                .map(|item| item.to_list_item(Some(&tag_metadata_formatter)))
                .collect::<Vec<ListItem>>();
            
            let items_list = List::new(visible_items)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        " Clipboard History ",
                        Style::default().add_modifier(Modifier::BOLD).fg(header_title_color),
                    ))
                    .border_type(border_type)
                    .border_style(Style::default().fg(items_border_color))
                )
                .style(Style::default().fg(items_text_color))
                .highlight_style({
                    // Use first tag's color for highlight if available
                    let tag_color = if let Some(selected) = ui.selected {
                        if selected < ui.shown.len() {
                            ui.shown[selected].tags.as_ref()
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
                TagMode::PromptingTagName { input, .. } => {
                    (Line::from(vec![
                        Span::styled("Tag: ", Style::default().fg(highlight_color)),
                        Span::styled(input, Style::default().fg(input_text_color)),
                        Span::styled(cursor, Style::default().fg(highlight_color)),
                    ]), " Tag Name ")
                }
                TagMode::PromptingTagEmoji { input, .. } => {
                    (Line::from(vec![
                        Span::styled("Emoji: ", Style::default().fg(highlight_color)),
                        Span::styled(input, Style::default().fg(input_text_color)),
                        Span::styled(cursor, Style::default().fg(highlight_color)),
                        Span::styled(" (or blank)", Style::default().fg(input_text_color).add_modifier(Modifier::DIM)),
                    ]), " Tag Emoji ")
                }
                TagMode::PromptingTagColor { input, .. } => {
                    (Line::from(vec![
                        Span::styled("Color: ", Style::default().fg(highlight_color)),
                        Span::styled(input, Style::default().fg(input_text_color)),
                        Span::styled(cursor, Style::default().fg(highlight_color)),
                        Span::styled(" (hex/name or blank)", Style::default().fg(input_text_color).add_modifier(Modifier::DIM)),
                    ]), " Tag Color ")
                }
                TagMode::RemovingTag { input, .. } => {
                    (Line::from(vec![
                        Span::styled("Remove: ", Style::default().fg(highlight_color)),
                        Span::styled(input, Style::default().fg(input_text_color)),
                        Span::styled(cursor, Style::default().fg(highlight_color)),
                        Span::styled(" (blank = all)", Style::default().fg(input_text_color).add_modifier(Modifier::DIM)),
                    ]), " Remove Tag ")
                }
                TagMode::Normal => {
                    (Line::from(vec![
                        Span::styled("(", Style::default().fg(input_text_color)),
                        Span::styled(
                            (ui.selected.map_or(0, |v| v + 1)).to_string(),
                            Style::default().fg(highlight_color),
                        ),
                        Span::styled("/", Style::default().fg(input_text_color)),
                        Span::styled(ui.shown.len().to_string(), Style::default().fg(input_text_color)),
                        Span::styled(") ", Style::default().fg(input_text_color)),
                        Span::styled(">", Style::default().fg(highlight_color)),
                        Span::styled("> ", Style::default().fg(input_text_color)),
                        Span::styled(&ui.query, Style::default().fg(input_text_color)),
                        Span::styled(cursor, Style::default().fg(highlight_color)),
                    ]), " Filter ")
                }
            };
            
            let input_paragraph = Paragraph::new(input_line)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        input_title,
                        Style::default().add_modifier(Modifier::BOLD).fg(header_title_color),
                    ))
                    .border_type(border_type)
                    .border_style(Style::default().fg(input_border_color))
                )
                .style(Style::default().fg(input_text_color))
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: false });
            
            // Render all components in their dynamic positions
            f.render_widget(content_paragraph, chunks[content_panel_index]);
            f.render_stateful_widget(items_list, chunks[items_panel_index], &mut list_state);
            f.render_widget(input_paragraph, chunks[input_panel_index]);
        })?;
        
        // After ratatui draws, handle image display
        if image_preview_enabled && current_is_image {
            let graphics = crate::ui::GraphicsAdapter::detect();
            
            // Show new image if current item is an image  
            {
                if let Some(selected) = ui.selected {
                    if selected < ui.shown.len() {
                        let item = &ui.shown[selected];
                        if let Some(rowid) = ui.get_cclip_rowid(item) {
                            // Get the content panel chunk position from the last draw
                            // We need to recalculate the layout to get the correct chunk positions
                            let term_size = terminal.get_frame().area();
                            let total_height = term_size.height;
                            let content_panel_height_percent = get_cclip_u16(cli.cclip_title_panel_height_percent, cli.dmenu_title_panel_height_percent, cli.title_panel_height_percent);
                            let content_height = (total_height as f32 * content_panel_height_percent as f32 / 100.0).round() as u16;
                            let content_height = content_height.max(3);
                            let input_panel_height = get_cclip_u16(cli.cclip_input_panel_height, cli.dmenu_input_panel_height, cli.input_panel_height);
                            let content_panel_position = get_cclip_panel_position(cli.cclip_title_panel_position, cli.dmenu_title_panel_position, cli.title_panel_position.unwrap_or(crate::cli::PanelPosition::Top));
                            
                            // Recalculate layout to get chunk positions (same as in draw)
                            let (chunks, content_panel_index, _, _) = match content_panel_position {
                                crate::cli::PanelPosition::Top => {
                                    let layout = Layout::default()
                                        .direction(Direction::Vertical)
                                        .constraints([
                                            Constraint::Length(content_height),
                                            Constraint::Min(1),
                                            Constraint::Length(input_panel_height),
                                        ].as_ref())
                                        .split(term_size);
                                    (layout, 0, 1, 2)
                                },
                                crate::cli::PanelPosition::Middle => {
                                    let layout = Layout::default()
                                        .direction(Direction::Vertical)
                                        .constraints([
                                            Constraint::Min(1),
                                            Constraint::Length(content_height),
                                            Constraint::Length(input_panel_height),
                                        ].as_ref())
                                        .split(term_size);
                                    (layout, 1, 0, 2)
                                },
                                crate::cli::PanelPosition::Bottom => {
                                    let layout = Layout::default()
                                        .direction(Direction::Vertical)
                                        .constraints([
                                            Constraint::Min(1),
                                            Constraint::Length(input_panel_height),
                                            Constraint::Length(content_height),
                                        ].as_ref())
                                        .split(term_size);
                                    (layout, 2, 0, 1)
                                }
                            };
                            
                            // Get the content panel chunk
                            let content_chunk = chunks[content_panel_index];
                            
                            // Calculate image area INSIDE the content panel borders
                            let image_area = ratatui::layout::Rect {
                                x: content_chunk.x + 1,  // Inside left border
                                y: content_chunk.y + 1,  // Inside top border
                                width: content_chunk.width.saturating_sub(2),  // Account for left+right borders
                                height: content_chunk.height.saturating_sub(2),  // Account for top+bottom borders
                            };
                            
                            // Show image inside the content panel
                            let _ = graphics.show_cclip_image_if_different(&rowid, image_area).await;
                        }
                    }
                }
            }
        }
        
        // Update state for next iteration
        previous_was_image = current_is_image;
        
        // Handle input events with full navigation and clipboard copying
        match input.next()? {
            Event::Input(key) => {
                match (key.code, key.modifiers) {
                    // Check for image preview keybind FIRST (before other Ctrl+key checks)
                    (code, mods) if cli.keybinds.matches_image_preview(code, mods) => {
                        if let Some(selected) = ui.selected {
                            if selected < ui.shown.len() {
                                let item = &ui.shown[selected];
                                if ui.display_image_to_terminal(item) {
                                    use crossterm::event::read;
                                    let _ = read();
                                    terminal.clear().wrap_err("Failed to clear terminal")?;
                                }
                            }
                        }
                    }
                    // Tag keybind (Ctrl+T)
                    (code, mods) if cli.keybinds.matches_tag(code, mods) => {
                        if ui.selected.is_some() && !ui.shown.is_empty() {
                            let selected_idx = ui.selected.unwrap();
                            if selected_idx < ui.shown.len() {
                                let selected_item = ui.shown[selected_idx].original_line.clone();
                                // Get available tags with just names (no formatting)
                                let available_tags = super::scan::get_all_tags().unwrap_or_default();
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
                        if ui.selected.is_some() && !ui.shown.is_empty() {
                            let selected_idx = ui.selected.unwrap();
                            let item = &ui.shown[selected_idx];
                            let selected_item = Some(item.original_line.clone());
                            if let Ok(cclip_item) = super::CclipItem::from_line(item.original_line.clone()) {
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
                        }
                    }
                    // Exit on escape or Ctrl+C/Q
                    (KeyCode::Esc, _) | (KeyCode::Char('q'), KeyModifiers::CONTROL) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
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
                            TagMode::PromptingTagName { input, selected_item, selected_tag: _, available_tags: _ } => {
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
                                    if let Ok(cclip_item) = super::CclipItem::from_line(item_line.clone()) {
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
                                        ui.set_temp_message(format!("Tag '{}' already applied (editing)", tag_name));
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
                            TagMode::PromptingTagEmoji { tag_name, input, selected_item } => {
                                let emoji = if input.trim().is_empty() {
                                    None
                                } else {
                                    Some(input.trim().to_string())
                                };
                                
                                // If no name and no emoji, cancel
                                if tag_name.is_empty() && emoji.is_none() {
                                    ui.set_temp_message("Tag requires either a name or an emoji".to_string());
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
                                let existing_color = tag_metadata_map.get(&final_tag_name)
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
                            TagMode::PromptingTagColor { tag_name, emoji, input, selected_item } => {
                                let color = if input.trim().is_empty() {
                                    None
                                } else {
                                    Some(input.trim().to_string())
                                };
                                
                                // Check if this is editing an existing tag (already on item)
                                let is_editing = if let Some(ref item_line) = selected_item {
                                    if let Ok(cclip_item) = super::CclipItem::from_line(item_line.clone()) {
                                        cclip_item.tags.contains(&tag_name)
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                };
                                
                                // Get rowid from selected_item
                                if let Some(ref item_line) = selected_item {
                                    let parts: Vec<&str> = item_line.splitn(4, '\t').collect::<Vec<&str>>();
                                    if !parts.is_empty() {
                                        let rowid = parts[0];
                                        
                                        // Only call tag_item if not editing (would fail if tag already exists)
                                        if !is_editing {
                                            if let Err(e) = super::select::tag_item(rowid, &tag_name) {
                                                eprintln!("Failed to tag item: {}", e);
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
                                            }
                                        );
                                        let _ = super::save_tag_metadata(&db, &tag_metadata_map);
                                        tag_metadata_formatter = super::TagMetadataFormatter::new(tag_metadata_map.clone());
                                        
                                        // Reload clipboard history to get updated tags
                                        if let Ok(updated_items) = super::scan::get_clipboard_history() {
                                            // Recreate items with updated formatter
                                            let new_items: Vec<Item> = updated_items
                                                .into_iter()
                                                .enumerate()
                                                .map(|(idx, cclip_item)| {
                                                    let display_name = if show_line_numbers {
                                                        cclip_item.get_display_name_with_number_formatter_options(Some(&tag_metadata_formatter), show_tag_color_names)
                                                    } else {
                                                        cclip_item.get_display_name_with_formatter_options(Some(&tag_metadata_formatter), show_tag_color_names)
                                                    };
                                                    
                                                    let mut item = Item::new_simple(
                                                        cclip_item.original_line.clone(),
                                                        display_name,
                                                        idx + 1
                                                    );
                                                    item.tags = Some(cclip_item.tags.clone());
                                                    item
                                                })
                                                .collect();
                                            
                                            // Preserve current selection by rowid (first field in original_line)
                                            let selected_rowid = ui.selected
                                                .and_then(|idx| ui.shown.get(idx))
                                                .and_then(|item| {
                                                    item.original_line.split('\t').next().map(|s| s.to_string())
                                                });
                                            
                                            // Update UI with new items
                                            ui.hidden = new_items;
                                            ui.shown.clear();
                                            ui.filter();
                                            
                                            // Restore selection by finding the same rowid
                                            if let Some(ref rowid) = selected_rowid {
                                                if let Some(pos) = ui.shown.iter().position(|item| {
                                                    item.original_line.split('\t').next() == Some(rowid.as_str())
                                                }) {
                                                    ui.selected = Some(pos);
                                                } else if !ui.shown.is_empty() {
                                                    ui.selected = Some(0);
                                                }
                                            }
                                        }
                                    }
                                }
                                
                                ui.tag_mode = TagMode::Normal;
                                continue;
                            }
                            TagMode::RemovingTag { input, selected_item, .. } => {
                                // Get rowid from selected_item
                                if let Some(ref item_line) = selected_item {
                                    let parts: Vec<&str> = item_line.splitn(4, '\t').collect::<Vec<&str>>();
                                    if !parts.is_empty() {
                                        let rowid = parts[0];
                                        let tag_to_remove = if input.trim().is_empty() {
                                            None
                                        } else {
                                            Some(input.trim())
                                        };
                                        
                                        if let Err(e) = super::select::untag_item(rowid, tag_to_remove) {
                                            eprintln!("Failed to remove tag: {}", e);
                                        } else {
                                            // Reload clipboard history to get updated tags
                                            if let Ok(updated_items) = super::scan::get_clipboard_history() {
                                                // Recreate items with current formatter
                                                let new_items: Vec<Item> = updated_items
                                                    .into_iter()
                                                    .enumerate()
                                                    .map(|(idx, cclip_item)| {
                                                        let display_name = if show_line_numbers {
                                                            cclip_item.get_display_name_with_number_formatter_options(Some(&tag_metadata_formatter), show_tag_color_names)
                                                        } else {
                                                            cclip_item.get_display_name_with_formatter_options(Some(&tag_metadata_formatter), show_tag_color_names)
                                                        };
                                                        
                                                        let mut item = Item::new_simple(
                                                            cclip_item.original_line.clone(),
                                                            display_name,
                                                            idx + 1
                                                        );
                                                        item.tags = Some(cclip_item.tags.clone());
                                                        item
                                                    })
                                                    .collect();
                                                
                                                // Preserve current selection by rowid (first field in original_line)
                                                let selected_rowid = ui.selected
                                                    .and_then(|idx| ui.shown.get(idx))
                                                    .and_then(|item| {
                                                        item.original_line.split('\t').next().map(|s| s.to_string())
                                                    });
                                                
                                                // Update UI with new items
                                                ui.hidden = new_items;
                                                ui.shown.clear();
                                                ui.filter();
                                                
                                                // Restore selection by finding the same rowid
                                                if let Some(ref rowid) = selected_rowid {
                                                    if let Some(pos) = ui.shown.iter().position(|item| {
                                                        item.original_line.split('\t').next() == Some(rowid.as_str())
                                                    }) {
                                                        ui.selected = Some(pos);
                                                    } else if !ui.shown.is_empty() {
                                                        ui.selected = Some(0);
                                                    }
                                                }
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
                                // parse the original cclip line to get rowid and mime_type
                                let original_line = &ui.shown[selected].original_line;
                                let parts: Vec<&str> = original_line.splitn(3, '\t').collect();
                                if parts.len() >= 2 {
                                    let rowid = parts[0];
                                    let mime_type = parts[1];
                                    
                                    // copy to clipboard using proper piping (no shell injection)
                                    let copy_result = if std::env::var("WAYLAND_DISPLAY").is_ok() {
                                        // wayland
                                        let cclip_child = std::process::Command::new("cclip")
                                            .args(&["get", rowid])
                                            .stdout(std::process::Stdio::piped())
                                            .stderr(std::process::Stdio::null())
                                            .spawn();
                                        
                                        if let Ok(mut cclip) = cclip_child {
                                            if let Some(cclip_stdout) = cclip.stdout.take() {
                                                let wl_copy = std::process::Command::new("wl-copy")
                                                    .args(&["-t", mime_type])
                                                    .stdin(std::process::Stdio::piped())
                                                    .stdout(std::process::Stdio::null())
                                                    .stderr(std::process::Stdio::null())
                                                    .spawn();
                                                
                                                if let Ok(mut wl) = wl_copy {
                                                    if let Some(wl_stdin) = wl.stdin.take() {
                                                        std::thread::spawn(move || {
                                                            let mut cclip_stdout = cclip_stdout;
                                                            let mut wl_stdin = wl_stdin;
                                                            std::io::copy(&mut cclip_stdout, &mut wl_stdin).ok();
                                                        });
                                                        
                                                        let _ = cclip.wait();
                                                        wl.wait().ok()
                                                    } else {
                                                        None
                                                    }
                                                } else {
                                                    None
                                                }
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    } else {
                                        // X11 - try xclip first, then xsel
                                        let x11_tools = [("xclip", vec!["-selection", "clipboard", "-t", mime_type]), 
                                                        ("xsel", vec!["--clipboard", "--input"])];
                                        
                                        let mut result = None;
                                        for (tool, args) in &x11_tools {
                                            let cclip_child = std::process::Command::new("cclip")
                                                .args(&["get", rowid])
                                                .stdout(std::process::Stdio::piped())
                                                .stderr(std::process::Stdio::null())
                                                .spawn();
                                            
                                            if let Ok(mut cclip) = cclip_child {
                                                if let Some(cclip_stdout) = cclip.stdout.take() {
                                                    let x11_child = std::process::Command::new(tool)
                                                        .args(args)
                                                        .stdin(std::process::Stdio::piped())
                                                        .stdout(std::process::Stdio::null())
                                                        .stderr(std::process::Stdio::null())
                                                        .spawn();
                                                    
                                                    if let Ok(mut x11) = x11_child {
                                                        if let Some(x11_stdin) = x11.stdin.take() {
                                                            std::thread::spawn(move || {
                                                                let mut cclip_stdout = cclip_stdout;
                                                                let mut x11_stdin = x11_stdin;
                                                                std::io::copy(&mut cclip_stdout, &mut x11_stdin).ok();
                                                            });
                                                            
                                                            let _ = cclip.wait();
                                                            if let Ok(status) = x11.wait() {
                                                                if status.success() {
                                                                    result = Some(status);
                                                                    break;
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        result
                                    };
                                    
                                    match copy_result {
                                        Some(status) if status.success() => {
                                            // clean up terminal completely
                                            terminal.show_cursor().wrap_err("Failed to show cursor")?;
                                            drop(terminal);
                                            let _ = io::stderr().execute(DisableMouseCapture);
                                            let _ = io::stderr().execute(LeaveAlternateScreen);
                                            let _ = disable_raw_mode();
                                            return Ok(());
                                        }
                                        _ => {
                                            // Ignore clipboard copy errors for now
                                        }
                                    }
                                }
                            }
                        }
                            }
                        }
                    }

                    // Add character to query or tag input
                    (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                        match &mut ui.tag_mode {
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
                        }
                    }
                    // Remove character from query or tag input
                    (KeyCode::Backspace, _) => {
                        match &mut ui.tag_mode {
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
                        }
                    }
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
                            let content_panel_height = get_cclip_u16(cli.cclip_title_panel_height_percent, cli.dmenu_title_panel_height_percent, cli.title_panel_height_percent);
                            let input_panel_height = get_cclip_u16(cli.cclip_input_panel_height, cli.dmenu_input_panel_height, cli.input_panel_height);
                            
                            // Use same calculation as rendering code
                            let content_height = (total_height as f32 * content_panel_height as f32 / 100.0).round() as u16;
                            let content_height = content_height.max(3);
                            let items_panel_height = total_height - content_height - input_panel_height;
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
                            TagMode::PromptingTagEmoji { .. } | TagMode::PromptingTagColor { .. } => {
                                // Disable navigation during emoji/color input
                                continue;
                            }
                            _ => {}
                        }
                        
                        if let Some(selected) = ui.selected {
                            let hard_stop = get_cclip_bool(cli.cclip_hard_stop, cli.dmenu_hard_stop, cli.hard_stop);
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
                                let content_panel_height = get_cclip_u16(cli.cclip_title_panel_height_percent, cli.dmenu_title_panel_height_percent, cli.title_panel_height_percent);
                                let input_panel_height = get_cclip_u16(cli.cclip_input_panel_height, cli.dmenu_input_panel_height, cli.input_panel_height);
                                
                                // Use same calculation as rendering code
                                let content_height = (total_height as f32 * content_panel_height as f32 / 100.0).round() as u16;
                                let content_height = content_height.max(3);
                                let items_panel_height = total_height - content_height - input_panel_height;
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
                            TagMode::PromptingTagEmoji { .. } | TagMode::PromptingTagColor { .. } => {
                                // Disable navigation during emoji/color input
                                continue;
                            }
                            _ => {}
                        }
                        
                        if let Some(selected) = ui.selected {
                            let hard_stop = get_cclip_bool(cli.cclip_hard_stop, cli.dmenu_hard_stop, cli.hard_stop);
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
                                let content_panel_height = get_cclip_u16(cli.cclip_title_panel_height_percent, cli.dmenu_title_panel_height_percent, cli.title_panel_height_percent);
                                let input_panel_height = get_cclip_u16(cli.cclip_input_panel_height, cli.dmenu_input_panel_height, cli.input_panel_height);
                                
                                // Use same calculation as rendering code
                                let content_height = (total_height as f32 * content_panel_height as f32 / 100.0).round() as u16;
                                let content_height = content_height.max(3);
                                let items_panel_height = total_height - content_height - input_panel_height;
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
                let content_panel_height = get_cclip_u16(cli.cclip_title_panel_height_percent, cli.dmenu_title_panel_height_percent, cli.title_panel_height_percent);
                let input_panel_height = get_cclip_u16(cli.cclip_input_panel_height, cli.dmenu_input_panel_height, cli.input_panel_height);
                
                // Use same calculation as rendering code
                let content_height = (total_height as f32 * content_panel_height as f32 / 100.0).round() as u16;
                let content_height = content_height.max(3);
                let items_panel_height = total_height - content_height - input_panel_height;
                
                // Get content panel position to calculate items panel position
                let content_panel_position = get_cclip_panel_position(cli.cclip_title_panel_position, cli.dmenu_title_panel_position, cli.title_panel_position.unwrap_or(crate::cli::PanelPosition::Top));
                
                // Calculate items panel coordinates based on layout
                let (items_panel_start, items_panel_height) = match content_panel_position {
                    crate::cli::PanelPosition::Top => {
                        // Top: content, items, input - items start after content
                        (content_height, items_panel_height)
                    },
                    crate::cli::PanelPosition::Middle => {
                        // Middle: items, content, input - items start at top
                        (0, items_panel_height)
                    },
                    crate::cli::PanelPosition::Bottom => {
                        // Bottom: items, input, content - items start at top
                        (0, items_panel_height)
                    }
                };
                
                let items_content_start = items_panel_start + 1; // +1 for top border
                let max_visible_rows = items_panel_height.saturating_sub(2); // -2 for borders
                let items_content_end = items_content_start + max_visible_rows;
                
                let update_selection_for_mouse_pos = |ui: &mut DmenuUI, mouse_row: u16| {
                    if !ui.shown.is_empty() && mouse_row >= items_content_start && mouse_row < items_content_end {
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
                        if mouse_row >= items_content_start && mouse_row < items_content_end && !ui.shown.is_empty() {
                            let row_in_content = mouse_row - items_content_start;
                            let clicked_item_index = ui.scroll_offset + row_in_content as usize;
                            
                            if clicked_item_index < ui.shown.len() {
                                // parse the original cclip line to get rowid and mime_type
                                let original_line = &ui.shown[clicked_item_index].original_line;
                                let parts: Vec<&str> = original_line.splitn(3, '\t').collect();
                                if parts.len() >= 2 {
                                    let rowid = parts[0];
                                    let mime_type = parts[1];
                                    
                                    // copy to clipboard using proper piping (no shell injection)
                                    let copy_result = if std::env::var("WAYLAND_DISPLAY").is_ok() {
                                        // wayland
                                        let cclip_child = std::process::Command::new("cclip")
                                            .args(&["get", rowid])
                                            .stdout(std::process::Stdio::piped())
                                            .stderr(std::process::Stdio::null())
                                            .spawn();
                                        
                                        if let Ok(mut cclip) = cclip_child {
                                            if let Some(cclip_stdout) = cclip.stdout.take() {
                                                let wl_copy = std::process::Command::new("wl-copy")
                                                    .args(&["-t", mime_type])
                                                    .stdin(std::process::Stdio::piped())
                                                    .stdout(std::process::Stdio::null())
                                                    .stderr(std::process::Stdio::null())
                                                    .spawn();
                                                
                                                if let Ok(mut wl) = wl_copy {
                                                    if let Some(wl_stdin) = wl.stdin.take() {
                                                        std::thread::spawn(move || {
                                                            let mut cclip_stdout = cclip_stdout;
                                                            let mut wl_stdin = wl_stdin;
                                                            std::io::copy(&mut cclip_stdout, &mut wl_stdin).ok();
                                                        });
                                                        
                                                        let _ = cclip.wait();
                                                        wl.wait().ok()
                                                    } else {
                                                        None
                                                    }
                                                } else {
                                                    None
                                                }
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    } else {
                                        // X11 - try xclip first, then xsel
                                        let x11_tools = [("xclip", vec!["-selection", "clipboard", "-t", mime_type]), 
                                                        ("xsel", vec!["--clipboard", "--input"])];
                                        
                                        let mut result = None;
                                        for (tool, args) in &x11_tools {
                                            let cclip_child = std::process::Command::new("cclip")
                                                .args(&["get", rowid])
                                                .stdout(std::process::Stdio::piped())
                                                .stderr(std::process::Stdio::null())
                                                .spawn();
                                            
                                            if let Ok(mut cclip) = cclip_child {
                                                if let Some(cclip_stdout) = cclip.stdout.take() {
                                                    let x11_child = std::process::Command::new(tool)
                                                        .args(args)
                                                        .stdin(std::process::Stdio::piped())
                                                        .stdout(std::process::Stdio::null())
                                                        .stderr(std::process::Stdio::null())
                                                        .spawn();
                                                    
                                                    if let Ok(mut x11) = x11_child {
                                                        if let Some(x11_stdin) = x11.stdin.take() {
                                                            std::thread::spawn(move || {
                                                                let mut cclip_stdout = cclip_stdout;
                                                                let mut x11_stdin = x11_stdin;
                                                                std::io::copy(&mut cclip_stdout, &mut x11_stdin).ok();
                                                            });
                                                            
                                                            let _ = cclip.wait();
                                                            if let Ok(status) = x11.wait() {
                                                                if status.success() {
                                                                    result = Some(status);
                                                                    break;
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        result
                                    };
                                    
                                    match copy_result {
                                        Some(status) if status.success() => {
                                            // clean up terminal completely
                                            terminal.show_cursor().wrap_err("Failed to show cursor")?;
                                            drop(terminal);
                                            if !disable_mouse {
                                                let _ = io::stderr().execute(DisableMouseCapture);
                                            }
                                            let _ = io::stderr().execute(LeaveAlternateScreen);
                                            let _ = disable_raw_mode();
                                            return Ok(());
                                        }
                                        _ => {
                                            // ignore clipboard copy errors for now
                                        }
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
        }
    }
}
