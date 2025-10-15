#![deny(unsafe_code)]
#![deny(missing_docs)]

//! # Fsel
//!
//! > _Blazing fast_ TUI launcher for GNU/Linux and *BSD
//!
//! For more info, check the [README](https://github.com/Mjoyufull/fsel)

/// CLI parser
mod cli;
/// Desktop file and history cache
mod cache;
/// Clipboard history integration
mod cclip;
/// Dmenu functionality
mod dmenu;
/// Terminal graphics handling (inspired by Yazi)
mod graphics;
/// Helper functions
mod helpers;
/// Terminal input helpers
mod input;
/// Keybind configuration
mod keybinds;
/// UI helpers
mod ui;
/// XDG apps
mod xdg;

use input::Event;
use ui::{UI, DmenuUI};
use dmenu::{is_stdin_piped, read_stdin_lines, parse_stdin_to_items};

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io;

use std::path;
use std::process;

use redb::ReadableTable;


use directories::ProjectDirs;
use eyre::eyre;
use eyre::WrapErr;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;
use crossterm::{
    event::{EnableMouseCapture, DisableMouseCapture, MouseButton, MouseEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use scopeguard::defer;

fn main() {
    if let Err(error) = real_main() {
        shutdown_terminal(false); // Use safe default - always cleanup mouse if enabled
        eprintln!("{error:?}");
        process::exit(1);
    }
}

fn setup_terminal(disable_mouse: bool) -> eyre::Result<()> {
    enable_raw_mode().wrap_err("Failed to enable raw mode")?;
    io::stderr().execute(EnterAlternateScreen).wrap_err("Failed to enter alternate screen")?;
    if !disable_mouse {
        io::stderr().execute(EnableMouseCapture).wrap_err("Failed to enable mouse capture")?;
    }
    Ok(())
}

fn shutdown_terminal(disable_mouse: bool) {
    if !disable_mouse {
        let _ = io::stderr().execute(DisableMouseCapture);
    }
    let _ = io::stderr().execute(LeaveAlternateScreen);
    let _ = disable_raw_mode();
}

fn find_processes_holding_file(path: &path::Path) -> io::Result<Vec<i32>> {
    let mut holders = Vec::new();

    if !path.exists() {
        return Ok(holders);
    }

    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let canonical_str = canonical.to_string_lossy();

    let proc_entries = match fs::read_dir("/proc") {
        Ok(entries) => entries,
        Err(_) => return Ok(holders),
    };

    for entry in proc_entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let file_name = entry.file_name();
        let pid: i32 = match file_name.to_str().and_then(|s| s.parse().ok()) {
            Some(pid) => pid,
            None => continue,
        };

        let fd_dir = entry.path().join("fd");
        let fd_entries = match fs::read_dir(fd_dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for fd_entry in fd_entries {
            let fd_entry = match fd_entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let target = match fs::read_link(fd_entry.path()) {
                Ok(t) => t,
                Err(_) => continue,
            };

            if target == canonical {
                holders.push(pid);
                break;
            }

            if let Some(target_str) = target.to_str() {
                if target_str.starts_with(canonical_str.as_ref()) {
                    holders.push(pid);
                    break;
                }
            }
        }
    }

    Ok(holders)
}

async fn run_cclip_mode(cli: &cli::Opts) -> eyre::Result<()> {
    use crossterm::event::{KeyCode, KeyModifiers};

    // Check if cclip is available
    if !cclip::check_cclip_available() {
        return Err(eyre!("cclip is not available. Please install cclip and ensure it's in your PATH."));
    }
    
    // Check if cclip database is accessible
    cclip::check_cclip_database()
        .wrap_err("cclip database check failed")?;

    // Handle tag list mode
    if cli.cclip_tag_list {
        let tags = cclip::get_all_tags()
            .wrap_err("Failed to get tags from cclip")?;
        
        if tags.is_empty() {
            println!("No tags found");
            return Ok(());
        }
        
        // If specific tag requested, show items in that tag
        if let Some(ref tag_name) = cli.cclip_tag {
            println!("Items tagged with '{}':", tag_name);
            let items = cclip::get_clipboard_history_by_tag(tag_name)
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
                    let items = cclip::get_clipboard_history_by_tag(&tag)
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
        cclip::get_clipboard_history_by_tag(tag_name)
            .wrap_err(format!("Failed to get clipboard history for tag '{}'", tag_name))?
    } else {
        cclip::get_clipboard_history()
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

    // Convert to DmenuItems
    let items: Vec<dmenu::DmenuItem> = cclip_items
        .into_iter()
        .enumerate()
        .map(|(idx, cclip_item)| {
            // Use numbered display name if show_line_numbers is enabled
            let display_name = if show_line_numbers {
                // Use database rowid (shows actual DB ID)
                cclip_item.get_display_name_with_number()
            } else {
                cclip_item.get_display_name()
            };
            
            dmenu::DmenuItem::new_simple(
                cclip_item.original_line.clone(),
                display_name,
                idx + 1
            )
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

    // Input handler
    let input = input::Config {
        disable_mouse: disable_mouse,
        ..input::Config::default()
    }.init();

    // Create dmenu UI using cclip settings with inheritance  
    let wrap_long_lines = cli.cclip_wrap_long_lines.or(Some(cli.dmenu_wrap_long_lines)).unwrap_or(true);
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
    let chafa_available = cclip::check_chafa_available();
    
    // enable inline preview for both Kitty and Sixel protocols
    let graphics_adapter = crate::graphics::GraphicsAdapter::detect();
    let supports_graphics = !matches!(graphics_adapter, crate::graphics::GraphicsAdapter::None);
    
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
        // Update UI content BEFORE drawing to avoid race conditions with graphics clearing
        let content_panel_width = terminal.size()?.width.saturating_sub(2);
        let content_panel_height = (terminal.size()?.height as f32 * get_cclip_u16(cli.cclip_title_panel_height_percent, cli.dmenu_title_panel_height_percent, cli.title_panel_height_percent) as f32 / 100.0).round() as u16;
        let content_panel_height = content_panel_height.max(3).saturating_sub(2);
        
        // DISABLED: Tag mode UI (waiting for cclip maintainer to add tag support)
        // match &ui.tag_mode {
        //     ui::TagMode::PromptingTagName { .. } => {
        //         ui.text = vec![
        //             Line::from(vec![
        //                 Span::styled("Tagging Mode", Style::default().add_modifier(Modifier::BOLD)),
        //             ]),
        //             Line::from(""),
        //             Line::from("Enter a tag name for this clipboard item."),
        //             Line::from(""),
        //             Line::from("Examples: prompt, code, important, todo"),
        //             Line::from(""),
        //             Line::from("Press Enter to continue, Esc to cancel."),
        //         ];
        //     }
        //     ui::TagMode::PromptingTagColor { tag_name, .. } => {
        //         ui.text = vec![
        //             Line::from(vec![
        //                 Span::styled("Tag Color", Style::default().add_modifier(Modifier::BOLD)),
        //             ]),
        //             Line::from(""),
        //             Line::from(format!("Tag: {}", tag_name)),
        //             Line::from(""),
        //             Line::from("Enter a color (optional):"),
        //             Line::from("  - Hex: #ff0000 or #f00"),
        //             Line::from("  - RGB: rgb(255,0,0)"),
        //             Line::from("  - Named: red, blue, green, etc."),
        //             Line::from("  - Leave blank for default"),
        //             Line::from(""),
        //             Line::from("Press Enter to continue, Esc to cancel."),
        //         ];
        //     }
        //     ui::TagMode::PromptingTagEmoji { tag_name, color, .. } => {
        //         ui.text = vec![
        //             Line::from(vec![
        //                 Span::styled("Tag Emoji", Style::default().add_modifier(Modifier::BOLD)),
        //             ]),
        //             Line::from(""),
        //             Line::from(format!("Tag: {}", tag_name)),
        //             Line::from(format!("Color: {}", color.as_ref().unwrap_or(&"default".to_string()))),
        //             Line::from(""),
        //             Line::from("Enter an emoji prefix (optional):"),
        //             Line::from("  Examples: 📌 🔥 ⭐ 💡 📝"),
        //             Line::from("  Leave blank for no emoji"),
        //             Line::from(""),
        //             Line::from("Press Enter to finish, Esc to cancel."),
        //         ];
        //     }
        //     ui::TagMode::Normal => {
                ui.info_with_image_support(
                    get_cclip_color(cli.cclip_highlight_color, cli.dmenu_highlight_color, cli.highlight_color),
                    image_preview_enabled,
                    hide_image_message,
                    content_panel_width,
                    content_panel_height
                );
        //     }
        // }
        
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
        
        // BEFORE drawing: Clear old image if transitioning from image to non-image
        if image_preview_enabled && previous_was_image && !current_is_image {
            let graphics = crate::graphics::GraphicsAdapter::detect();
            let _ = graphics.image_hide();
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
            
            let content_paragraph = Paragraph::new(ui.text.clone())
                .block(content_block)
                .style(Style::default().fg(main_text_color))
                .wrap(Wrap { trim: false })
                .alignment(Alignment::Left);
            
            // Items panel
            let items_panel_height = chunks[items_panel_index].height;
            let max_visible = items_panel_height.saturating_sub(2) as usize;
            
            let visible_items = ui.shown
                .iter()
                .skip(ui.scroll_offset)
                .take(max_visible)
                .map(ListItem::from)
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
                .highlight_style(
                    Style::default()
                        .fg(highlight_color)
                        .add_modifier(Modifier::BOLD),
                )
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
            
            // Input panel
            // DISABLED: Tag mode input panel (waiting for cclip maintainer to add tag support)
            // let (input_line, input_title) = match &ui.tag_mode {
            //     ui::TagMode::PromptingTagName { input } => {
            //         (Line::from(vec![
            //             Span::styled("Tag: ", Style::default().fg(highlight_color)),
            //             Span::styled(input, Style::default().fg(input_text_color)),
            //             Span::styled(cursor, Style::default().fg(highlight_color)),
            //         ]), " Tag Name ")
            //     }
            //     ui::TagMode::PromptingTagColor { input, .. } => {
            //         (Line::from(vec![
            //             Span::styled("Color: ", Style::default().fg(highlight_color)),
            //             Span::styled(input, Style::default().fg(input_text_color)),
            //             Span::styled(cursor, Style::default().fg(highlight_color)),
            //             Span::styled(" (hex/name or blank)", Style::default().fg(input_text_color).add_modifier(Modifier::DIM)),
            //         ]), " Tag Color ")
            //     }
            //     ui::TagMode::PromptingTagEmoji { input, .. } => {
            //         (Line::from(vec![
            //             Span::styled("Emoji: ", Style::default().fg(highlight_color)),
            //             Span::styled(input, Style::default().fg(input_text_color)),
            //             Span::styled(cursor, Style::default().fg(highlight_color)),
            //             Span::styled(" (or blank)", Style::default().fg(input_text_color).add_modifier(Modifier::DIM)),
            //         ]), " Tag Emoji ")
            //     }
            //     ui::TagMode::Normal => {
                    let (input_line, input_title) = (Line::from(vec![
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
                    ]), " Filter ");
            //     }
            // };
            
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
            let graphics = crate::graphics::GraphicsAdapter::detect();
            
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
                    // DISABLED: Tag keybind (waiting for cclip maintainer to add tag support)
                    // (code, mods) if cli.keybinds.matches_tag(code, mods) => {
                    //     if ui.selected.is_some() && !ui.shown.is_empty() {
                    //         ui.tag_mode = ui::TagMode::PromptingTagName { input: String::new() };
                    //     }
                    // }
                    // Exit on escape or Ctrl+C/Q
                    (KeyCode::Esc, _) | (KeyCode::Char('q'), KeyModifiers::CONTROL) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        // DISABLED: Tag mode cancellation (waiting for cclip maintainer to add tag support)
                        // if ui.tag_mode != ui::TagMode::Normal {
                        //     ui.tag_mode = ui::TagMode::Normal;
                        // } else {
                            return Ok(()); // Exit without copying
                        // }
                    }
                    // Handle Enter key (clipboard copy)
                    (KeyCode::Enter, _) | (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                        // DISABLED: Tag mode progression (waiting for cclip maintainer to add tag support)
                        // match ui.tag_mode.clone() {
                        //     ui::TagMode::PromptingTagName { input } => {
                        //         if input.trim().is_empty() {
                        //             ui.tag_mode = ui::TagMode::Normal;
                        //         } else {
                        //             ui.tag_mode = ui::TagMode::PromptingTagColor {
                        //                 tag_name: input.trim().to_string(),
                        //                 input: String::new(),
                        //             };
                        //         }
                        //     }
                        //     ui::TagMode::PromptingTagColor { tag_name, input } => {
                        //         let color = if input.trim().is_empty() {
                        //             None
                        //         } else {
                        //             Some(input.trim().to_string())
                        //         };
                        //         ui.tag_mode = ui::TagMode::PromptingTagEmoji {
                        //             tag_name,
                        //             color,
                        //             input: String::new(),
                        //         };
                        //     }
                        //     ui::TagMode::PromptingTagEmoji { tag_name, color, input } => {
                        //         let emoji = if input.trim().is_empty() {
                        //             None
                        //         } else {
                        //             Some(input.trim().to_string())
                        //         };
                        //         
                        //         if let Some(selected) = ui.selected {
                        //             if selected < ui.shown.len() {
                        //                 let item = &ui.shown[selected];
                        //                 if let Some(rowid) = ui.get_cclip_rowid_any(item) {
                        //                     if let Err(e) = cclip::tag_item(&rowid, &tag_name) {
                        //                         eprintln!("Failed to tag item: {}", e);
                        //                     } else {
                        //                         let (db, _) = helpers::open_history_db()?;
                        //                         let mut tag_metadata = cclip::load_tag_metadata(&db);
                        //                         let metadata = cclip::TagMetadata::new(tag_name.clone());
                        //                         let metadata = if let Some(c) = color {
                        //                             metadata.with_color(c)
                        //                         } else {
                        //                             metadata
                        //                         };
                        //                         let metadata = if let Some(e) = emoji {
                        //                             metadata.with_emoji(e)
                        //                         } else {
                        //                             metadata
                        //                         };
                        //                         tag_metadata.insert(tag_name, metadata);
                        //                         let _ = cclip::save_tag_metadata(&db, &tag_metadata);
                        //                     }
                        //                 }
                        //             }
                        //         }
                        //         
                        //         ui.tag_mode = ui::TagMode::Normal;
                        //     }
                        //     ui::TagMode::Normal => {
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

                    // Add character to query
                    (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                        // DISABLED: Tag mode input (waiting for cclip maintainer to add tag support)
                        // match &mut ui.tag_mode {
                        //     ui::TagMode::PromptingTagName { input } => {
                        //         input.push(c);
                        //     }
                        //     ui::TagMode::PromptingTagColor { input, .. } => {
                        //         input.push(c);
                        //     }
                        //     ui::TagMode::PromptingTagEmoji { input, .. } => {
                        //         input.push(c);
                        //     }
                        //     ui::TagMode::Normal => {
                                ui.query.push(c);
                                ui.filter();
                        //     }
                        // }
                    }
                    // Remove character from query
                    (KeyCode::Backspace, _) => {
                        // DISABLED: Tag mode input (waiting for cclip maintainer to add tag support)
                        // match &mut ui.tag_mode {
                        //     ui::TagMode::PromptingTagName { input } => {
                        //         input.pop();
                        //     }
                        //     ui::TagMode::PromptingTagColor { input, .. } => {
                        //         input.pop();
                        //     }
                        //     ui::TagMode::PromptingTagEmoji { input, .. } => {
                        //         input.pop();
                        //     }
                        //     ui::TagMode::Normal => {
                                ui.query.pop();
                                ui.filter();
                        //     }
                        // }
                    }
                    // Navigation - Left: go to first item
                    (KeyCode::Left, _) => {
                        if !ui.shown.is_empty() {
                            ui.selected = Some(0);
                            ui.scroll_offset = 0;
                        }
                    }
                    // Navigation - Right: go to last item
                    (KeyCode::Right, _) => {
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
                    // Navigation - Down: next item with scrolling
                    (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
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
                    // Navigation - Up: previous item with scrolling
                    (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
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
                        update_selection_for_mouse_pos(&mut ui, mouse_row);
                    }
                    MouseEventKind::Down(MouseButton::Left) => {
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
                        if !ui.shown.is_empty() && ui.scroll_offset > 0 {
                            ui.scroll_offset -= 1;
                            update_selection_for_mouse_pos(&mut ui, mouse_row);
                        }
                    }
                    MouseEventKind::ScrollDown => {
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

fn run_dmenu_mode(cli: &cli::Opts) -> eyre::Result<()> {
    use crossterm::{
        event::{KeyCode, KeyModifiers},
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        ExecutableCommand,
    };
    use crossterm::event::{EnableMouseCapture, DisableMouseCapture, MouseButton, MouseEventKind};
    use ratatui::backend::CrosstermBackend;
    use ratatui::layout::{Alignment, Constraint, Direction, Layout};
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap};
    use ratatui::Terminal;

    // Check if stdin is piped (unless prompt-only mode)
    if !cli.dmenu_prompt_only && !is_stdin_piped() {
        return Err(eyre!("dmenu mode requires input from stdin"));
    }

    // Read stdin lines
    let lines = if cli.dmenu_prompt_only {
        vec![] // No input in prompt-only mode
    } else if cli.dmenu_null_separated {
        dmenu::read_stdin_null_separated()
            .wrap_err("Failed to read from stdin")?
    } else {
        read_stdin_lines()
            .wrap_err("Failed to read from stdin")?
    };
    
    // Exit immediately if no input and exit_if_empty is set
    if cli.dmenu_exit_if_empty && lines.is_empty() {
        return Ok(());
    }
    
    // Also check if lines only contain empty strings
    if cli.dmenu_exit_if_empty && lines.iter().all(|l| l.trim().is_empty()) {
        return Ok(());
    }

    // Parse items
    let items = parse_stdin_to_items(
        lines,
        &cli.dmenu_delimiter,
        cli.dmenu_with_nth.as_ref(),
    );

    // Setup terminal
    enable_raw_mode().wrap_err("Failed to enable raw mode")?;
    io::stderr().execute(EnterAlternateScreen).wrap_err("Failed to enter alternate screen")?;
    
    // Get effective disable_mouse setting with dmenu -> regular inheritance
    let disable_mouse = cli.dmenu_disable_mouse.unwrap_or(cli.disable_mouse);
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

    // Initialize terminal using stderr to keep stdout clean for dmenu output
    let backend = CrosstermBackend::new(io::stderr());
    let mut terminal = Terminal::new(backend).wrap_err("Failed to start crossterm terminal")?;
    terminal.hide_cursor().wrap_err("Failed to hide cursor")?;
    terminal.clear().wrap_err("Failed to clear terminal")?;

    // Input handler
    let input = input::Config {
        disable_mouse: disable_mouse,
        ..input::Config::default()
    }.init();

    // Create dmenu UI
    let mut ui = DmenuUI::new(items, cli.dmenu_wrap_long_lines, cli.dmenu_show_line_numbers);
    ui.set_match_mode(cli.match_mode);
    ui.set_match_nth(cli.dmenu_match_nth.clone());
    
    // Pre-fill search if -ss was provided
    if let Some(ref search) = cli.search_string {
        ui.query = search.clone();
    }
    
    ui.filter(); // Initial filter to show all items (or filtered by search_string)
    
    // Handle pre-selection
    if let Some(ref select_str) = cli.dmenu_select {
        // Find first matching item (case-insensitive)
        let select_lower = select_str.to_lowercase();
        for (idx, item) in ui.shown.iter().enumerate() {
            if item.display_text.to_lowercase().contains(&select_lower) {
                ui.selected = Some(idx);
                break;
            }
        }
    } else if let Some(select_idx) = cli.dmenu_select_index {
        if select_idx < ui.shown.len() {
            ui.selected = Some(select_idx);
        }
    }
    
    // Ensure we have a valid selection if there are items
    if !ui.shown.is_empty() && ui.selected.is_none() {
        ui.selected = Some(0);
    }
    
    ui.info(cli.dmenu_highlight_color.unwrap_or(cli.highlight_color));
    
    // List state for ratatui
    let mut list_state = ListState::default();
    
    // Get effective dmenu colors with fallback
    let get_dmenu_color = |dmenu_opt: Option<ratatui::style::Color>, default: ratatui::style::Color| {
        dmenu_opt.unwrap_or(default)
    };
    let get_dmenu_bool = |dmenu_opt: Option<bool>, default: bool| {
        dmenu_opt.unwrap_or(default)
    };
    let get_dmenu_u16 = |dmenu_opt: Option<u16>, default: u16| {
        dmenu_opt.unwrap_or(default)
    };
    let get_dmenu_panel_position = |dmenu_opt: Option<crate::cli::PanelPosition>, default: crate::cli::PanelPosition| {
        dmenu_opt.unwrap_or(default)
    };
    // Get effective cursor string
    let cursor = cli.dmenu_cursor.as_ref().unwrap_or(&cli.cursor);

    // Main TUI loop
    loop {
        terminal.draw(|f| {
            // Get effective colors and settings for dmenu mode
            let highlight_color = get_dmenu_color(cli.dmenu_highlight_color, cli.highlight_color);
            let main_border_color = get_dmenu_color(cli.dmenu_main_border_color, cli.main_border_color);
            let items_border_color = get_dmenu_color(cli.dmenu_items_border_color, cli.apps_border_color);
            let input_border_color = get_dmenu_color(cli.dmenu_input_border_color, cli.input_border_color);
            let main_text_color = get_dmenu_color(cli.dmenu_main_text_color, cli.main_text_color);
            let items_text_color = get_dmenu_color(cli.dmenu_items_text_color, cli.apps_text_color);
            let input_text_color = get_dmenu_color(cli.dmenu_input_text_color, cli.input_text_color);
            let header_title_color = get_dmenu_color(cli.dmenu_header_title_color, cli.header_title_color);
            let rounded_borders = get_dmenu_bool(cli.dmenu_rounded_borders, cli.rounded_borders);
            let content_panel_height = get_dmenu_u16(cli.dmenu_title_panel_height_percent, cli.title_panel_height_percent);
            let input_panel_height = get_dmenu_u16(cli.dmenu_input_panel_height, cli.input_panel_height);
            
            // Layout calculation
            let total_height = f.area().height;
            let content_height = (total_height as f32 * content_panel_height as f32 / 100.0).round() as u16;
            
            // Get content panel position (defaults to Top if not set)
            let content_panel_position = get_dmenu_panel_position(cli.dmenu_title_panel_position, cli.title_panel_position.unwrap_or(crate::cli::PanelPosition::Top));
            
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
            
            // Content panel (shows selected item's full content)
            let content_block = Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    " Content ",
                    Style::default().add_modifier(Modifier::BOLD).fg(header_title_color),
                ))
                .border_type(border_type)
                .border_style(Style::default().fg(main_border_color));
            
            let content_paragraph = Paragraph::new(ui.text.clone())
                .block(content_block)
                .style(Style::default().fg(main_text_color))
                .wrap(Wrap { trim: false })
                .alignment(Alignment::Left);
            
            // Items panel
            let items_panel_height = chunks[items_panel_index].height;
            let max_visible = items_panel_height.saturating_sub(2) as usize;
            
            let visible_items = ui.shown
                .iter()
                .skip(ui.scroll_offset)
                .take(max_visible)
                .map(ListItem::from)
                .collect::<Vec<ListItem>>();
            
            let items_list = List::new(visible_items)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        " Items ",
                        Style::default().add_modifier(Modifier::BOLD).fg(header_title_color),
                    ))
                    .border_type(border_type)
                    .border_style(Style::default().fg(items_border_color))
                )
                .style(Style::default().fg(items_text_color))
                .highlight_style(
                    Style::default()
                        .fg(highlight_color)
                        .add_modifier(Modifier::BOLD),
                )
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
            
            // Input panel
            let input_paragraph = Paragraph::new(Line::from(vec![
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
                Span::styled(
                    if cli.dmenu_password_mode {
                        cli.dmenu_password_character.repeat(ui.query.len())
                    } else {
                        ui.query.clone()
                    },
                    Style::default().fg(input_text_color)
                ),
                Span::styled(cursor, Style::default().fg(highlight_color)),
            ]))
            .block(Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    if cli.dmenu_prompt_only { " Input " } else { " Filter " },
                    Style::default().add_modifier(Modifier::BOLD).fg(header_title_color),
                ))
                .border_type(border_type)
                .border_style(Style::default().fg(input_border_color))
            )
            .style(Style::default().fg(input_text_color))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false });
            
            // Render all components in their dynamic positions
            // Only render content panel if not hide_before_typing or query is not empty
            if !cli.dmenu_hide_before_typing || !ui.query.is_empty() {
                f.render_widget(content_paragraph, chunks[content_panel_index]);
            }
            // Only render items list if not in prompt-only mode and (not hide_before_typing or query is not empty)
            if !cli.dmenu_prompt_only && (!cli.dmenu_hide_before_typing || !ui.query.is_empty()) {
                f.render_stateful_widget(items_list, chunks[items_panel_index], &mut list_state);
            }
            f.render_widget(input_paragraph, chunks[input_panel_index]);
        })?;
        
        // Handle input events
        match input.next()? {
            Event::Input(key) => {
                match (key.code, key.modifiers) {
                    // Exit on escape or Ctrl+C/Q
                    (KeyCode::Esc, _) | (KeyCode::Char('q'), KeyModifiers::CONTROL) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        return Ok(()); // Exit without output
                    }
                    // Select item on Enter or Ctrl+Y
                    (KeyCode::Enter, _) | (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                        // Auto-select if only one match and auto_select is enabled
                        if cli.dmenu_auto_select && ui.shown.len() == 1 {
                            ui.selected = Some(0);
                        }
                        
                        // Store selection and exit loop to handle output outside TUI context
                        if let Some(selected) = ui.selected {
                            if selected < ui.shown.len() {
                                let output = if cli.dmenu_index_mode {
                                    // Output index instead of text
                                    selected.to_string()
                                } else if let Some(ref accept_cols) = cli.dmenu_accept_nth {
                                    // Output specific columns
                                    ui.shown[selected].get_accept_nth_output(accept_cols)
                                } else {
                                    // Output original line
                                    ui.shown[selected].original_line.clone()
                                };
                                
                                // Clean up terminal completely
                                terminal.show_cursor().wrap_err("Failed to show cursor")?;
                                drop(terminal);
                                if !disable_mouse {
                                    let _ = io::stderr().execute(DisableMouseCapture);
                                }
                                let _ = io::stderr().execute(LeaveAlternateScreen);
                                let _ = disable_raw_mode();
                                
                                // Print to stdout
                                println!("{}", output);
                                return Ok(());
                            }
                        } else if !cli.dmenu_only_match && !ui.query.is_empty() {
                            // No selection but have query - output the query itself (unless only_match is set)
                            terminal.show_cursor().wrap_err("Failed to show cursor")?;
                            drop(terminal);
                            if !disable_mouse {
                                let _ = io::stderr().execute(DisableMouseCapture);
                            }
                            let _ = io::stderr().execute(LeaveAlternateScreen);
                            let _ = disable_raw_mode();
                            
                            println!("{}", ui.query);
                            return Ok(());
                        }
                        
                        // only_match is set and no selection - don't exit
                        if cli.dmenu_only_match {
                            continue;
                        }
                        
                        return Ok(()); // Exit without selection
                    }
                    // Add character to query
                    (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                        ui.query.push(c);
                        ui.filter();
                        
                        // Auto-select if only one match
                        if cli.dmenu_auto_select && ui.shown.len() == 1 {
                            ui.selected = Some(0);
                        }
                    }
                    // Remove character from query
                    (KeyCode::Backspace, _) => {
                        ui.query.pop();
                        ui.filter();
                        
                        // Auto-select if only one match
                        if cli.dmenu_auto_select && ui.shown.len() == 1 {
                            ui.selected = Some(0);
                        }
                    }
                    // Navigation
                    (KeyCode::Left, _) => {
                        if !ui.shown.is_empty() {
                            ui.selected = Some(0);
                            ui.scroll_offset = 0;
                        }
                    }
                    (KeyCode::Right, _) => {
                        if !ui.shown.is_empty() {
                            let last_index = ui.shown.len() - 1;
                            ui.selected = Some(last_index);
                            
                            // Scroll to show last item
                            let total_height = terminal.size()?.height;
                            let content_panel_height = get_dmenu_u16(cli.dmenu_title_panel_height_percent, cli.title_panel_height_percent);
                            let input_panel_height = get_dmenu_u16(cli.dmenu_input_panel_height, cli.input_panel_height);
                            
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
                    (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                        if let Some(selected) = ui.selected {
                            let hard_stop = get_dmenu_bool(cli.dmenu_hard_stop, cli.hard_stop);
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
                                let content_panel_height = get_dmenu_u16(cli.dmenu_title_panel_height_percent, cli.title_panel_height_percent);
                                let input_panel_height = get_dmenu_u16(cli.dmenu_input_panel_height, cli.input_panel_height);
                                
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
                    (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                        if let Some(selected) = ui.selected {
                            let hard_stop = get_dmenu_bool(cli.dmenu_hard_stop, cli.hard_stop);
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
                                let content_panel_height = get_dmenu_u16(cli.dmenu_title_panel_height_percent, cli.title_panel_height_percent);
                                let input_panel_height = get_dmenu_u16(cli.dmenu_input_panel_height, cli.input_panel_height);
                                
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
                
                // Update info display
                ui.info(get_dmenu_color(cli.dmenu_highlight_color, cli.highlight_color));
            }
            Event::Mouse(mouse_event) => {
                // Dmenu-specific mouse handling with proper layout calculations
                let mouse_row = mouse_event.row;
                let total_height = terminal.size()?.height;
                let content_panel_height = get_dmenu_u16(cli.dmenu_title_panel_height_percent, cli.title_panel_height_percent);
                let input_panel_height = get_dmenu_u16(cli.dmenu_input_panel_height, cli.input_panel_height);
                
                // Use same calculation as rendering code
                let content_height = (total_height as f32 * content_panel_height as f32 / 100.0).round() as u16;
                let content_height = content_height.max(3);
                let items_panel_height = total_height - content_height - input_panel_height;
                
                // Get content panel position to calculate items panel position
                let content_panel_position = get_dmenu_panel_position(cli.dmenu_title_panel_position, cli.title_panel_position.unwrap_or(crate::cli::PanelPosition::Top));
                
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
                            ui.info(get_dmenu_color(cli.dmenu_highlight_color, cli.highlight_color));
                        }
                    }
                };
                
                match mouse_event.kind {
                    MouseEventKind::Moved => {
                        update_selection_for_mouse_pos(&mut ui, mouse_row);
                    }
                    MouseEventKind::Down(MouseButton::Left) => {
                        if mouse_row >= items_content_start && mouse_row < items_content_end && !ui.shown.is_empty() {
                            let row_in_content = mouse_row - items_content_start;
                            let clicked_item_index = ui.scroll_offset + row_in_content as usize;
                            
                            if clicked_item_index < ui.shown.len() {
                                // Store the original line as-is for dmenu output
                                let selected_line = &ui.shown[clicked_item_index].original_line;
                                
                                // Clean up terminal completely
                                terminal.show_cursor().wrap_err("Failed to show cursor")?;
                                drop(terminal); // Ensure terminal is fully cleaned up
                                if !disable_mouse {
                                    let _ = io::stderr().execute(DisableMouseCapture);
                                }
                                let _ = io::stderr().execute(LeaveAlternateScreen);
                                let _ = disable_raw_mode();
                                
                                // Output selection in clean context
                                println!("{}", selected_line);
                                return Ok(());
                            }
                        }
                    }
                    MouseEventKind::ScrollUp => {
                        if !ui.shown.is_empty() && ui.scroll_offset > 0 {
                            ui.scroll_offset -= 1;
                            // Update selection to match current mouse position after scrolling
                            update_selection_for_mouse_pos(&mut ui, mouse_row);
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        if !ui.shown.is_empty() {
                            // Calculate maximum visible items (account for borders)
                            let max_visible = max_visible_rows as usize;
                            
                            // Only scroll down if there are more items to show
                            if ui.scroll_offset + max_visible < ui.shown.len() {
                                ui.scroll_offset += 1;
                                // Update selection to match current mouse position after scrolling
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

/// Fast app lookup by exact name - uses name index for instant lookup
fn find_app_by_name_fast(db: &std::sync::Arc<redb::Database>, app_name: &str, cli: &cli::Opts) -> eyre::Result<Option<xdg::App>> {
    let desktop_cache = cache::DesktopCache::new(db.clone())?;
    let history_cache = cache::HistoryCache::load(db)?;
    
    // Try the name index first - this is instant if the app is cached
    if let Ok(Some(app)) = desktop_cache.get_by_name(app_name) {
        // Apply filtering if needed
        if cli.filter_desktop {
            if let Ok(current_desktop) = env::var("XDG_CURRENT_DESKTOP") {
                let desktops: Vec<String> = current_desktop.split(':').map(|s| s.to_string()).collect();
                
                if !app.not_show_in.is_empty() {
                    let should_hide = app.not_show_in.iter()
                        .any(|d| desktops.iter().any(|cd| cd.eq_ignore_ascii_case(d)));
                    if should_hide {
                        return Ok(None);
                    }
                }
                
                if !app.only_show_in.is_empty() {
                    let should_show = app.only_show_in.iter()
                        .any(|d| desktops.iter().any(|cd| cd.eq_ignore_ascii_case(d)));
                    if !should_show {
                        return Ok(None);
                    }
                }
            }
        }
        
        // Found in cache! Apply history and return
        return Ok(Some(history_cache.apply_to_app(app)));
    }
    
    // Not in cache - need to search for it
    use walkdir::WalkDir;
    
    let mut dirs: Vec<path::PathBuf> = vec![];
    
    // Add user's data directory
    if let Some(xdg_data_home) = env::var("XDG_DATA_HOME").ok().filter(|s| !s.is_empty()) {
        let mut dir = path::PathBuf::from(xdg_data_home);
        dir.push("applications");
        if dir.exists() {
            dirs.push(dir);
        }
    } else if let Some(home_dir) = dirs::home_dir() {
        let mut dir = home_dir;
        dir.push(".local/share/applications");
        if dir.exists() {
            dirs.push(dir);
        }
    }
    
    // Add system data directories
    if let Ok(res) = env::var("XDG_DATA_DIRS") {
        for data_dir in res.split(':').filter(|s| !s.is_empty()) {
            let mut dir = path::PathBuf::from(data_dir);
            dir.push("applications");
            if dir.exists() {
                dirs.push(dir);
            }
        }
    } else {
        let mut default_paths = vec![
            path::PathBuf::from("/usr/local/share"),
            path::PathBuf::from("/usr/share"),
        ];
        
        #[cfg(target_os = "openbsd")]
        {
            default_paths.push(path::PathBuf::from("/usr/X11R6/share"));
        }
        
        for data_dir in &mut default_paths {
            data_dir.push("applications");
            if data_dir.exists() {
                dirs.push(data_dir.clone());
            }
        }
    }
    
    let desktop_cache = cache::DesktopCache::new(db.clone())?;
    let history_cache = cache::HistoryCache::load(db)?;
    
    // Search for the specific app
    for dir in &dirs {
        for entry in WalkDir::new(dir)
            .min_depth(1)
            .max_depth(5)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| {
                !entry.file_type().is_dir() && 
                entry.path().extension().and_then(|s| s.to_str()) == Some("desktop")
            })
        {
            let file_path = entry.path();
            
            // Try cache first
            let app_result: Result<xdg::App, eyre::Report> = if let Ok(Some(cached_app)) = desktop_cache.get(file_path) {
                Ok(cached_app)
            } else {
                // Parse the file
                match fs::read_to_string(file_path) {
                    Ok(contents) => {
                        if !contents.contains("[Desktop Entry]") {
                            continue;
                        }
                        
                        match xdg::App::parse(&contents, None, cli.filter_desktop) {
                            Ok(mut app) => {
                                if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
                                    app.desktop_id = Some(file_name.to_string());
                                }
                                let _ = desktop_cache.set(file_path, app.clone());
                                Ok(app)
                            }
                            Err(_) => continue,
                        }
                    }
                    Err(_) => continue,
                }
            };
            
            if let Ok(app) = app_result {
                // Check if this is the app we're looking for
                if app.name == app_name {
                    // Apply filtering if needed
                    if cli.filter_desktop {
                        if let Ok(current_desktop) = env::var("XDG_CURRENT_DESKTOP") {
                            let desktops: Vec<String> = current_desktop.split(':').map(|s| s.to_string()).collect();
                            
                            if !app.not_show_in.is_empty() {
                                let should_hide = app.not_show_in.iter()
                                    .any(|d| desktops.iter().any(|cd| cd.eq_ignore_ascii_case(d)));
                                if should_hide {
                                    continue;
                                }
                            }
                            
                            if !app.only_show_in.is_empty() {
                                let should_show = app.only_show_in.iter()
                                    .any(|d| desktops.iter().any(|cd| cd.eq_ignore_ascii_case(d)));
                                if !should_show {
                                    continue;
                                }
                            }
                        }
                    }
                    
                    // Found it! Apply history and return
                    return Ok(Some(history_cache.apply_to_app(app)));
                }
            }
        }
    }
    
    Ok(None)
}

fn launch_program_directly(cli: &cli::Opts, program_name: &str) -> eyre::Result<()> {
    use nucleo_matcher::{Matcher, Config, Utf32Str};
    
    // Open database for history
    let (db, _data_dir) = helpers::open_history_db()?;
    
    let program_name_lower = program_name.to_lowercase();
    
    // FAST PATH: Check history for exact or prefix match
    // This avoids loading any desktop files for common cases
    let history_cache = cache::HistoryCache::load(&db)?;
    
    // First try exact match in history
    for (app_name, _count) in history_cache.history.iter() {
        if app_name.to_lowercase() == program_name_lower {
            // Found exact match in history - try to find and launch it quickly
            if let Some(app) = find_app_by_name_fast(&db, app_name, cli)? {
                if cli.no_exec {
                    println!("{}", app.command);
                    return Ok(());
                }
                return helpers::launch_app(&app, cli, &db);
            }
        }
    }
    
    // Try prefix match in history (e.g., "fire" -> "Firefox")
    if let Some((app_name, _)) = history_cache.get_best_match(program_name) {
        if let Some(app) = find_app_by_name_fast(&db, app_name, cli)? {
            if cli.no_exec {
                println!("{}", app.command);
                return Ok(());
            }
            return helpers::launch_app(&app, cli, &db);
        }
    }
    
    // SLOW PATH: No exact match in history, need to load all apps
    let mut dirs: Vec<path::PathBuf> = vec![];
    
    // Add user's data directory
    if let Some(xdg_data_home) = env::var("XDG_DATA_HOME").ok().filter(|s| !s.is_empty()) {
        let mut dir = path::PathBuf::from(xdg_data_home);
        dir.push("applications");
        if dir.exists() {
            dirs.push(dir);
        }
    } else if let Some(home_dir) = dirs::home_dir() {
        let mut dir = home_dir;
        dir.push(".local/share/applications");
        if dir.exists() {
            dirs.push(dir);
        }
    }
    
    // Add system data directories
    if let Ok(res) = env::var("XDG_DATA_DIRS") {
        for data_dir in res.split(':').filter(|s| !s.is_empty()) {
            let mut dir = path::PathBuf::from(data_dir);
            dir.push("applications");
            if dir.exists() {
                dirs.push(dir);
            }
        }
    } else {
        // Default paths for Linux and BSD
        let mut default_paths = vec![
            path::PathBuf::from("/usr/local/share"),
            path::PathBuf::from("/usr/share"),
        ];
        
        // Add BSD-specific paths
        #[cfg(target_os = "openbsd")]
        {
            default_paths.push(path::PathBuf::from("/usr/X11R6/share"));
        }
        
        for data_dir in &mut default_paths {
            data_dir.push("applications");
            if data_dir.exists() {
                dirs.push(data_dir.clone());
            }
        }
    }
    
    // Read applications with filtering options
    let apps_receiver = xdg::read_with_options(dirs, &db, cli.filter_desktop, cli.list_executables_in_path);
    
    // Collect all apps
    let mut all_apps = Vec::new();
    while let Ok(app) = apps_receiver.recv() {
        all_apps.push(app);
    }
    
    if all_apps.is_empty() {
        return Err(eyre!("No applications found"));
    }
    
    // Find the best match using improved matching logic for -p
    let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
    let mut best_app: Option<(xdg::App, i64)> = None;
    
    for app in all_apps {
        let app_name_lower = app.name.to_lowercase();
        
        // extract executable name from command
        let exec_name = helpers::extract_exec_name(&app.command);
        let exec_name_lower = exec_name.to_lowercase();
        
        // Prioritized matching: exact > prefix > fuzzy
        let mut final_score = if app_name_lower == program_name_lower {
            1_000_000 // Exact app name match
        } else if exec_name_lower == program_name_lower {
            900_000 // Exact executable name match
        } else if exec_name_lower.starts_with(&program_name_lower) {
            800_000 // Executable prefix match (e.g., "fo" matches "foot")
        } else if app_name_lower.starts_with(&program_name_lower) {
            700_000 // App name prefix match
        } else {
            // Fuzzy matching with priority for executable name (SIMD-accelerated)
            let name_score = matcher.fuzzy_match(Utf32Str::Ascii(app.name.as_bytes()), Utf32Str::Ascii(program_name.as_bytes())).unwrap_or(0) as i64;
            let exec_score = matcher.fuzzy_match(Utf32Str::Ascii(exec_name.as_bytes()), Utf32Str::Ascii(program_name.as_bytes())).unwrap_or(0) as i64;
            
            // Prioritize executable name matches (2x weight)
            let best_score = std::cmp::max(name_score, exec_score * 2);
            
            if best_score == 0 {
                continue; // No match at all
            }
            
            best_score
        };
        
        // apply pin boost (highest priority after exact matches)
        if app.pinned {
            if final_score < 700_000 {
                final_score += 500_000; // boost fuzzy matches significantly
            } else {
                final_score += 50_000; // boost exact matches slightly
            }
        }
        
        // include history in scoring (but don't let it dominate exact/prefix matches)
        if app.history > 0 {
            final_score = if final_score >= 700_000 {
                // for exact/prefix matches, history is just a tiebreaker
                final_score + app.history as i64
            } else {
                // for fuzzy matches, history multiplies the score
                final_score * app.history as i64
            };
        }
        
        if let Some((_, current_best_score)) = &best_app {
            if final_score > *current_best_score {
                best_app = Some((app, final_score));
            }
        } else {
            best_app = Some((app, final_score));
        }
    }
    
    let app_to_run = match best_app {
        Some((app, _)) => app,
        None => {
            return Err(eyre!("No matching application found for '{}'", program_name));
        }
    };
    
    // confirm first launch if enabled and app has no history
    if cli.confirm_first_launch && app_to_run.history == 0 {
        use std::io::{self, Write};
        eprint!("Launch {} [Y/n]? ", app_to_run.name);
        io::stderr().flush()?;
        
        let mut response = String::new();
        io::stdin().read_line(&mut response)?;
        let response = response.trim().to_lowercase();
        
        if response == "n" || response == "no" {
            // user said no, drop into TUI with search pre-filled
            // we need to return an error that signals to continue to TUI
            // but we can't easily do that from here, so just exit
            eprintln!("Cancelled. Use 'fsel -ss {}' to search in TUI.", program_name);
            std::process::exit(0);
        }
    }
    
    // print what we're launching if verbose
    if cli.verbose.unwrap_or(0) > 0 {
        eprintln!("Launching: {} ({})", app_to_run.name, app_to_run.command);
    }
    
    // handle --no-exec: print command and exit cleanly
    if cli.no_exec {
        println!("{}", app_to_run.command);
        return Ok(());
    }
    
    // launch the app
    helpers::launch_app(&app_to_run, cli, &db)?;
    
    Ok(())
}

fn real_main() -> eyre::Result<()> {
    let cli = cli::parse()?;
    
    // Handle dmenu mode
    if cli.dmenu_mode {
        return run_dmenu_mode(&cli);
    }
    
    // handle cclip mode
    if cli.cclip_mode {
        // check if cclip is available
        if !cclip::check_cclip_available() {
            eprintln!("error: cclip is not installed or not in PATH");
            eprintln!("install cclip from: https://github.com/heather7283/cclip");
            std::process::exit(1);
        }
        
        // check if cclipd is running and has data
        if let Err(e) = cclip::check_cclip_database() {
            eprintln!("error: {}", e);
            eprintln!("\nto use cclip mode, you need to:");
            eprintln!("1. start cclipd daemon:");
            eprintln!("   cclipd -s 2 -t \"image/png\" -t \"image/*\" -t \"text/plain;charset=utf-8\" -t \"text/*\" -t \"*\"");
            eprintln!("2. copy some stuff to build up history");
            eprintln!("\nfor more info: https://github.com/heather7283/cclip");
            std::process::exit(1);
        }
        
        // Handle lock file for cclip mode
        let lock_path = if let Some(project_dirs) = ProjectDirs::from("ch", "forkbomb9", env!("CARGO_PKG_NAME")) {
            let mut cache_dir = project_dirs.cache_dir().to_path_buf();
            if !cache_dir.exists() {
                fs::create_dir_all(&cache_dir)?;
            }
            cache_dir.push("fsel-cclip.lock");
            cache_dir
        } else {
            return Err(eyre!("can't find cache dir for {}", env!("CARGO_PKG_NAME")));
        };

        let contents = match fs::read_to_string(&lock_path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
            Ok(c) => c,
            Err(e) => {
                return Err(e).wrap_err("Failed to read cclip lockfile");
            }
        };

        if !contents.is_empty() {
            if cli.replace {
                let pid: i32 = contents
                    .parse()
                    .wrap_err("Failed to parse cclip lockfile contents")?;
                #[allow(unsafe_code)]
                unsafe {
                    libc::kill(pid, libc::SIGTERM);
                }
                fs::remove_file(&lock_path)?;
                std::thread::sleep(std::time::Duration::from_millis(200));
            } else {
                // cclip mode is already running
                return Err(eyre!("Fsel cclip mode is already running"));
            }
        }

        // Write current pid to lock file
        let mut lock_file = fs::File::create(&lock_path)?;
        let pid;
        // Safety: call to getpid is safe
        #[allow(unsafe_code)]
        unsafe {
            pid = libc::getpid();
        }
        use std::io::Write;
        lock_file.write_all(pid.to_string().as_bytes())?;

        // Lock file cleanup guard
        struct CclipLockGuard(path::PathBuf);
        impl Drop for CclipLockGuard {
            fn drop(&mut self) {
                let _ = fs::remove_file(&self.0);
            }
        }
        let _cclip_lock_guard = CclipLockGuard(lock_path);
        
        let rt = tokio::runtime::Runtime::new()?;
        return rt.block_on(run_cclip_mode(&cli));
    }
    
    // Handle direct launch mode (bypass TUI)
    // Require at least 2 characters, otherwise just launch TUI
    if let Some(ref program_name) = cli.program {
        if program_name.len() >= 2 {
            return launch_program_directly(&cli, program_name);
        }
        // Less than 2 characters, ignore and continue to TUI
    }
    
    setup_terminal(cli.disable_mouse)?;
    defer! {
        shutdown_terminal(cli.disable_mouse);
    }
    let db: std::sync::Arc<redb::Database>;
    let lock_path: path::PathBuf;

    // Open redb database
    if let Some(project_dirs) = ProjectDirs::from("ch", "forkbomb9", env!("CARGO_PKG_NAME")) {
        let data_dir = project_dirs.data_local_dir().to_path_buf();

        if !data_dir.exists() {
            // Create dir if it doesn't exist
            if let Err(error) = fs::create_dir_all(&data_dir) {
                return Err(eyre!("Failed to create data directory: {}", error));
            }
        }

        let hist_db_file = data_dir.join("hist_db.redb");

        // Check if Fsel is already running (mode-specific lock file) - BEFORE opening database
        {
            let mut lock = data_dir.clone();
            lock.push("fsel-fsel.lock");
            lock_path = lock;
            let contents = match fs::read_to_string(&lock_path) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
                Ok(c) => c,
                Err(e) => return Err(e).wrap_err("Failed to read lockfile"),
            };

            if !contents.is_empty() {
                if cli.replace {
                    let mut target_pids: BTreeSet<i32> = BTreeSet::new();

                    if let Ok(pid) = contents.parse::<i32>() {
                        target_pids.insert(pid);
                    }

                    if let Ok(holders) = find_processes_holding_file(&hist_db_file) {
                        target_pids.extend(holders);
                    }

                    for pid in target_pids.clone() {
                        #[allow(unsafe_code)]
                        unsafe {
                            let _ = libc::kill(pid, libc::SIGTERM);
                        }

                        const CHECK_INTERVAL_MS: u64 = 5;
                        const TOTAL_WAIT_MS: u64 = 30;
                        let mut waited_ms = 0u64;
                        let mut escalated = false;

                        loop {
                            #[allow(unsafe_code)]
                            let still_running = unsafe { libc::kill(pid, 0) == 0 };

                            if !still_running {
                                break;
                            }

                            if !escalated {
                                #[allow(unsafe_code)]
                                unsafe {
                                    let _ = libc::kill(pid, libc::SIGKILL);
                                }
                                escalated = true;
                            }

                            if waited_ms >= TOTAL_WAIT_MS {
                                return Err(eyre::eyre!(
                                    "Existing fsel instance (pid {pid}) refused to exit"
                                ));
                            }

                            std::thread::sleep(std::time::Duration::from_millis(CHECK_INTERVAL_MS));
                            waited_ms += CHECK_INTERVAL_MS;
                        }
                    }

                    if let Ok(mut remaining) = find_processes_holding_file(&hist_db_file) {
                        remaining.retain(|pid| !target_pids.contains(pid));

                        if !remaining.is_empty() {
                            return Err(eyre::eyre!(
                                "Existing fsel instance (pid(s) {:?}) refused to exit",
                                remaining
                            ));
                        }
                    }
                } else {
                    return Err(eyre!("Fsel is already running"));
                }
            } else if cli.replace {
                if let Ok(holders) = find_processes_holding_file(&hist_db_file) {
                    if !holders.is_empty() {
                        for pid in holders.clone() {
                            #[allow(unsafe_code)]
                            unsafe {
                                let _ = libc::kill(pid, libc::SIGTERM);
                            }

                            const CHECK_INTERVAL_MS: u64 = 5;
                            const TOTAL_WAIT_MS: u64 = 30;
                            let mut waited_ms = 0u64;
                            let mut escalated = false;

                            loop {
                                #[allow(unsafe_code)]
                                let still_running = unsafe { libc::kill(pid, 0) == 0 };

                                if !still_running {
                                    break;
                                }

                                if !escalated {
                                    #[allow(unsafe_code)]
                                    unsafe {
                                        let _ = libc::kill(pid, libc::SIGKILL);
                                    }
                                    escalated = true;
                                }

                                if waited_ms >= TOTAL_WAIT_MS {
                                    return Err(eyre::eyre!(
                                        "Existing fsel instance (pid {pid}) refused to exit"
                                    ));
                                }

                                std::thread::sleep(std::time::Duration::from_millis(CHECK_INTERVAL_MS));
                                waited_ms += CHECK_INTERVAL_MS;
                            }
                        }

                        if let Ok(final_holders) = find_processes_holding_file(&hist_db_file) {
                            if !final_holders.is_empty() {
                                return Err(eyre::eyre!(
                                    "Existing fsel instance (pid(s) {:?}) refused to exit",
                                    final_holders
                                ));
                            }
                        }
                    }
                }
            }

            if let Err(err) = fs::remove_file(&lock_path) {
                if err.kind() != io::ErrorKind::NotFound {
                    return Err(err).wrap_err("Failed to remove existing lockfile");
                }
            }

            let mut lock_file = fs::File::create(&lock_path)?;
            let pid;
            #[allow(unsafe_code)]
            unsafe {
                pid = libc::getpid();
            }
            use std::io::Write;
            lock_file.write_all(pid.to_string().as_bytes())?;
        }

        // Lock file cleanup guard
        struct LockGuard(path::PathBuf);
        impl Drop for LockGuard {
            fn drop(&mut self) {
                let _ = fs::remove_file(&self.0);
            }
        }
        let _lock_guard = LockGuard(lock_path.clone());

        let mut db_instance = redb::Database::create(&hist_db_file);
        if let Err(err) = &db_instance {
            if cli.replace && err.to_string().contains("Cannot acquire lock") {
                std::thread::sleep(std::time::Duration::from_millis(15));
                db_instance = redb::Database::create(&hist_db_file);
            }
        }

        let db_instance = db_instance
            .wrap_err_with(|| format!("Failed to open database at {:?}", hist_db_file))?;

        db = std::sync::Arc::new(db_instance);


        if cli.clear_history {
            // Clear all tables in redb
            const HISTORY_TABLE: redb::TableDefinition<&str, u64> = redb::TableDefinition::new("history");
            const PINNED_TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("pinned_apps");
            
            let write_txn = db.begin_write().wrap_err("Error starting transaction")?;
            {
                let mut history_table = write_txn.open_table(HISTORY_TABLE)?;
                let mut pinned_table = write_txn.open_table(PINNED_TABLE)?;
                
                // Collect keys first, then delete
                let history_keys: Vec<String> = history_table.iter()?.filter_map(|r| r.ok().map(|(k, _)| k.value().to_string())).collect();
                let pinned_keys: Vec<String> = pinned_table.iter()?.filter_map(|r| r.ok().map(|(k, _)| k.value().to_string())).collect();
                
                for key in history_keys {
                    history_table.remove(key.as_str())?;
                }
                for key in pinned_keys {
                    pinned_table.remove(key.as_str())?;
                }
            }
            write_txn.commit().wrap_err("Error clearing database")?;
            
            println!("Database cleared succesfully!");
            println!(
                "To fully remove the database, delete {}",
                project_dirs.data_local_dir().display()
            );
            // Lock file cleanup is handled by LockGuard when it goes out of scope
            return Ok(());
        }
        
        if cli.clear_cache {
            let cache = crate::cache::DesktopCache::new(db.clone())?;
            cache.clear().wrap_err("Error clearing cache")?;
            println!("Desktop file cache cleared successfully!");
            return Ok(());
        }
        
        if cli.refresh_cache {
            let cache = crate::cache::DesktopCache::new(db.clone())?;
            // Just clear the file list, parsed apps stay cached
            cache.clear_file_list().wrap_err("Error refreshing cache")?;
            println!("Desktop file list refreshed - will rescan on next launch!");
            return Ok(());
        }

    } else {
        return Err(eyre!(
            "can't find data dir for {}, is your system broken?",
            env!("CARGO_PKG_NAME")
        ));
    };

    // Directories to look for applications (XDG Base Directory Specification)
    let mut dirs: Vec<path::PathBuf> = vec![];
    
    // User data directory (XDG_DATA_HOME or ~/.local/share)
    if let Some(xdg_data_home) = env::var("XDG_DATA_HOME").ok().filter(|s| !s.is_empty()) {
        let mut dir = path::PathBuf::from(xdg_data_home);
        dir.push("applications");
        if dir.exists() {
            dirs.push(dir);
        }
    } else if let Some(home_dir) = dirs::home_dir() {
        let mut dir = home_dir;
        dir.push(".local/share/applications");
        if dir.exists() {
            dirs.push(dir);
        }
    }
    
    // System data directories (XDG_DATA_DIRS)
    if let Ok(res) = env::var("XDG_DATA_DIRS") {
        for data_dir in res.split(':').filter(|s| !s.is_empty()) {
            let mut dir = path::PathBuf::from(data_dir);
            dir.push("applications");
            if dir.exists() {
                dirs.push(dir);
            }
        }
    } else {
        // XDG specification fallback directories for Linux and BSD
        let mut default_paths = vec![
            path::PathBuf::from("/usr/local/share"),
            path::PathBuf::from("/usr/share"),
        ];
        
        // add BSD-specific paths
        #[cfg(target_os = "openbsd")]
        {
            default_paths.push(path::PathBuf::from("/usr/X11R6/share"));
        }
        
        for data_dir in &mut default_paths {
            data_dir.push("applications");
            if data_dir.exists() {
                dirs.push(data_dir.clone());
            }
        }
    }


    // Read applications with filtering options
    let apps = xdg::read_with_options(dirs, &db, cli.filter_desktop, cli.list_executables_in_path);

    // Initialize the terminal with crossterm backend using stderr
    let backend = CrosstermBackend::new(io::stderr());
    let mut terminal = Terminal::new(backend).wrap_err("Failed to start crossterm terminal")?;
    terminal.hide_cursor().wrap_err("Failed to hide cursor")?;
    terminal.clear().wrap_err("Failed to clear terminal")?;

    // Input handler with fast tick rate for instant app loading
    let input = input::Config {
        disable_mouse: cli.disable_mouse,
        tick_rate: std::time::Duration::from_millis(16), // ~60fps for instant updates
        ..input::Config::default()
    }.init();

    // PERFORMANCE FIX: Load ALL apps FIRST, then filter ONCE (eliminates "populating" effect)
    let mut all_apps = Vec::with_capacity(500);
    while let Ok(app) = apps.recv() {
        all_apps.push(app);
    }
    
    // Create UI with ALL apps loaded
    let mut ui = UI::new(all_apps);

    // Set user-defined verbosity level
    if let Some(level) = cli.verbose {
        ui.verbosity(level);
    }
    
    // Pre-fill search string if provided
    if let Some(ref search_str) = cli.search_string {
        ui.query = search_str.clone();
    }
    
    // Filter ONCE with all apps loaded - INSTANT display
    ui.filter(cli.match_mode);
    ui.info(cli.highlight_color, cli.fancy_mode);

    // App list
    let mut app_state = ListState::default();

    loop {

        // Draw UI
        terminal.draw(|f| {
            // Calculate layout based on configuration
            let total_height = f.area().height;
            let title_height = (total_height as f32 * cli.title_panel_height_percent as f32 / 100.0).round() as u16;
            let input_height = cli.input_panel_height;
            
            // Get title panel position (defaults to Top if not set)
            let title_panel_position = cli.title_panel_position.unwrap_or(crate::cli::PanelPosition::Top);
            
            // Split the window into three parts based on title panel position
            let (window, title_panel_index, apps_panel_index, input_panel_index) = match title_panel_position {
                crate::cli::PanelPosition::Top => {
                    // Top: title, apps, input (original layout)
                    let layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(title_height.max(3)),  // Title panel (min 3 lines)
                            Constraint::Min(1),                       // Apps panel (remaining space)
                            Constraint::Length(input_height),         // Input panel
                        ].as_ref())
                        .split(f.area());
                    (layout, 0, 1, 2)
                },
                crate::cli::PanelPosition::Middle => {
                    // Middle: apps, title, input
                    let layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Min(1),                       // Apps panel (remaining space)
                            Constraint::Length(title_height.max(3)),  // Title panel
                            Constraint::Length(input_height),         // Input panel
                        ].as_ref())
                        .split(f.area());
                    (layout, 1, 0, 2)
                },
                crate::cli::PanelPosition::Bottom => {
                    // Bottom: apps, input, title
                    let layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Min(1),                       // Apps panel (remaining space)
                            Constraint::Length(input_height),         // Input panel
                            Constraint::Length(title_height.max(3)),  // Title panel at bottom
                        ].as_ref())
                        .split(f.area());
                    (layout, 2, 0, 1)
                }
            };

            // Create blocks with configurable colors and borders
            let border_type = if cli.rounded_borders {
                BorderType::Rounded
            } else {
                BorderType::Plain
            };
            
            let create_main_block = |title: String| {
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        format!(" {} ", title), // Add spaces around title
                        Style::default().add_modifier(Modifier::BOLD).fg(cli.header_title_color),
                    ))
                    .border_type(border_type)
                    .border_style(Style::default().fg(cli.main_border_color))
            };
            
            let create_apps_block = |title: String| {
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        format!(" {} ", title), // Add spaces around title
                        Style::default().add_modifier(Modifier::BOLD).fg(cli.header_title_color),
                    ))
                    .border_type(border_type)
                    .border_style(Style::default().fg(cli.apps_border_color))
            };
            
            let create_input_block = |title: String| {
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        format!(" {} ", title), // Add spaces around title
                        Style::default().add_modifier(Modifier::BOLD).fg(cli.header_title_color),
                    ))
                    .border_type(border_type)
                    .border_style(Style::default().fg(cli.input_border_color))
            };

            // Determine panel titles based on fancy mode
            let (main_title, apps_title) = if cli.fancy_mode 
                && ui.selected.is_some() 
                && !ui.shown.is_empty() 
                && ui.selected.unwrap() < ui.shown.len() {
                let selected_app = &ui.shown[ui.selected.unwrap()];
                // In fancy mode: main panel shows app name, apps panel shows "Apps"
                (selected_app.name.clone(), "Apps".to_string())
            } else {
                // Normal mode: static titles
                ("Fsel".to_string(), "Apps".to_string())
            };
            
            // Description of the current app
            let description = Paragraph::new(ui.text.clone())
                .block(create_main_block(main_title))
                .style(Style::default().fg(cli.main_text_color))
                // Don't trim leading spaces when wrapping
                .wrap(Wrap { trim: false })
                .alignment(Alignment::Left);

            // Calculate apps panel height - account for borders (2 rows: top + bottom)
            let apps_panel_height = window[apps_panel_index].height;
            let max_visible = apps_panel_height.saturating_sub(2) as usize; // -2 for top/bottom borders
            
            // get the visible slice of apps based on scroll offset
            let visible_apps = ui.shown
                .iter()
                .skip(ui.scroll_offset)
                .take(max_visible)
                .map(|app| {
                    if app.pinned {
                        // add pin icon with color
                        let pin_span = Span::styled(
                            format!("{} ", cli.pin_icon),
                            Style::default().fg(cli.pin_color)
                        );
                        let name_span = Span::raw(&app.name);
                        ListItem::new(Line::from(vec![pin_span, name_span]))
                    } else {
                        ListItem::new(app.name.clone())
                    }
                })
                .collect::<Vec<ListItem>>();

            // App list (stateful widget) with borders
            let list = List::new(visible_apps)
                .block(create_apps_block(apps_title))
                .style(Style::default().fg(cli.apps_text_color))
                // Bold & colorized selection
                .highlight_style(
                    Style::default()
                        .fg(cli.highlight_color)
                        .add_modifier(Modifier::BOLD),
                )
                // Prefixed before the list item
                .highlight_symbol("> ");

            // Ensure we always have a valid selection when rendering
            if !ui.shown.is_empty() {
                match ui.selected {
                    None => {
                        // No selection at all, default to first visible item
                        ui.selected = Some(ui.scroll_offset.min(ui.shown.len() - 1));
                    }
                    Some(sel) if sel >= ui.shown.len() => {
                        // Selection is out of bounds, clamp to valid range
                        ui.selected = Some((ui.shown.len() - 1).min(sel));
                    }
                    _ => {
                        // Selection is valid, keep it
                    }
                }
            }
            
            // Update selection - adjust for scroll offset
            let visible_selection = ui.selected.and_then(|sel| {
                if sel >= ui.scroll_offset && sel < ui.scroll_offset + max_visible {
                    Some(sel - ui.scroll_offset)
                } else {
                    None
                }
            });
            app_state.select(visible_selection);

            // Query
            let query = Paragraph::new(Line::from(vec![
                // Format: (10/51) >> query
                Span::styled("(", Style::default().fg(cli.input_text_color)),
                Span::styled(
                    (ui.selected.map_or(0, |v| v + 1)).to_string(),
                    Style::default().fg(cli.highlight_color),
                ),
                Span::styled("/", Style::default().fg(cli.input_text_color)),
                Span::styled(ui.shown.len().to_string(), Style::default().fg(cli.input_text_color)),
                Span::styled(") ", Style::default().fg(cli.input_text_color)),
                Span::styled(">", Style::default().fg(cli.highlight_color)),
                Span::styled("> ", Style::default().fg(cli.input_text_color)),
                Span::styled(&ui.query, Style::default().fg(cli.input_text_color)),
                Span::styled(&cli.cursor, Style::default().fg(cli.highlight_color)),
            ]))
            .block(create_input_block("Input".to_string()))
            .style(Style::default().fg(cli.input_text_color))
            .alignment(Alignment::Left)
            .wrap(ratatui::widgets::Wrap { trim: false });

            // Render panels in their dynamic positions
            f.render_widget(description, window[title_panel_index]);
            // Only render app list if not hide_before_typing or query is not empty
            if !cli.hide_before_typing || !ui.query.is_empty() {
                f.render_stateful_widget(list, window[apps_panel_index], &mut app_state);
            }
            f.render_widget(query, window[input_panel_index]);
        })?;

        // Handle user input
        match input.next()? {
            Event::Input(key) => {
            use crossterm::event::KeyCode;
            
            // check keybinds
            if cli.keybinds.matches_exit(key.code, key.modifiers) {
                ui.selected = None;
                break;
            } else if cli.keybinds.matches_select(key.code, key.modifiers) {
                break;
            } else if cli.keybinds.matches_pin(key.code, key.modifiers) {
                if let Some(selected) = ui.selected {
                    if selected < ui.shown.len() {
                        let app_name = ui.shown[selected].name.clone();
                        if let Ok(is_pinned) = helpers::toggle_pin(&db, &app_name) {
                            // Update all apps with same name (handles duplicates like 2x Alacritty)
                            for app in &mut ui.shown {
                                if app.name == app_name {
                                    app.pinned = is_pinned;
                                }
                            }
                            ui.filter(cli.match_mode);
                            // Cursor stays put, app moves to top
                        }
                    }
                }
            } else if cli.keybinds.matches_backspace(key.code, key.modifiers) {
                ui.query.pop();
                ui.filter(cli.match_mode);
            } else if cli.keybinds.matches_left(key.code, key.modifiers) {
                if !ui.shown.is_empty() {
                    ui.selected = Some(0);
                }
            } else if cli.keybinds.matches_right(key.code, key.modifiers) {
                if !ui.shown.is_empty() {
                    ui.selected = Some(ui.shown.len() - 1);
                }
            } else if cli.keybinds.matches_down(key.code, key.modifiers) {
                    if let Some(selected) = ui.selected {
                        ui.selected = if selected < ui.shown.len() - 1 {
                            Some(selected + 1)
                        } else if !cli.hard_stop {
                            Some(0)
                        } else {
                            Some(selected)
                        };
                        
                        // Auto-scroll to keep selection visible
                        if let Some(new_selected) = ui.selected {
                            let total_height = terminal.size()?.height;
                            let title_height = (total_height as f32 * cli.title_panel_height_percent as f32 / 100.0).round() as u16;
                            let input_height = cli.input_panel_height;
                            let apps_panel_height = total_height - title_height - input_height;
                            let max_visible = apps_panel_height.saturating_sub(2) as usize; // -2 for borders
                            
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
            } else if cli.keybinds.matches_up(key.code, key.modifiers) {
                if let Some(selected) = ui.selected {
                        ui.selected = if selected > 0 {
                            Some(selected - 1)
                        } else if !cli.hard_stop {
                            Some(ui.shown.len() - 1)
                        } else {
                            Some(selected)
                        };
                        
                        // Auto-scroll to keep selection visible
                        if let Some(new_selected) = ui.selected {
                            let total_height = terminal.size()?.height;
                            let title_height = (total_height as f32 * cli.title_panel_height_percent as f32 / 100.0).round() as u16;
                            let input_height = cli.input_panel_height;
                            let apps_panel_height = total_height - title_height - input_height;
                            let max_visible = apps_panel_height.saturating_sub(2) as usize; // -2 for borders
                            
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
            } else {
                // regular character input
                match (key.code, key.modifiers) {
                    (KeyCode::Char(c), crossterm::event::KeyModifiers::NONE) | 
                    (KeyCode::Char(c), crossterm::event::KeyModifiers::SHIFT) => {
                        ui.query.push(c);
                        ui.filter(cli.match_mode);
                    }
                    _ => {}
                }
            }

            ui.info(cli.highlight_color, cli.fancy_mode);
            }
            Event::Mouse(mouse_event) => {
                let mouse_row = mouse_event.row;
                
                // Calculate panel positions based on title_panel_position
                let total_height = terminal.size()?.height;
                let title_height = (total_height as f32 * cli.title_panel_height_percent as f32 / 100.0).round() as u16;
                let input_height = cli.input_panel_height;
                let title_panel_position = cli.title_panel_position.unwrap_or(crate::cli::PanelPosition::Top);
                
                // Calculate apps panel coordinates based on layout
                let (apps_panel_start, apps_panel_height) = match title_panel_position {
                    crate::cli::PanelPosition::Top => {
                        // Top: title, apps, input - apps start after title
                        (title_height, total_height - title_height - input_height)
                    },
                    crate::cli::PanelPosition::Middle => {
                        // Middle: apps, title, input - apps start at top
                        (0, total_height - title_height - input_height)
                    },
                    crate::cli::PanelPosition::Bottom => {
                        // Bottom: apps, input, title - apps start at top
                        (0, total_height - title_height - input_height)
                    }
                };
                
                // List content area (inside the borders) - first item starts 1 row down from panel start
                let list_content_start = apps_panel_start + 1; // +1 for top border
                let max_visible_rows = apps_panel_height.saturating_sub(2); // -2 for top/bottom borders
                let list_content_end = list_content_start + max_visible_rows;
                
                let update_selection_for_mouse_pos = |ui: &mut UI, mouse_row: u16| {
                    if !ui.shown.is_empty() && mouse_row >= list_content_start && mouse_row < list_content_end {
                        let row_in_content = mouse_row - list_content_start;
                        let hovered_app_index = ui.scroll_offset + row_in_content as usize;
                        if hovered_app_index < ui.shown.len() {
                            ui.selected = Some(hovered_app_index);
                            ui.info(cli.highlight_color, cli.fancy_mode);
                        }
                    }
                };
                
                match mouse_event.kind {
                    // Handle mouse movement for hover highlighting
                    MouseEventKind::Moved => {
                        update_selection_for_mouse_pos(&mut ui, mouse_row);
                    }
                    // Handle left mouse button clicks to launch
                    MouseEventKind::Down(MouseButton::Left) => {
                        // Check if click is within the list content area
                        if mouse_row >= list_content_start && mouse_row < list_content_end && !ui.shown.is_empty() {
                            let row_in_content = mouse_row - list_content_start;
                            let clicked_app_index = ui.scroll_offset + row_in_content as usize;
                            
                            if clicked_app_index < ui.shown.len() {
                                ui.selected = Some(clicked_app_index);
                                ui.info(cli.highlight_color, cli.fancy_mode);
                                break; // Launch the clicked app
                            }
                        }
                    }
                    // Handle scroll wheel only when mouse is over the apps list
                    MouseEventKind::ScrollUp => {
                        if mouse_row >= list_content_start && mouse_row < list_content_end {
                            if !ui.shown.is_empty() && ui.scroll_offset > 0 {
                                ui.scroll_offset -= 1;
                                update_selection_for_mouse_pos(&mut ui, mouse_row);
                            }
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        if mouse_row >= list_content_start && mouse_row < list_content_end {
                            if !ui.shown.is_empty() {
                                let max_visible = max_visible_rows as usize;
                                if ui.scroll_offset + max_visible < ui.shown.len() {
                                    ui.scroll_offset += 1;
                                    update_selection_for_mouse_pos(&mut ui, mouse_row);
                                }
                            }
                        }
                    }
                    _ => {} // Ignore other mouse events
                }
            }
            Event::Tick => {}
        }
    }

    // Clean terminal exit (defer handles the rest)
    terminal.show_cursor().wrap_err("Failed to show cursor")?;

    if let Some(selected) = ui.selected {
        let app_to_run = &ui.shown[selected];
        
        // Handle --no-exec: print command and exit cleanly
        if cli.no_exec {
            println!("{}", app_to_run.command);
            return Ok(());
        }

        // launch the app
        helpers::launch_app(&app_to_run, &cli, &db)?;
    }

    // Lock file cleanup is handled by LockGuard
    Ok(())
}

