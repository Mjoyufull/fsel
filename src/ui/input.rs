use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, MouseEvent};

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
    /// Creates a new `Input` with the configuration in `Self`
    pub fn init(self) -> Input {
        Input::with_config(self)
    }
}

#[derive(Debug)]
pub enum Event<I> {
    Input(I),
    Mouse(MouseEvent),
    Tick,
}

/// Small input handler. Uses crossterm as the backend.
pub struct Input {
    rx: mpsc::Receiver<Event<KeyEvent>>,
    _input_handle: thread::JoinHandle<()>,
    _tick_handle: thread::JoinHandle<()>,
}

impl Input {
    pub fn with_config(config: Config) -> Self {
        let (tx, rx) = mpsc::channel();

        let _input_handle = {
            let tx = tx.clone();

            thread::spawn(move || loop {
                if let Ok(true) = event::poll(Duration::from_millis(100)) {
                    if let Ok(event) = event::read() {
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
    pub fn next(&self) -> Result<Event<KeyEvent>, mpsc::RecvError> {
        self.rx.recv()
    }

    /// Next key pressed by user with timeout.
    #[allow(dead_code)]
    pub fn next_timeout(
        &self,
        timeout: Duration,
    ) -> Result<Event<KeyEvent>, mpsc::RecvTimeoutError> {
        self.rx.recv_timeout(timeout)
    }
}
