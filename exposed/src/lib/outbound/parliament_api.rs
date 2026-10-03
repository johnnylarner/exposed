//! Minimal client for the UK parliament API

use std::num::NonZeroU8;

use serde::Deserialize;
use thiserror::Error;

use reqwest::Client;

use crate::domain::{
    models::{
        declaration_ingestion::{CapturedDeclaration, StoredMember},
        entity_ingestion::EntityIngestionError,
        parliament_member::ParliamentMember,
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
        let batch = self.batch_size;
        let mut offset = 0;
        let mut members = Vec::new();
        loop {
            let url = format!(
                "{BASE_URL}/api/Members/Search?House=1&IsCurrentMember=true&skip={offset}&take={batch}"
            );
            let result = self
                .client
                .get(url)
                .send()
                .await
                .map_err(|e| ApiError::ResponseError(e.to_string()))?;

            let bytes = result
                .bytes()
                .await
                .map_err(|e| ApiError::ResponseError(e.to_string()))?;

            let response: ParliamentResponse = serde_json::from_slice(&bytes)
                .map_err(|e| ApiError::ResponseError(e.to_string()))?;

            offset += response.items.len();
            members.extend(response.items);

            if members.len() == response.total_results {
                break;
            }
        }

        Ok(members
            .into_iter()
            .map(|m| {
                ParliamentMember::new(
                    m.value.name,
                    m.value.id,
                    m.value.party.name,
                    m.value.party.id,
                    m.value.membership.membership_from,
                )
            })
            .collect())
    }

    async fn get_declarations(
        &self,
        member: StoredMember,
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
    party: PartyDetails,
    #[serde(rename(deserialize = "latestHouseMembership"))]
    membership: MembershipDetails,
}

#[derive(Clone, Deserialize)]
struct PartyDetails {
    id: u32,
    name: String,
}

#[derive(Clone, Deserialize)]
struct MembershipDetails {
    #[serde(rename(deserialize = "membershipFrom"))]
    membership_from: String,
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
