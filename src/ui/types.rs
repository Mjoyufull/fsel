use serde::Deserialize;
use std::str::FromStr;

/// Title panel position
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum PanelPosition {
    /// Panel at the top (default behavior)
    #[default]
    Top,
    /// Panel in the middle (where results/apps usually are)
    Middle,
    /// Panel at the bottom (above input field)
    Bottom,
}

impl FromStr for PanelPosition {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "top" => Ok(PanelPosition::Top),
            "middle" => Ok(PanelPosition::Middle),
            "bottom" => Ok(PanelPosition::Bottom),
            _ => Err(format!(
                "Invalid panel position: '{}'. Valid options: top, middle, bottom",
                s
            )),
        }
    }
}
