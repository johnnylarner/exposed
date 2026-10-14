use super::ParliamentApiClient;
use crate::domain::{
    models::{
        declaration_ingestion::DeclarationId,
        declaration_sync::{DiscoveryReceipt, DiscoveryWindow, OwnedDeclaration},
        entity_ingestion::EntityIngestionError,
        parliament_member::MemberId,
    },
    repositories::declaration_sync::DeclarationSyncSource,
    services::declaration_sync::sha256_digest,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Page {
    total_results: usize,
    items: Vec<Value>,
}

impl ParliamentApiClient {
    async fn sync_pages(
        &self,
        filters: Vec<(&str, String)>,
    ) -> Result<(Vec<Value>, Vec<DiscoveryReceipt>), EntityIngestionError> {
        let mut offset = 0;
        let mut expected_total = None;
        let mut items = Vec::new();
        let mut seen = BTreeMap::new();
        let mut receipts = Vec::new();
        loop {
            let mut query = filters.clone();
            query.extend([
                ("Type", "Commons".into()),
                ("ExcludeExpired", "false".into()),
                ("ExpandChildInterests", "false".into()),
                ("Skip", offset.to_string()),
                ("Take", self.batch_size.to_string()),
            ]);
            let mut endpoint = reqwest::Url::parse(&self.interests_url).map_err(error)?;
            endpoint.query_pairs_mut().extend_pairs(query);
            let request = self.client.get(endpoint).build().map_err(error)?;
            let url = request.url().to_string();
            let mut response = None;
            for attempt in 0..3 {
                match self
                    .client
                    .execute(
                        request
                            .try_clone()
                            .ok_or_else(|| error("request cannot be cloned"))?,
                    )
                    .await
                {
                    Ok(result) if result.status().is_success() => {
                        response = Some(result);
                        break;
                    }
                    Ok(result)
                        if (result.status().is_server_error()
                            || result.status().as_u16() == 429)
                            && attempt < 2 => {}
                    Ok(result) => {
                        return Err(error(format!(
                            "declaration request failed with {} at {url}",
                            result.status()
                        )));
                    }
                    Err(_) if attempt < 2 => {}
                    Err(failure) => return Err(error(failure)),
                }
                tokio::time::sleep(std::time::Duration::from_millis(100 * (attempt + 1))).await;
            }
            let bytes = response
                .ok_or_else(|| error("declaration retries exhausted"))?
                .bytes()
                .await
                .map_err(error)?;
            let page: Page = serde_json::from_slice(&bytes).map_err(error)?;
            if page.total_results > 1_000_000
                || expected_total.is_some_and(|total| total != page.total_results)
                || page.items.len() > usize::from(self.batch_size.get())
                || offset + page.items.len() > page.total_results
            {
                return Err(error(
                    "inconsistent declaration pagination total or page size",
                ));
            }
            expected_total = Some(page.total_results);
            if page.items.is_empty() && offset < page.total_results {
                return Err(error(
                    "declaration pagination truncated before its reported total",
                ));
            }
            let mut identities = Vec::new();
            for item in &page.items {
                let (id, owner) = identity(item)?;
                if seen.insert(id, owner).is_some() {
                    return Err(error(
                        "duplicate declaration ID across source pages; retry reconciliation",
                    ));
                }
                identities.push((id, owner));
            }
            offset += page.items.len();
            receipts.push(DiscoveryReceipt {
                url,
                sha256: sha256_digest(&bytes),
                total: page.total_results,
                declarations: identities,
            });
            items.extend(page.items);
            if offset == page.total_results {
                break;
            }
        }
        Ok((items, receipts))
    }
}

impl DeclarationSyncSource for ParliamentApiClient {
    fn policy(&self) -> String {
        format!(
            "{}; Commons; IncludeExpired; unfiltered hydration; offset pagination; UTC date windows v1",
            self.interests_url
        )
    }
    async fn discover(
        &self,
        window: &DiscoveryWindow,
    ) -> Result<(BTreeMap<DeclarationId, MemberId>, Vec<DiscoveryReceipt>), EntityIngestionError>
    {
        let mut identities = BTreeMap::new();
        let mut receipts = Vec::new();
        for (from, through) in [
            ("PublishedFrom", "PublishedTo"),
            ("UpdatedFrom", "UpdatedTo"),
        ] {
            let (items, pages) = self
                .sync_pages(vec![
                    (from, window.from.to_string()),
                    (through, window.through.to_string()),
                ])
                .await?;
            receipts.extend(pages);
            for item in items {
                let (id, owner) = identity(&item)?;
                if identities
                    .insert(id, owner)
                    .is_some_and(|previous| previous != owner)
                {
                    return Err(error("discovery has conflicting declaration registrants"));
                }
            }
        }
        Ok((identities, receipts))
    }
    async fn hydrate(
        &self,
        id: DeclarationId,
    ) -> Result<(Option<OwnedDeclaration>, Vec<DiscoveryReceipt>), EntityIngestionError> {
        let (mut items, receipts) = self
            .sync_pages(vec![("InterestIds", id.to_string())])
            .await?;
        if items.len() > 1 {
            return Err(error("ID hydration returned multiple declarations"));
        }
        let record = items.pop().map(decode).transpose()?;
        if record
            .as_ref()
            .is_some_and(|record| record.declaration.id() != id)
        {
            return Err(error("ID hydration returned the wrong declaration"));
        }
        Ok((record, receipts))
    }
    async fn capture_member(
        &self,
        member: MemberId,
    ) -> Result<(Vec<OwnedDeclaration>, Vec<DiscoveryReceipt>), EntityIngestionError> {
        let (items, receipts) = self
            .sync_pages(vec![("MemberId", member.to_string())])
            .await?;
        let records = items
            .into_iter()
            .map(decode)
            .collect::<Result<Vec<_>, _>>()?;
        if records.iter().any(|record| record.member != member) {
            return Err(error("member capture returned another registrant"));
        }
        Ok((records, receipts))
    }
}

fn identity(value: &Value) -> Result<(DeclarationId, MemberId), EntityIngestionError> {
    #[derive(Deserialize)]
    struct Identity {
        id: DeclarationId,
        registrant: Registrant,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Registrant {
        r#type: String,
        member_detail: Member,
    }
    #[derive(Deserialize)]
    struct Member {
        id: MemberId,
    }
    let identity: Identity = serde_json::from_value(value.clone()).map_err(error)?;
    if identity.registrant.r#type != "Member" {
        return Err(error("declaration registrant is not a member"));
    }
    Ok((identity.id, identity.registrant.member_detail.id))
}
fn decode(source: Value) -> Result<OwnedDeclaration, EntityIngestionError> {
    let (_, member) = identity(&source)?;
    let declaration = crate::outbound::declaration_source::decode_declaration(source, Utc::now())
        .map_err(error)?;
    Ok(OwnedDeclaration {
        member,
        declaration,
    })
}
fn error(error: impl std::fmt::Display) -> EntityIngestionError {
    EntityIngestionError::DataError(error.to_string())
}
