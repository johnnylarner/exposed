use anyhow::Context;

/// Configuration for the HTTP server.
pub struct ServerConfig {
    /// PostgreSQL connection string.
    pub connection_string: String,
}

impl ServerConfig {
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
