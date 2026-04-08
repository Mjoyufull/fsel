mod defaults;
mod env;
mod error;
mod schema;

use std::fs;
use std::path::{Path, PathBuf};

pub use error::{ConfigError, ConfigValidationError};
pub use schema::FselConfig;

impl FselConfig {
    pub fn new(cli_config_path: Option<PathBuf>) -> Result<Self, ConfigError> {
        let cli_provided = cli_config_path.is_some();
        let config_path = cli_config_path.or_else(crate::app::paths::legacy_config_file_path);
        let mut cfg = load_config_file(config_path.as_deref(), cli_provided)?;
        env::apply_env_overrides(&mut cfg)?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        if self.general.systemd_run && self.general.uwsm {
            return Err(ConfigValidationError::MultipleLaunchMethods);
        }

        Ok(())
    }
}

fn load_config_file(
    config_path: Option<&Path>,
    cli_provided: bool,
) -> Result<FselConfig, ConfigError> {
    if let Some(path) = config_path {
        if path.exists() {
            let contents = fs::read_to_string(path)?;
            return Ok(toml::from_str(&contents)?);
        }

        if cli_provided {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Config file not found at {}", path.display()),
            )
            .into());
        }
    }

    Ok(FselConfig::default())
}

#[cfg(test)]
mod tests {
    use super::{ConfigError, ConfigValidationError, FselConfig, load_config_file};
    use crate::cli::{MatchMode, PinnedOrderMode, RankingMode};
    use crate::ui::PanelPosition;
    use std::fs;

    fn temp_config_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("fsel-config-{name}-{}.toml", std::process::id()))
    }

    #[test]
    fn explicit_missing_config_path_returns_not_found() {
        let path = temp_config_path("missing");
        let error = load_config_file(Some(path.as_path()), true).unwrap_err();
        assert!(matches!(error, ConfigError::Io(io) if io.kind() == std::io::ErrorKind::NotFound));
    }

    #[test]
    fn loads_explicit_config_file() {
        let path = temp_config_path("load");
        let contents = r#"
terminal_launcher = "kitty -e"
match_mode = "exact"

[dmenu]
delimiter = "|"
"#;

        fs::write(&path, contents).unwrap();

        let config = load_config_file(Some(path.as_path()), true).unwrap();
        assert_eq!(config.general.terminal_launcher, "kitty -e");
        assert_eq!(config.general.match_mode, MatchMode::Exact);
        assert_eq!(config.dmenu.delimiter.as_deref(), Some("|"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn loads_typed_legacy_alias_values() {
        let path = temp_config_path("aliases");
        let contents = r#"
ranking_mode = "frequency"
pinned_order = "newest"
title_panel_position = "middle"

[app_launcher]
match_mode = "exact"
pinned_order = "oldest"
"#;

        fs::write(&path, contents).unwrap();

        let config = load_config_file(Some(path.as_path()), true).unwrap();
        assert_eq!(config.general.ranking_mode, RankingMode::Frequency);
        assert_eq!(config.general.pinned_order, PinnedOrderMode::NewestPinned);
        assert_eq!(config.layout.title_panel_position, PanelPosition::Middle);
        assert_eq!(config.app_launcher.match_mode, Some(MatchMode::Exact));
        assert_eq!(
            config.app_launcher.pinned_order,
            Some(PinnedOrderMode::OldestPinned)
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn validate_rejects_conflicting_launch_methods() {
        let mut config = FselConfig::default();
        config.general.systemd_run = true;
        config.general.uwsm = true;

        let error = config.validate().unwrap_err();
        assert_eq!(error, ConfigValidationError::MultipleLaunchMethods);
    }
}
