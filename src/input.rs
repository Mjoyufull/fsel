use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use termion::event::Key;
use termion::input::TermRead;

/// Builder for `Input`
///
/// For now, you can only configure the exit key (Esc by default).
/// But in the future, there may be some more interesting configuration options...
///
/// # Example
/// ```rust
/// // Build a default `Input` (Esc ends the handling thread)
/// let input = Config::default().init();
/// // Customize the exit key
/// let input = Config {
///     exit_key: Key::Backspace,
///     ..Default::default()
/// }.init();
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub exit_key: Key,
    pub tick_rate: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            exit_key: Key::Esc,
            tick_rate: Duration::from_millis(250),
        }
    }
}

impl Config {
    /// Creates a new `Input` with the configuration in `Self`
    pub fn init(self) -> Input {
        Input::with_config(self)
    }
}

pub enum Event<I> {
    Input(I),
    Tick,
}

/// Small input handler. Uses Termion as the backend.
pub struct Input {
    rx: mpsc::Receiver<Event<Key>>,
    _input_handle: thread::JoinHandle<()>,
    _tick_handle: thread::JoinHandle<()>,
}

impl Input {
    pub fn new() -> Self {
        Config::default().init()
    }

    pub fn with_config(config: Config) -> Self {
        let (tx, rx) = mpsc::channel();

        let _input_handle = {
            let tx = tx.clone();

            thread::spawn(move || {
                let stdin = io::stdin();
                for key in stdin.keys().flatten() {
                    if tx.send(Event::Input(key)).is_err() {
                        return;
                    }
                    if key == config.exit_key {
                        return;
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
    pub fn next(&self) -> Result<Event<Key>, mpsc::RecvError> {
        self.rx.recv()
    }
}
