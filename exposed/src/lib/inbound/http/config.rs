use std::{collections::HashMap, env, io, num::ParseIntError};

use sqlx::postgres::PgConnectOptions;
use thiserror::Error;

/// Configuration for the HTTP server.
pub struct ServerConfig {
    /// Parsed PostgreSQL connection options.
    pub connection_options: PgConnectOptions,
    /// Port for the server to listen on.
    pub port: u16,
}

impl ServerConfig {
    /// Reads `DATABASE_URL` and optional `PORT` (default: 6999) from the environment.
    /// Missing values fall back to `.env` in the current directory or nearest parent.
    ///
    /// # Errors
    /// Returns an error if `.env` cannot be read or parsed, `DATABASE_URL` is missing,
    /// or either configuration value cannot be parsed.
    pub fn from_env() -> Result<Self, ServerConfigError> {
        let file_values = dotenv_values()?;
        let database_url =
            env_var("DATABASE_URL", &file_values)?.ok_or(ServerConfigError::MissingDatabaseUrl)?;
        let connection_options = database_url
            .parse()
            .map_err(ServerConfigError::InvalidDatabaseUrl)?;
        let port = env_var("PORT", &file_values)?
            .map(|value| value.parse())
            .transpose()
            .map_err(ServerConfigError::InvalidPort)?
            .unwrap_or(6999);

        Ok(Self {
            connection_options,
            port,
        })
    }
}

fn dotenv_values() -> Result<HashMap<String, String>, ServerConfigError> {
    let entries = match dotenvy::dotenv_iter() {
        Ok(entries) => entries,
        Err(error) if error.not_found() => return Ok(HashMap::new()),
        Err(error) => return Err(error.into()),
    };

    // Read file values without changing the environment of the running process.
    let mut values = HashMap::new();
    for entry in entries {
        let (key, value) = entry?;
        values.entry(key).or_insert(value);
    }
    Ok(values)
}

fn env_var(
    name: &'static str,
    file_values: &HashMap<String, String>,
) -> Result<Option<String>, ServerConfigError> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(file_values.get(name).cloned()),
        Err(env::VarError::NotUnicode(_)) => Err(ServerConfigError::NonUnicode(name)),
    }
}

/// Errors from server configuration in the environment.
#[derive(Error, Debug)]
pub enum ServerConfigError {
    /// The optional `.env` file cannot be read.
    #[error("unable to read .env")]
    EnvFileRead(#[source] io::Error),
    /// The `.env` file contains invalid syntax.
    #[error("unable to parse .env")]
    EnvFileFormat,
    /// The required database URL is absent.
    #[error("DATABASE_URL must be set")]
    MissingDatabaseUrl,
    /// An environment variable contains non-Unicode data.
    #[error("{0} must contain valid Unicode")]
    NonUnicode(&'static str),
    /// `SQLx` cannot parse the database URL.
    #[error("DATABASE_URL must be a valid PostgreSQL connection URL")]
    InvalidDatabaseUrl(#[source] sqlx::Error),
    /// The port cannot be parsed as a 16-bit unsigned integer.
    #[error("PORT must be an integer between 0 and 65535")]
    InvalidPort(#[source] ParseIntError),
}

impl From<dotenvy::Error> for ServerConfigError {
    fn from(error: dotenvy::Error) -> Self {
        match error {
            dotenvy::Error::Io(error) => Self::EnvFileRead(error),
            // Parser errors include file contents, which can contain credentials.
            _ => Self::EnvFileFormat,
        }
    }
}
