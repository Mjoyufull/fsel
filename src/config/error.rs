/// Error type for config loading.
#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Toml(toml::de::Error),
    Validation(ConfigValidationError),
    InvalidEnvironmentOverride {
        key: String,
        value: String,
        expected: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigValidationError {
    MultipleLaunchMethods,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(error) => write!(f, "IO error: {error}"),
            ConfigError::Toml(error) => write!(f, "TOML parse error: {error}"),
            ConfigError::Validation(error) => write!(f, "{error}"),
            ConfigError::InvalidEnvironmentOverride {
                key,
                value,
                expected,
            } => write!(
                f,
                "Invalid environment override {key}='{value}'; expected {expected}"
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

impl std::fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MultipleLaunchMethods => write!(
                f,
                "Only one launch method can be specified at a time in configuration"
            ),
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(error: toml::de::Error) -> Self {
        Self::Toml(error)
    }
}

impl From<ConfigValidationError> for ConfigError {
    fn from(error: ConfigValidationError) -> Self {
        Self::Validation(error)
    }
}
