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

/// Runtime configuration for offline statistical declaration resolution.
#[derive(Deserialize)]
pub struct DeclarationResolverConfig {
    /// Root directory containing the existing ingestion run.
    pub data_dir: PathBuf,
    /// Maximum profile candidates and expanded observation pairs.
    #[serde(default = "default_candidate_budget")]
    pub candidate_budget: usize,
}
const fn default_candidate_budget() -> usize {
    1_000_000
}
impl TryFrom<&PathBuf> for DeclarationResolverConfig {
    type Error = anyhow::Error;
    fn try_from(path: &PathBuf) -> Result<Self, Self::Error> {
        let config: Self = rust_yaml::from_str(&read_to_string(path)?)?;
        anyhow::ensure!(
            config.candidate_budget > 0,
            "candidate_budget must be positive"
        );
        Ok(config)
    }
}

/// Declaration-only synchronization configuration.
#[derive(Deserialize)]
pub struct DeclarationSyncConfig {
    /// Durable artifact root, retained for every committed snapshot.
    pub data_dir: PathBuf,
    /// Nonzero Parliament page size.
    pub batch_size: u8,
    /// Global resolution candidate limit.
    #[serde(default = "default_candidate_budget")]
    pub candidate_budget: usize,
    /// Previous source dates to rediscover.
    #[serde(default = "default_overlap_days")]
    pub overlap_days: u64,
    /// UTC dates excluded to allow source indexing.
    #[serde(default = "default_indexing_lag_days")]
    pub indexing_lag_days: u64,
    /// Explicit inclusive source cutoff, otherwise UTC today minus lag.
    pub cutoff: Option<chrono::NaiveDate>,
    /// Official production endpoint, injectable for deterministic source tests.
    #[serde(default = "default_interests_url")]
    pub interests_url: String,
}
const fn default_overlap_days() -> u64 {
    7
}
const fn default_indexing_lag_days() -> u64 {
    1
}
fn default_interests_url() -> String {
    "https://interests-api.parliament.uk/api/v2/Interests".into()
}
impl TryFrom<&PathBuf> for DeclarationSyncConfig {
    type Error = anyhow::Error;
    fn try_from(path: &PathBuf) -> Result<Self, Self::Error> {
        let config: Self = rust_yaml::from_str(&read_to_string(path)?)?;
        anyhow::ensure!(
            config.batch_size > 0 && config.candidate_budget > 0,
            "batch_size and candidate_budget must be positive"
        );
        Ok(config)
    }
}
