//! # Fsel
//!
//! > _Blazing fast_ TUI launcher for GNU/Linux and *BSD
//!
//! For more info, check the [README](https://github.com/Mjoyufull/fsel)

mod app;
mod cli;
mod common;
mod config;
mod core;
mod desktop;
mod modes;
mod process;
mod strings;
mod ui;

/// Runs the application entrypoint and dispatches to the selected mode.
pub fn run() -> eyre::Result<std::process::ExitCode> {
    app::run()
}

/// Performs best-effort terminal cleanup after a fatal top-level error.
pub fn cleanup_after_error() {
    app::cleanup_after_error();
}
