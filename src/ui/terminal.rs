use crossterm::{
    ExecutableCommand,
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use eyre::WrapErr;
use std::io;

pub(crate) fn setup_terminal(disable_mouse: bool) -> eyre::Result<()> {
    enable_raw_mode().wrap_err("Failed to enable raw mode")?;
    io::stderr()
        .execute(EnterAlternateScreen)
        .wrap_err("Failed to enter alternate screen")?;
    if !disable_mouse {
        io::stderr()
            .execute(EnableMouseCapture)
            .wrap_err("Failed to enable mouse capture")?;
    }
    Ok(())
}

pub(crate) fn shutdown_terminal(disable_mouse: bool) -> eyre::Result<()> {
    if !disable_mouse {
        io::stderr().execute(DisableMouseCapture)?;
    }
    io::stderr().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
