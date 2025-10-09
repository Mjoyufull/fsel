use std::io::{self, Write};
use std::sync::Mutex;
use crossterm::{
    cursor::{MoveTo, RestorePosition, SavePosition},
    queue,
};
use ratatui::layout::Rect;

/// Combined display state to eliminate race conditions
#[derive(Debug, Clone, PartialEq)]
enum DisplayState {
    /// No content displayed
    Empty,
    /// Image content is displayed with area and rowid
    Image(Rect, String),
}

/// Single atomic state tracker to eliminate lock contention
static DISPLAY_STATE: Mutex<DisplayState> = Mutex::new(DisplayState::Empty);

/// Direct terminal graphics handler inspired by Yazi
pub struct TerminalGraphics;

impl TerminalGraphics {
    /// Write graphics data directly to terminal (bypasses ratatui)
    pub fn write_at_position<F, T>(pos: (u16, u16), writer_fn: F) -> io::Result<T>
    where
        F: FnOnce(&mut io::Stderr) -> io::Result<T>,
    {
        use crossterm::cursor::Hide;
        let mut stderr = io::stderr();
        
        // Hide cursor to prevent jumping and flickering
        queue!(stderr, Hide, SavePosition)?;
        
        // Move to target position
        queue!(stderr, MoveTo(pos.0, pos.1))?;
        
        // Execute the writer function with direct terminal access
        let result = writer_fn(&mut stderr);
        
        // Restore cursor position
        queue!(stderr, RestorePosition)?;
        
        // Ensure all commands are flushed
        stderr.flush()?;
        
        result
    }
}

/// Image display adapter - chooses the right graphics protocol
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GraphicsAdapter {
    Kitty,
    Sixel,
    None,
}

impl GraphicsAdapter {
    /// Detect the best graphics adapter for the current terminal
    pub fn detect() -> Self {
        let term = std::env::var("TERM").unwrap_or_default();
        let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
        
        if term_program == "kitty" || term.contains("kitty") {
            Self::Kitty
        } else if term.starts_with("foot") || term.contains("xterm") || term_program == "WezTerm" {
            Self::Sixel
        } else {
            Self::None
        }
    }
    
    
    /// Display cclip image data at the given area
    pub async fn show_cclip_image(&self, rowid: &str, area: Rect) -> io::Result<()> {
        // Show the new image (will overwrite any existing content)
        let result = match self {
            Self::Kitty => self.show_cclip_kitty_image(rowid, area).await,
            Self::Sixel => self.show_cclip_sixel_image(rowid, area).await,
            Self::None => Ok(()), // No graphics support
        };
        
        // Update state to track the new image
        if result.is_ok() && *self != Self::None {
            if let Ok(mut state) = DISPLAY_STATE.lock() {
                *state = DisplayState::Image(area, rowid.to_string());
            }
        }
        
        result
    }
    
    /// Display cclip image only if it's different from the currently displayed one
    /// Uses Yazi's approach: always hide current image before showing new one
    pub async fn show_cclip_image_if_different(&self, rowid: &str, area: Rect) -> io::Result<()> {
        // Check if we're already displaying this exact image at this area
        let needs_update = if let Ok(current_state) = DISPLAY_STATE.lock() {
            match &*current_state {
                DisplayState::Image(current_area, current_rowid) => {
                    // Need update if different image or different area
                    current_rowid != rowid || *current_area != area
                },
                _ => true, // Not showing image, need to show
            }
        } else {
            true
        };
        
        if !needs_update {
            // Same image at same position, skip redraw
            return Ok(());
        }
        
        // ALWAYS hide/clear any current image before showing new one to prevent stacking
        self.image_hide()?;
        
        // Show the new image
        self.show_cclip_image(rowid, area).await
    }

    
    
    /// Hide any currently displayed image (Yazi's approach)
    pub fn image_hide(&self) -> io::Result<()> {
        if let Ok(mut state) = DISPLAY_STATE.lock() {
            if let DisplayState::Image(area, _) = &*state {
                self.image_erase(*area)?;
                *state = DisplayState::Empty;
            }
        }
        Ok(())
    }
    
