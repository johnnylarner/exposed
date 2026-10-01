use std::{fs::read_to_string, io::Error as IoError, path::PathBuf};
use thiserror::Error;

use serde::Deserialize;

#[derive(Deserialize)]
/// Config for the exposed application
pub struct Config {
    /// Postgres conn string
    pub connection_string: String,
    /// Port for server to listen on
    pub port: u16,
}

impl TryFrom<&PathBuf> for Config {
    type Error = ConfigError;
    fn try_from(value: &PathBuf) -> Result<Self, Self::Error> {
        let raw = read_to_string(value)?;
        let config = rust_yaml::from_str(&raw)?;
        Ok(config)
    }
}

#[derive(Error, Debug)]
/// Collection of errors when parsing configuration
pub enum ConfigError {
    #[error("yaml malformatted")]
    /// Yaml cannot be parsed
    FormattingError(#[source] Box<rust_yaml::Error>),
    #[error("unable to read config")]
    /// Config path invalid
    FileNotFound(#[from] IoError),
}

impl From<rust_yaml::Error> for ConfigError {
    fn from(error: rust_yaml::Error) -> Self {
        Self::FormattingError(Box::new(error))
    }
}

/// Settings required only by member acquisition; PostgreSQL is optional and unused.
#[derive(Deserialize)]
pub struct MemberFetchConfig {
    /// Root of the raw capture filesystem.
    pub data_root: PathBuf,
    /// Beginning of the Parliament cohort.
    pub term_start: chrono::NaiveDate,
    /// Members API origin, overridable for local HTTP fixtures.
    #[serde(default = "members_api_url")]
    pub members_api_url: String,
}

/// Settings required only by offline member loading.
#[derive(Deserialize)]
pub struct MemberLoadConfig {
    /// Root containing the explicitly selected raw capture.
    pub data_root: PathBuf,
    /// PostgreSQL connection string.
    pub connection_string: String,
}

fn members_api_url() -> String {
    "https://members-api.parliament.uk".into()
}

/// Parse only the configuration needed by the selected data command.
///
/// # Errors
/// Returns errors for unreadable files or missing/invalid operation settings.
pub fn read_data_config<T: serde::de::DeserializeOwned>(
    path: &std::path::Path,
) -> Result<T, ConfigError> {
    Ok(rust_yaml::from_str(&read_to_string(path)?)?)
}
