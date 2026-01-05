//! Async input handling using crossterm's EventStream
//!
//! This module provides async event handling for the TUI, using tokio and crossterm's EventStream.
//! AsyncInput is infrastructure for future async migration.

#![allow(dead_code)]

use crossterm::event::{Event as CrosstermEvent, EventStream, KeyCode, KeyEvent, MouseEvent};
use futures::StreamExt;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;

/// Builder for `AsyncInput`
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub exit_key: KeyCode,
    pub tick_rate: Duration,
    pub disable_mouse: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            exit_key: KeyCode::Esc,
            tick_rate: Duration::from_millis(250),
            disable_mouse: false,
        }
    }
}

impl Config {
    /// Creates a new sync `Input` with the configuration in `Self`
    /// blocking input handler for simple modes
    pub fn init(self) -> Input {
        Input::with_config(self)
    }

    /// Creates a new async `AsyncInput` with the configuration in `Self`
    /// Used by async modes (app_launcher when migrated)
    pub fn init_async(self) -> AsyncInput {
        AsyncInput::with_config(self)
    }
}

#[derive(Debug)]
pub enum Event<I> {
    Input(I),
    Mouse(MouseEvent),
    Tick,
    Render,
}

/// Async input handler using crossterm's EventStream
pub struct AsyncInput {
    rx: mpsc::UnboundedReceiver<Event<KeyEvent>>,
    _task: tokio::task::JoinHandle<()>,
}

impl AsyncInput {
    pub fn with_config(config: Config) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        let _task = tokio::spawn(async move {
            let mut reader = EventStream::new();
            let mut tick_interval = interval(config.tick_rate);
            let mut render_interval = interval(Duration::from_millis(16)); // ~60 FPS

            loop {
                tokio::select! {
                    // Handle terminal events
                    maybe_event = reader.next() => {
                        match maybe_event {
                            Some(Ok(event)) => {
                                match event {
                                    CrosstermEvent::Key(key) => {
                                        // Filter for KeyPress only (avoid duplicate events on some platforms)
                                        if key.kind == crossterm::event::KeyEventKind::Press {
                                            if tx.send(Event::Input(key)).is_err() {
                                                return;
                                            }
                                            if key.code == config.exit_key {
                                                return;
                                            }
                                        }
                                    }
                                    CrosstermEvent::Mouse(mouse) => {
                                        if !config.disable_mouse && tx.send(Event::Mouse(mouse)).is_err() {
                                            return;
                                        }
                                    }
                                    CrosstermEvent::Resize(_, _) => {
                                        // Trigger a render on resize
                                        let _ = tx.send(Event::Render);
                                    }
                                    _ => {}
                                }
                            }
                            Some(Err(_)) => {
                                // Event read error, exit
                                return;
                            }
                            None => {
                                // Stream ended
                                return;
                            }
                        }
                    }
                    // Tick events for periodic updates
                    _ = tick_interval.tick() => {
                        if tx.send(Event::Tick).is_err() {
                            return;
                        }
                    }
                    // Render events for frame rate control
                    _ = render_interval.tick() => {
                        if tx.send(Event::Render).is_err() {
                            return;
                        }
                    }
                }
            }
        });

        Self { rx, _task }
    }

    /// Next event (async)
    pub async fn next(&mut self) -> Option<Event<KeyEvent>> {
        self.rx.recv().await
    }
}

// =============================================================================
// LEGACY SYNC INPUT (kept for backwards compatibility with dmenu/cclip modes)
// =============================================================================

use std::sync::mpsc as std_mpsc;
use std::thread;

/// Legacy sync input handler (for modes not yet migrated to async)
pub struct Input {
    rx: std_mpsc::Receiver<Event<KeyEvent>>,
    _input_handle: thread::JoinHandle<()>,
    _tick_handle: thread::JoinHandle<()>,
}

impl Input {
    pub fn with_config(config: Config) -> Self {
        let (tx, rx) = std_mpsc::channel();

        let _input_handle = {
            let tx = tx.clone();

            thread::spawn(move || loop {
                if let Ok(true) = crossterm::event::poll(Duration::from_millis(100)) {
                    if let Ok(event) = crossterm::event::read() {
                        match event {
                            CrosstermEvent::Key(key) => {
                                if tx.send(Event::Input(key)).is_err() {
                                    return;
                                }
                                if key.code == config.exit_key {
                                    return;
                                }
                            }
                            CrosstermEvent::Mouse(mouse) => {
                                if !config.disable_mouse && tx.send(Event::Mouse(mouse)).is_err() {
                                    return;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            })
        };

        let _tick_handle = {
            thread::spawn(move || loop {
                if tx.send(Event::Tick).is_err() {
                    break;
                }
                thread::sleep(config.tick_rate);
            })
        };

        Self {
            rx,
            _input_handle,
            _tick_handle,
        }
    }

    /// Next key pressed by user.
    pub fn next(&self) -> Result<Event<KeyEvent>, std_mpsc::RecvError> {
        self.rx.recv()
    }

    /// Next key pressed by user with timeout.
    #[allow(dead_code)]
    pub fn next_timeout(
        &self,
        timeout: Duration,
    ) -> Result<Event<KeyEvent>, std_mpsc::RecvTimeoutError> {
        self.rx.recv_timeout(timeout)
    }
}
