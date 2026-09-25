pub(crate) mod members;

use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum ImportError {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Rejected(String),
    #[error("Source operation failed: {0}")]
    Source(String),
    #[error("Database operation failed")]
    Storage(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("Another import is running")]
    Busy,
}

pub(crate) type Result<T> = std::result::Result<T, ImportError>;
pub(crate) mod coordinator;
pub(crate) mod declarations;
pub(crate) mod ports;
pub(crate) mod refresh;
