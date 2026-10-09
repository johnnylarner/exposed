//! Deserialize and project the Interests API response.

use crate::domain::models::parliament_member::MemberId;
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;

use super::{ApiError, ParliamentApiClient};
use crate::domain::models::declaration_ingestion::CapturedDeclaration;
use crate::outbound::declaration_source::decode_declaration;

const INTERESTS_URL: &str = "https://interests-api.parliament.uk/api/v2/Interests";

impl ParliamentApiClient {
    pub(super) async fn capture_declarations(
        &self,
        member: MemberId,
    ) -> Result<Vec<CapturedDeclaration>, ApiError> {
        let mut declarations = Vec::new();
        let mut offset = 0;
        loop {
            let url = format!(
                "{INTERESTS_URL}?MemberId={}&Type=Commons&ExcludeExpired=false&ExpandChildInterests=false&Skip={offset}&Take={}",
                member.value(),
                self.batch_size,
            );
            let bytes = self.client.get(url).send().await?.bytes().await?;
            let response: InterestPage = serde_json::from_slice(&bytes)?;
            if response.items.is_empty() {
                break;
            }
            let fetched_at = Utc::now();
            offset += response.items.len();
            for source in response.items {
                declarations.push(decode_declaration(source, fetched_at)?);
            }
            if offset >= response.total_results {
                break;
            }
        }
        Ok(declarations)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InterestPage {
    total_results: usize,
    // Retain every source field alongside the typed projection below.
    items: Vec<Value>,
}
