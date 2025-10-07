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
    /// Text content is displayed  
    Text,
    /// Image content is displayed with area and rowid
    Image(Rect, String),
}

/// Single atomic state tracker to eliminate lock contention
static DISPLAY_STATE: Mutex<DisplayState> = Mutex::new(DisplayState::Empty);

/// Direct terminal graphics handler inspired by Yazi
pub struct TerminalGraphics;

impl TerminalGraphics {
    /// Write graphics data directly to terminal at specific coordinates
    /// This bypasses ratatui completely to avoid corruption of graphics escape sequences
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
        
        // Restore cursor position (cursor remains hidden - ratatui will show it)
        queue!(stderr, RestorePosition)?;
        
        // Ensure all commands are flushed
        stderr.flush()?;
        
        result
    }
    
    /// Clear an area by overwriting with spaces (for image cleanup)
    pub fn clear_area(area: Rect) -> io::Result<()> {
        // Aggressively clear the ENTIRE content area and force refresh
        Self::write_at_position((area.x, area.y), |stderr| {
            // Clear the entire area
            let spaces = " ".repeat(area.width as usize);
            for y in area.top()..area.bottom() {
                queue!(stderr, MoveTo(area.x, y))?;
                write!(stderr, "{}", spaces)?;
            }
            // Force terminal to refresh
            stderr.flush()?;
            Ok(())
        })
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
        // Check current state and only clear if showing different image
        let should_clear = if let Ok(current_state) = DISPLAY_STATE.lock() {
            match &*current_state {
                DisplayState::Image(current_area, current_rowid) => {
                    if current_rowid != rowid {
                        Some(*current_area)
                    } else {
                        None
                    }
                },
                _ => None,
            }
        } else {
            None
        };
        
        if let Some(area_to_clear) = should_clear {
            // Different image - clear the old one with minimal clearing
            self.clear_image_minimal(area_to_clear)?;
        }
        
        // Show the new image
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
        if let Ok(current_state) = DISPLAY_STATE.lock() {
            if let DisplayState::Image(current_area, current_rowid) = &*current_state {
                if current_rowid == rowid && *current_area == area {
                    // Same image in same area - no need to redraw
                    return Ok(());
                }
            }
        }
        
        // Yazi approach: hide any current image before showing new one
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
    
    /// Erase image from specific area (Yazi's approach)
    pub fn image_erase(&self, area: Rect) -> io::Result<()> {
        match self {
            Self::Kitty => self.clear_kitty_graphics(area)?,
            Self::Sixel | Self::None => {
                // For Sixel: minimal clearing - just clear a few lines to break graphics sequences
                // Let ratatui text naturally overwrite the rest to avoid timing issues
                TerminalGraphics::write_at_position((area.x, area.y), |stderr| {
                    let spaces = " ".repeat(area.width as usize);
                    // Only clear first 3 lines to break Sixel sequences, not entire area
                    for y in area.top()..(area.top() + 3).min(area.bottom()) {
                        queue!(stderr, MoveTo(area.x, y))?;
                        write!(stderr, "{}", spaces)?;
                    }
                    // Don't flush immediately - let it happen naturally
                    Ok(())
                })?;
            },
        }
        Ok(())
    }
    
    /// Clear the currently displayed image
    pub fn clear_current_image(&self) -> io::Result<()> {
        self.image_hide()
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
    
    /// Clear Sixel graphics more effectively
    fn clear_sixel_graphics(&self, area: Rect) -> io::Result<()> {
        // Use the same full area clearing as the general clear_area method
        TerminalGraphics::clear_area(area)
    }
    
    /// Minimal image clearing that doesn't destroy text content
    fn clear_image_minimal(&self, area: Rect) -> io::Result<()> {
        match self {
            Self::Kitty => self.clear_kitty_graphics(area)?,
            Self::Sixel | Self::None => {
                // For sixel, only clear a small area at the top to remove graphics headers
                TerminalGraphics::write_at_position((area.x, area.y), |stderr| {
                    let spaces = " ".repeat(area.width.min(20) as usize);
                    for y in area.top()..(area.top() + 1).min(area.bottom()) {
                        queue!(stderr, MoveTo(area.x, y))?;
                        write!(stderr, "{}", spaces)?;
                    }
                    Ok(())
                })?;
            },
        }
        Ok(())
    }
    
    
    async fn show_cclip_kitty_image(&self, rowid: &str, area: Rect) -> io::Result<()> {
        // Pipe cclip data directly to chafa for kitty graphics with proper positioning
        let result = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&format!(
                "cclip get {} | chafa -f kitty --size {}x{} --animate=off --polite=on -",
                rowid, area.width, area.height
            ))
            .output()
            .await?;
            
        if result.status.success() {
            let graphics_data = result.stdout;
            
            TerminalGraphics::write_at_position((area.x, area.y), |stderr| {
                // Position cursor at the target location
                write!(stderr, "\x1b[{};{}H", area.y + 1, area.x + 1)?;
                stderr.write_all(&graphics_data)?;
                stderr.flush()?; // Force immediate display
                Ok(())
            })?;
        }
        
        Ok(())
    }
    
    async fn show_cclip_sixel_image(&self, rowid: &str, area: Rect) -> io::Result<()> {
        // Pipe cclip data directly to chafa for sixel graphics
        let result = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&format!(
                "cclip get {} | chafa -f sixels --size {}x{} --animate=off --polite=on -",
                rowid, area.width, area.height
            ))
            .output()
            .await?;
            
        if result.status.success() {
            let graphics_data = result.stdout;
            
            TerminalGraphics::write_at_position((area.x, area.y), |stderr| {
                // Position cursor at the target location
                write!(stderr, "\x1b[{};{}H", area.y + 1, area.x + 1)?;
                stderr.write_all(&graphics_data)?;
                stderr.flush()?; // Force immediate display
                Ok(())
            })?;
        }
        
        Ok(())
    }
}
