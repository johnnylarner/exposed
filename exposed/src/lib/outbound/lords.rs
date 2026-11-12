//! Lords source, immutable artifacts, and isolated PostgreSQL projection.

mod database;
mod storage;

pub use storage::LordsCaptureFiles;

use crate::domain::{models::lords::LordsError, repositories::lords::LordsSource};

const REGISTER_URL: &str = "https://members-api.parliament.uk/api/LordsInterests/Register";

/// Parliament's full Lords register endpoint.
pub struct LordsApiClient {
    client: reqwest::Client,
}

impl LordsApiClient {
    /// Creates a client with bounded network waits.
    ///
    /// # Errors
    /// Returns a transport configuration error.
    pub fn new() -> Result<Self, LordsError> {
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(90))
                .build()
                .map_err(|error| LordsError::Source(error.to_string()))?,
        })
    }
}

impl LordsSource for LordsApiClient {
    async fn page(&self, page: u32) -> Result<Vec<u8>, LordsError> {
        let page = page.max(1);
        self.client
            .get(format!("{REGISTER_URL}?page={page}&includeDeleted=true"))
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|error| LordsError::Source(error.to_string()))?
            .bytes()
            .await
            .map(|bytes| bytes.to_vec())
            .map_err(|error| LordsError::Source(error.to_string()))
    }
}

#[cfg(test)]
mod tests;
