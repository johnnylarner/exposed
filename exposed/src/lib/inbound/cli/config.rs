//! Config for CLI

use std::{fs::read_to_string, io::Error as IoError, path::PathBuf};
use thiserror::Error;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
/// Config for the exposed application
pub struct FetcherConfig {
    /// Size of API batches
    pub batch_size: u8,
    /// Data store key
    pub data_dir: PathBuf,
}

impl TryFrom<&PathBuf> for FetcherConfig {
    type Error = CliConfigError;
    fn try_from(value: &PathBuf) -> Result<Self, Self::Error> {
        let raw = read_to_string(value)?;
        let config = rust_yaml::from_str(&raw)?;
        Ok(config)
    }
}

#[derive(Serialize, Deserialize)]
/// Config for the exposed application
pub struct LoaderConfig {
    /// Database connection string
    pub connection_string: String,
    /// Data store key
    pub data_dir: PathBuf,
}

impl TryFrom<&PathBuf> for LoaderConfig {
    type Error = CliConfigError;
    fn try_from(value: &PathBuf) -> Result<Self, Self::Error> {
        let raw = read_to_string(value)?;
        let config = rust_yaml::from_str(&raw)?;
        Ok(config)
    }
}

#[derive(Error, Debug)]
/// Collection of errors when parsing configuration
pub enum CliConfigError {
    #[error("yaml malformatted")]
    /// Yaml cannot be parsed
    FormattingError(#[from] rust_yaml::Error),
    #[error("unable to read config")]
    /// Config path invalid
    FileNotFound(#[from] IoError),
}
