// keybind configuration

use crossterm::event::{KeyCode, KeyModifiers};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Keybinds {
    #[serde(default = "default_up")]
    pub up: Vec<KeyBind>,
    #[serde(default = "default_down")]
    pub down: Vec<KeyBind>,
    #[serde(default = "default_left")]
    pub left: Vec<KeyBind>,
    #[serde(default = "default_right")]
    pub right: Vec<KeyBind>,
    #[serde(default = "default_select")]
    pub select: Vec<KeyBind>,
    #[serde(default = "default_exit")]
    pub exit: Vec<KeyBind>,
    #[serde(default = "default_pin")]
    pub pin: Vec<KeyBind>,
    #[serde(default = "default_backspace")]
    pub backspace: Vec<KeyBind>,
    #[serde(default = "default_image_preview")]
    pub image_preview: Vec<KeyBind>,
    #[serde(default = "default_tag")]
    #[allow(dead_code)]
    pub tag: Vec<KeyBind>,
}

impl Default for Keybinds {
    fn default() -> Self {
        Self {
            up: default_up(),
            down: default_down(),
            left: default_left(),
            right: default_right(),
            select: default_select(),
            exit: default_exit(),
            pin: default_pin(),
            backspace: default_backspace(),
            image_preview: default_image_preview(),
            tag: default_tag(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum KeyBind {
    Simple(String),
    WithMod { key: String, modifiers: String },
}

impl KeyBind {
    pub fn matches(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        match self {
            KeyBind::Simple(key) => {
                let parsed = parse_key(key);
                parsed.0 == code && mods == KeyModifiers::NONE
            }
            KeyBind::WithMod { key, modifiers } => {
                let parsed = parse_key(key);
                let parsed_mods = parse_modifiers(modifiers);
                parsed.0 == code && mods == parsed_mods
            }
        }
    }
}

fn parse_key(key: &str) -> (KeyCode, KeyModifiers) {
    match key.to_lowercase().as_str() {
        "up" => (KeyCode::Up, KeyModifiers::NONE),
        "down" => (KeyCode::Down, KeyModifiers::NONE),
        "left" => (KeyCode::Left, KeyModifiers::NONE),
        "right" => (KeyCode::Right, KeyModifiers::NONE),
        "enter" => (KeyCode::Enter, KeyModifiers::NONE),
        "esc" | "escape" => (KeyCode::Esc, KeyModifiers::NONE),
        "backspace" => (KeyCode::Backspace, KeyModifiers::NONE),
        "space" => (KeyCode::Char(' '), KeyModifiers::NONE),
        s if s.len() == 1 => (KeyCode::Char(s.chars().next().unwrap()), KeyModifiers::NONE),
        _ => (KeyCode::Null, KeyModifiers::NONE),
    }
}

fn parse_modifiers(mods: &str) -> KeyModifiers {
    let mut result = KeyModifiers::NONE;
    for part in mods.split('+') {
        match part.trim().to_lowercase().as_str() {
            "ctrl" | "control" => result |= KeyModifiers::CONTROL,
            "shift" => result |= KeyModifiers::SHIFT,
            "alt" => result |= KeyModifiers::ALT,
            _ => {}
        }
    }
    result
}

fn default_up() -> Vec<KeyBind> {
    vec![
        KeyBind::Simple("up".to_string()),
        KeyBind::WithMod {
            key: "p".to_string(),
            modifiers: "ctrl".to_string(),
        },
    ]
}

fn default_down() -> Vec<KeyBind> {
    vec![
        KeyBind::Simple("down".to_string()),
        KeyBind::WithMod {
            key: "n".to_string(),
            modifiers: "ctrl".to_string(),
        },
    ]
}

fn default_left() -> Vec<KeyBind> {
    vec![KeyBind::Simple("left".to_string())]
}

fn default_right() -> Vec<KeyBind> {
    vec![KeyBind::Simple("right".to_string())]
}

fn default_select() -> Vec<KeyBind> {
    vec![
        KeyBind::Simple("enter".to_string()),
        KeyBind::WithMod {
            key: "y".to_string(),
            modifiers: "ctrl".to_string(),
        },
    ]
}

fn default_exit() -> Vec<KeyBind> {
    vec![
        KeyBind::Simple("esc".to_string()),
        KeyBind::WithMod {
            key: "q".to_string(),
            modifiers: "ctrl".to_string(),
        },
        KeyBind::WithMod {
            key: "c".to_string(),
            modifiers: "ctrl".to_string(),
        },
    ]
}

fn default_pin() -> Vec<KeyBind> {
    vec![KeyBind::WithMod {
        key: "space".to_string(),
        modifiers: "ctrl".to_string(),
    }]
}

fn default_backspace() -> Vec<KeyBind> {
    vec![KeyBind::Simple("backspace".to_string())]
}

fn default_image_preview() -> Vec<KeyBind> {
    // Note: Ctrl+I is the same as Tab in terminals, so we use Alt+I instead
    vec![KeyBind::WithMod {
        key: "i".to_string(),
        modifiers: "alt".to_string(),
    }]
}

fn default_tag() -> Vec<KeyBind> {
    vec![KeyBind::WithMod {
        key: "t".to_string(),
        modifiers: "ctrl".to_string(),
    }]
}

impl Keybinds {
    pub fn matches_up(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.up.iter().any(|kb| kb.matches(code, mods))
    }

    pub fn matches_down(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.down.iter().any(|kb| kb.matches(code, mods))
    }

    pub fn matches_left(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.left.iter().any(|kb| kb.matches(code, mods))
    }

    pub fn matches_right(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.right.iter().any(|kb| kb.matches(code, mods))
    }

    pub fn matches_select(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.select.iter().any(|kb| kb.matches(code, mods))
    }

    pub fn matches_exit(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.exit.iter().any(|kb| kb.matches(code, mods))
    }

    pub fn matches_pin(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.pin.iter().any(|kb| kb.matches(code, mods))
    }

    pub fn matches_backspace(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.backspace.iter().any(|kb| kb.matches(code, mods))
    }

    pub fn matches_image_preview(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.image_preview.iter().any(|kb| kb.matches(code, mods))
    }

    /// DISABLED: Waiting for cclip maintainer to add tag support
    #[allow(dead_code)]
    pub fn matches_tag(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.tag.iter().any(|kb| kb.matches(code, mods))
    }
}
