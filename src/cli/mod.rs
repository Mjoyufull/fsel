mod color;
mod from_config;
mod help;
mod launch;
mod parse;
mod types;
mod validate;

use std::sync::atomic::AtomicBool;

pub use color::string_to_color;
pub use parse::parse;
pub use types::{MatchMode, Opts, PinnedOrderMode, RankingMode};

pub use crate::ui::PanelPosition;

pub static DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);
