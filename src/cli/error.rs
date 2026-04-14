use crate::config::ConfigError;
use std::process::ExitCode;

#[derive(Debug)]
pub enum CliError {
    Config(ConfigError),
    Parse(lexopt::Error),
    Message(String),
}

impl CliError {
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }

    pub fn exit_code(&self) -> ExitCode {
        ExitCode::from(1)
    }

    pub fn render(&self) -> String {
        match self {
            Self::Config(error) => format!("Error loading configuration: {error}\n"),
            Self::Parse(error) => format!("Error: {error}\n"),
            Self::Message(message) => message.clone(),
        }
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.render())
    }
}

impl std::error::Error for CliError {}

impl From<ConfigError> for CliError {
    fn from(error: ConfigError) -> Self {
        Self::Config(error)
    }
}

impl From<lexopt::Error> for CliError {
    fn from(error: lexopt::Error) -> Self {
        Self::Parse(error)
    }
}