    /// Erase image from specific area
    pub fn image_erase(&self, area: Rect) -> io::Result<()> {
        match self {
            Self::Kitty => self.clear_kitty_graphics(area)?,
            Self::Sixel | Self::None => {
                // For Sixel: clear the entire area to prevent stacking
                TerminalGraphics::write_at_position((area.x, area.y), |stderr| {
                    let spaces = " ".repeat(area.width as usize);
                    // Clear entire area to remove Sixel data
                    for y in area.top()..area.bottom() {
                        queue!(stderr, MoveTo(area.x, y))?;
                        write!(stderr, "{}", spaces)?;
                    }
                    stderr.flush()?;
                    Ok(())
                })?;
            },
        }
        Ok(())
    }
    
    /// Clear Kitty graphics using the graphics protocol
    fn clear_kitty_graphics(&self, area: Rect) -> io::Result<()> {
        // Use Kitty's graphics protocol to delete images only, don't clear text
        TerminalGraphics::write_at_position((area.x, area.y), |stderr| {
            // Delete all graphics using Kitty protocol
            write!(stderr, "\x1b_Ga=d,d=A\x1b\\")?;  // Delete all images
            stderr.flush()?; // Force immediate execution
            Ok(())
        })
    }
    
    async fn show_cclip_kitty_image(&self, rowid: &str, area: Rect) -> io::Result<()> {
        // pipe cclip data directly to chafa for kitty graphics with proper positioning
        let mut cclip_child = tokio::process::Command::new("cclip")
            .args(&["get", rowid])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        
        if let Some(cclip_stdout) = cclip_child.stdout.take() {
            let size_arg = format!("{}x{}", area.width, area.height);
            let mut chafa_child = tokio::process::Command::new("chafa")
                .args(&["-f", "kitty", "--size", &size_arg, "--animate=off", "--polite=on", "-"])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .spawn()?;
            
            if let Some(mut chafa_stdin) = chafa_child.stdin.take() {
                // pipe cclip output to chafa input
                tokio::spawn(async move {
                    let mut cclip_stdout = cclip_stdout;
                    tokio::io::copy(&mut cclip_stdout, &mut chafa_stdin).await.ok();
                });
                
                let result = chafa_child.wait_with_output().await?;
                let _ = cclip_child.wait().await;
                
                if result.status.success() {
                    let graphics_data = result.stdout;
                    
                    TerminalGraphics::write_at_position((area.x, area.y), |stderr| {
                        // position cursor at the target location
                        write!(stderr, "\x1b[{};{}H", area.y + 1, area.x + 1)?;
                        stderr.write_all(&graphics_data)?;
                        stderr.flush()?;
                        Ok(())
                    })?;
                }
            }
        }
        
        Ok(())
    }
    
    async fn show_cclip_sixel_image(&self, rowid: &str, area: Rect) -> io::Result<()> {
        // pipe cclip data directly to chafa for sixel graphics
        let mut cclip_child = tokio::process::Command::new("cclip")
            .args(&["get", rowid])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        
        if let Some(cclip_stdout) = cclip_child.stdout.take() {
            let size_arg = format!("{}x{}", area.width, area.height);
            let mut chafa_child = tokio::process::Command::new("chafa")
                .args(&["-f", "sixels", "--size", &size_arg, "--animate=off", "--polite=on", "-"])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .spawn()?;
            
            if let Some(mut chafa_stdin) = chafa_child.stdin.take() {
                // pipe cclip output to chafa input
                tokio::spawn(async move {
                    let mut cclip_stdout = cclip_stdout;
                    tokio::io::copy(&mut cclip_stdout, &mut chafa_stdin).await.ok();
                });
                
                let result = chafa_child.wait_with_output().await?;
                let _ = cclip_child.wait().await;
                
                if result.status.success() {
                    let graphics_data = result.stdout;
                    
                    TerminalGraphics::write_at_position((area.x, area.y), |stderr| {
                        // position cursor at the target location
                        write!(stderr, "\x1b[{};{}H", area.y + 1, area.x + 1)?;
                        stderr.write_all(&graphics_data)?;
                        stderr.flush()?;
                        Ok(())
                    })?;
                }
            }
        }
        
        Ok(())
    }
}
