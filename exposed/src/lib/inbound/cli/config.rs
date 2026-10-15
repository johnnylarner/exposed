//! Config for CLI

use anyhow::Context;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::{fs::read_to_string, path::PathBuf};
use thiserror::Error;

use serde::{Deserialize, Serialize};

/// Environment configuration for CLI commands that use the database.
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
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct DeclarationFetcherConfig {
    /// Root directory for ingestion datasets.
    pub data_dir: PathBuf,
    /// API page size, from 1 to 255.
    pub batch_size: u8,
}

impl TryFrom<&PathBuf> for DeclarationFetcherConfig {
    type Error = CliConfigError;

    fn try_from(value: &PathBuf) -> Result<Self, Self::Error> {
        read_config(value)
    }
}

#[derive(Serialize, Deserialize, JsonSchema)]
/// Config for the exposed application
pub struct FetcherConfig {
    /// API page size, from 1 to 255.
    pub batch_size: u8,
    /// Root directory for ingestion datasets.
    pub data_dir: PathBuf,
}

impl TryFrom<&PathBuf> for FetcherConfig {
    type Error = CliConfigError;
    fn try_from(value: &PathBuf) -> Result<Self, Self::Error> {
        read_config(value)
    }
}

#[derive(Serialize, Deserialize, JsonSchema)]
/// Config for the exposed application
pub struct LoaderConfig {
    /// Root directory for ingestion datasets.
    pub data_dir: PathBuf,
}

impl TryFrom<&PathBuf> for LoaderConfig {
    type Error = CliConfigError;
    fn try_from(value: &PathBuf) -> Result<Self, Self::Error> {
        read_config(value)
    }
}

#[derive(Error, Debug)]
/// An unreadable or malformed YAML configuration, including its selected path.
pub enum CliConfigError {
    /// The selected file could not be read.
    #[error("unable to read configuration {path}")]
    Read {
        /// Selected configuration path.
        path: PathBuf,
        /// Underlying file read failure.
        #[source]
        source: std::io::Error,
    },
    /// The selected file does not match the YAML configuration shape.
    #[error("unable to parse YAML configuration {path}")]
    Parse {
        /// Selected configuration path.
        path: PathBuf,
        /// Underlying YAML decoding failure.
        #[source]
        source: Box<rust_yaml::Error>,
    },
}

fn read_config<T: DeserializeOwned>(path: &PathBuf) -> Result<T, CliConfigError> {
    let raw = read_to_string(path).map_err(|source| CliConfigError::Read {
        path: path.clone(),
        source,
    })?;
    rust_yaml::from_str(&raw).map_err(|source| CliConfigError::Parse {
        path: path.clone(),
        source: Box::new(source),
    })
}

/// Runtime configuration for offline statistical declaration resolution.
#[derive(Deserialize, JsonSchema)]
pub struct DeclarationResolverConfig {
    /// Root directory containing the existing ingestion run.
    pub data_dir: PathBuf,
    /// Maximum profile candidates and expanded observation pairs. Must be positive.
    #[serde(default = "default_candidate_budget")]
    pub candidate_budget: usize,
}
const fn default_candidate_budget() -> usize {
    1_000_000
}
impl TryFrom<&PathBuf> for DeclarationResolverConfig {
    type Error = anyhow::Error;
    fn try_from(path: &PathBuf) -> Result<Self, Self::Error> {
        let config: Self = read_config(path)?;
        anyhow::ensure!(
            config.candidate_budget > 0,
            "configuration {}: candidate_budget must be positive",
            path.display()
        );
        Ok(config)
    }
}
