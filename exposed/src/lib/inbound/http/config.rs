use std::{env, num::ParseIntError};

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
    ///
    /// # Errors
    /// Returns an error if `DATABASE_URL` is missing or either value cannot be parsed.
    pub fn from_env() -> Result<Self, ServerConfigError> {
        let database_url = env_var("DATABASE_URL")?.ok_or(ServerConfigError::MissingDatabaseUrl)?;
        let connection_options = database_url
            .parse()
            .map_err(ServerConfigError::InvalidDatabaseUrl)?;
        let port = env_var("PORT")?
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

fn env_var(name: &'static str) -> Result<Option<String>, ServerConfigError> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err(ServerConfigError::NonUnicode(name)),
    }
}

/// Errors from server configuration in the environment.
#[derive(Error, Debug)]
pub enum ServerConfigError {
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
