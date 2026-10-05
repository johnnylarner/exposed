//! Config for CLI

use anyhow::Context;
use std::{fs::read_to_string, io::Error as IoError, path::PathBuf};
use thiserror::Error;

use serde::{Deserialize, Serialize};

/// Environment configuration for data commands that use the database.
pub struct DataConfig {
    /// PostgreSQL connection string.
    pub connection_string: String,
}

impl DataConfig {
    /// Reads the database connection string from the environment.
    ///
    /// # Errors
    /// Returns an error if `DATABASE_URL` is missing or contains non-Unicode data.
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            connection_string: std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?,
        })
    }
}

/// Configuration for capturing declarations for the stored member cohort.
#[derive(Serialize, Deserialize)]
pub struct DeclarationFetcherConfig {
    /// Root directory for ingestion datasets.
    pub data_dir: PathBuf,
    /// Nonzero API page size; validated when constructing the HTTP adapter.
    pub batch_size: u8,
}

impl TryFrom<&PathBuf> for DeclarationFetcherConfig {
    type Error = CliConfigError;

    fn try_from(value: &PathBuf) -> Result<Self, Self::Error> {
        let raw = read_to_string(value)?;
        Ok(rust_yaml::from_str(&raw)?)
    }
}

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
