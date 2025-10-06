use std::io::{self, Write};
use std::sync::Mutex;
use crossterm::{
    cursor::{MoveTo, RestorePosition, SavePosition},
    queue,
};
use ratatui::layout::Rect;

/// Track the currently displayed image area for proper cleanup
static CURRENT_IMAGE_AREA: Mutex<Option<(Rect, String)>> = Mutex::new(None);

/// Track whether the current content is an image (to know when to clear)
static CURRENT_CONTENT_IS_IMAGE: Mutex<bool> = Mutex::new(false);

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
        // For Sixel, just clear a minimal area to avoid text corruption
        let spaces = " ".repeat((area.width / 2) as usize); // Clear less area
        
        Self::write_at_position((area.x, area.y), |stderr| {
            // Only clear a few lines to remove sixel graphics
            for y in area.top()..(area.top() + 3).min(area.bottom()) {
                queue!(stderr, MoveTo(area.x, y))?;
                write!(stderr, "{}", spaces)?;
            }
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
        } else if term.contains("foot") || term.contains("alacritty") || term.contains("xterm") {
            Self::Sixel
        } else {
            Self::None
        }
    }
    
    
    /// Display cclip image data at the given area
    pub async fn show_cclip_image(&self, rowid: &str, area: Rect) -> io::Result<()> {
        // Only clear if we're showing a different image
        // This prevents clearing the content area when switching to text items
        if let Ok(current) = CURRENT_IMAGE_AREA.lock() {
            if let Some((_, current_rowid)) = &*current {
                if current_rowid != rowid {
                    // Different image - clear the old one
                    drop(current);
                    self.clear_current_image()?;
                }
            }
        }
        
        // Show the new image
        let result = match self {
            Self::Kitty => self.show_cclip_kitty_image(rowid, area).await,
            Self::Sixel => self.show_cclip_sixel_image(rowid, area).await,
            Self::None => Ok(()), // No graphics support
        };
        
        // Track the current image for future clearing
        if result.is_ok() && *self != Self::None {
            if let Ok(mut current) = CURRENT_IMAGE_AREA.lock() {
                *current = Some((area, rowid.to_string()));
            }
        }
        
        result
    }
    
    /// Display cclip image only if it's different from the currently displayed one
    /// This prevents flickering from redrawing the same image
    pub async fn show_cclip_image_if_different(&self, rowid: &str, area: Rect) -> io::Result<()> {
        // Mark that we're showing image content
        if let Ok(mut is_image) = CURRENT_CONTENT_IS_IMAGE.lock() {
            *is_image = true;
        }
        
        // Check if we're already displaying this exact image at this area
        if let Ok(current) = CURRENT_IMAGE_AREA.lock() {
            if let Some((current_area, current_rowid)) = &*current {
                if current_rowid == rowid && *current_area == area {
                    // Same image in same area - no need to redraw
                    return Ok(());
                }
            }
        }
        
        // Different image or area - show it
        self.show_cclip_image(rowid, area).await
    }
    
    /// Handle transition to non-image content
    /// This clears any displayed image when switching to text content
    pub fn handle_non_image_content(&self) -> io::Result<()> {
        // Only clear if we have an actual image displayed AND we're not in None mode
        if let Ok(mut is_image) = CURRENT_CONTENT_IS_IMAGE.lock() {
            if *is_image && *self != Self::None {
                // Clear the image area before showing text
                self.clear_current_image()?;
                *is_image = false;
            }
        }
        Ok(())
    }
    
    /// Clear the currently displayed image
    pub fn clear_current_image(&self) -> io::Result<()> {
        if let Ok(mut current) = CURRENT_IMAGE_AREA.lock() {
            if let Some((area, _)) = current.take() {
                match self {
                    Self::Kitty => self.clear_kitty_graphics(area)?,
                    Self::Sixel | Self::None => TerminalGraphics::clear_area(area)?,
                }
            }
        }
        Ok(())
    }
    
    /// Clear Kitty graphics using the graphics protocol
    fn clear_kitty_graphics(&self, area: Rect) -> io::Result<()> {
        // Use Kitty's graphics protocol to delete images only, don't clear text
        TerminalGraphics::write_at_position((area.x, area.y), |stderr| {
            // Delete all graphics using Kitty protocol
            write!(stderr, "\x1b_Ga=d,d=A\x1b\\")?;  // Delete all images
            Ok(())
        })
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
                Ok(())
            })?;
        }
        
        Ok(())
    }
}
