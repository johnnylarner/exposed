//! Minimal client for the UK parliament API

use serde::Deserialize;
use thiserror::Error;

use reqwest::Client;

use crate::domain::{
    models::parliament_member::ParliamentMember,
    repositories::parliament_api::{ParliamentApi as Interface, ParliamentApiError},
};

const BASE_URL: &str = "https://members-api.parliament.uk";

/// Parliament API implementation
#[derive(Clone)]
pub struct ParliamentApiClient {
    client: Client,
    batch_size: u8,
}

impl ParliamentApiClient {
    /// Creates a new instance
    ///
    /// # Errors
    /// - Client error
    pub fn new(batch_size: u8) -> Result<Self, ApiError> {
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

            println!("retrieved {batch} records from offset {offset}");
            offset += response.items.len();
            members.extend(response.items);

            if members.len() == response.total_results {
                println!("received all members");
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
                    m.value.membership.membership_from,
                )
            })
            .collect())
    }

    async fn get_declarations_for_sitting_members(&self) -> Result<(), ParliamentApiError> {
        todo!("still needs doing")
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
