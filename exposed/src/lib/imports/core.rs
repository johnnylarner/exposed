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
    #[error("Source operation failed: {operation}")]
    SourceFailure {
        operation: String,
        #[source]
        cause: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("Database operation failed")]
    Storage(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("Another import is running")]
    Busy,
}

impl ImportError {
    pub fn source_failure(
        operation: impl Into<String>,
        cause: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::SourceFailure {
            operation: operation.into(),
            cause: Box::new(cause),
        }
    }
}

pub(crate) type Result<T> = std::result::Result<T, ImportError>;
pub(crate) mod coordinator;
pub(crate) mod declarations;
pub(crate) mod ports;
pub(crate) mod refresh;
