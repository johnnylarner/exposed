//! Minimal client for the UK parliament API

use std::num::NonZeroU8;

use serde::{Deserialize, de::DeserializeOwned};
use thiserror::Error;

use reqwest::{Client, StatusCode};

use crate::domain::{
    models::{
        declaration_ingestion::CapturedDeclaration,
        entity_ingestion::EntityIngestionError,
        membership::{MemberProfile, SourceMembership},
        parliament_member::{MemberId, ParliamentMember},
    },
    repositories::parliament_api::{ParliamentApi as Interface, ParliamentApiError},
};

const BASE_URL: &str = "https://members-api.parliament.uk";

mod declarations;

/// Parliament API implementation
#[derive(Clone)]
pub struct ParliamentApiClient {
    client: Client,
    batch_size: NonZeroU8,
}

impl ParliamentApiClient {
    /// Creates a new instance
    ///
    /// # Errors
    /// - Client error
    pub fn new(batch_size: u8) -> Result<Self, ApiError> {
        let batch_size = NonZeroU8::new(batch_size)
            .ok_or_else(|| ApiError::BuilderError("batch size must be nonzero".into()))?;
        let client = Client::builder()
            .build()
            .map_err(|e| ApiError::BuilderError(e.to_string()))?;

        Ok(Self { client, batch_size })
    }
}

impl Interface for ParliamentApiClient {
    async fn get_sitting_members(&self) -> Result<Vec<ParliamentMember>, ParliamentApiError> {
        let captured_at = chrono::Utc::now();
        let capture_key = uuid::Uuid::now_v7();
        let batch = self.batch_size;
        let mut offset = 0;
        let mut members = Vec::new();
        loop {
            let url = format!(
                "{BASE_URL}/api/Members/Search?House=1&IsCurrentMember=true&skip={offset}&take={batch}"
            );
            let result = self
                .client
                .get(&url)
                .send()
                .await
                .map_err(|e| ApiError::ResponseError(e.to_string()))?;

            let status = result.status();
            let bytes = result
                .bytes()
                .await
                .map_err(|e| ApiError::ResponseError(e.to_string()))?;

            let response: ParliamentResponse = decode_response(&bytes, &url, status)?;

            offset += response.items.len();
            members.extend(response.items);

            if members.len() == response.total_results {
                break;
            }
        }

        members
            .into_iter()
            .map(|m| {
                Ok(ParliamentMember {
                    profile: MemberProfile {
                        id: MemberId::new(m.value.id)
                            .map_err(|e| ApiError::ResponseError(e.to_string()))?,
                        name: m.value.name,
                        party_name: m.value.party.as_ref().and_then(|party| party.name.clone()),
                        party_id: m.value.party.and_then(|party| party.id),
                        membership: m
                            .value
                            .membership
                            .into_membership()
                            .map_err(ApiError::ResponseError)?,
                    },
                    captured_at: Some(captured_at),
                    capture_key: Some(capture_key),
                })
            })
            .collect()
    }

    async fn get_declarations(
        &self,
        member: MemberId,
    ) -> Result<Vec<CapturedDeclaration>, ParliamentApiError> {
        self.capture_declarations(member).await.map_err(Into::into)
    }
}

#[derive(Clone, Deserialize)]
struct ParliamentResponse {
    items: Vec<NestedMember>,
    #[serde(rename(deserialize = "totalResults"))]
    total_results: usize,
}

#[derive(Clone, Deserialize)]
struct NestedMember {
    value: ParliamentMemberItem,
}

#[derive(Clone, Deserialize)]
struct ParliamentMemberItem {
    id: u32,
    #[serde(rename(deserialize = "nameDisplayAs"))]
    name: String,
    #[serde(rename(deserialize = "latestParty"))]
    party: Option<PartyDetails>,
    #[serde(rename(deserialize = "latestHouseMembership"))]
    membership: SourceMembership,
}

#[derive(Clone, Deserialize)]
struct PartyDetails {
    id: Option<u32>,
    name: Option<String>,
}

/// Possible parliament api errors
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("error when building client: {0}")]
    BuilderError(String),

    #[error("error when calling the client: {0}")]
    ResponseError(String),
}

impl From<ApiError> for ParliamentApiError {
    fn from(value: ApiError) -> Self {
        match value {
            ApiError::BuilderError(e) | ApiError::ResponseError(e) => Self::ApiError(e),
        }
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(error: reqwest::Error) -> Self {
        Self::ResponseError(error.to_string())
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(error: serde_json::Error) -> Self {
        Self::ResponseError(error.to_string())
    }
}

impl From<EntityIngestionError> for ApiError {
    fn from(error: EntityIngestionError) -> Self {
        Self::ResponseError(error.to_string())
    }
}

fn decode_response<T: DeserializeOwned>(
    bytes: &[u8],
    url: &str,
    status: StatusCode,
) -> Result<T, ApiError> {
    serde_json::from_slice(bytes).map_err(|error| {
        ApiError::ResponseError(format!(
            "failed to decode response from {url} (HTTP {status}): {error}\nResponse body:\n{}",
            String::from_utf8_lossy(bytes),
        ))
    })
}
