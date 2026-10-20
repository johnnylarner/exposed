mod entity_details;
pub use entity_details::{funder_details, member_details};

use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        models::entity_search::{
            Entity, EntitySearchError, EntitySearchRequest, SearchMatchSource,
        },
        services::{entity_details::EntityDetailsService, entity_search::EntitySearchService},
    },
    inbound::http::{error::ApiError, state::AppState, success::ApiSuccess},
};

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SearchEntitiesHttpRequest {
    term: String,
    max_entries: u8,
    strictness: Option<f32>,
}

impl SearchEntitiesHttpRequest {
    /// Converts the HTTP request body into a domain request.
    fn try_into_domain(self) -> Result<EntitySearchRequest, EntitySearchError> {
        match self.strictness {
            Some(s) => EntitySearchRequest::new_with_strictness(self.term, self.max_entries, s),
            None => EntitySearchRequest::new_with_strictness(
                self.term,
                self.max_entries,
                crate::domain::models::entity_search::DEFAULT_SEARCH_STRICTNESS,
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchEntityResponseData {
    entities: Vec<SearchEntity>,
}

impl From<&[Entity]> for SearchEntityResponseData {
    fn from(value: &[Entity]) -> Self {
        let entities = value.iter().map(SearchEntity::from).collect();
        Self { entities }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchEntity {
    id: String,
    name: String,
    kind: String,
    funder_kind: Option<String>,
    match_source: SearchMatchSource,
}

impl From<&Entity> for SearchEntity {
    fn from(value: &Entity) -> Self {
        Self {
            id: match value {
                Entity::Funder(funder) => funder.funder().id().value().to_string(),
                Entity::ParliamentMember(member) => member.member_id().to_string(),
            },
            match_source: match value {
                Entity::Funder(funder) => funder.source().clone(),
                Entity::ParliamentMember(_) => SearchMatchSource::Name,
            },
            name: value.name().to_string(),
            kind: value.kind().to_string(),
            funder_kind: value.funder_kind().map(String::from),
        }
    }
}

pub async fn search_entities<E: EntitySearchService, D: EntityDetailsService>(
    State(state): State<AppState<E, D>>,
    Query(query): Query<SearchEntitiesHttpRequest>,
) -> Result<ApiSuccess<SearchEntityResponseData>, ApiError> {
    let domain_req = query.try_into_domain()?;
    state
        .entity_search_service
        .search_entities(&domain_req)
        .await
        .map_err(ApiError::from)
        .map(|ref entities| ApiSuccess::new(StatusCode::OK, entities.as_slice().into()))
}

#[cfg(test)]
mod search_source {
    use super::{SearchEntity, SearchMatchSource};
    use crate::domain::models::{
        entity_search::{Entity, FunderSearchMatch},
        funder::{Funder, FunderId, FunderKind},
        parliament_member::{MemberId, ParliamentMember},
    };

    #[test]
    fn serializes_search_provenance_and_canonical_identity() {
        let source = SearchMatchSource::Alias {
            name: "West Midlands".into(),
        };
        let result = Entity::Funder(FunderSearchMatch::new(
            Funder::new(
                FunderId::new(uuid::Uuid::nil()),
                "Unite the Union".into(),
                FunderKind::TradeUnion,
            ),
            source,
        ));
        let json = serde_json::to_value(SearchEntity::from(&result)).unwrap();
        assert_eq!(json["name"], "Unite the Union");
        assert_eq!(json["id"], uuid::Uuid::nil().to_string());
        assert_eq!(
            json["match_source"],
            serde_json::json!({"kind": "alias", "name": "West Midlands"})
        );
        let mp = Entity::ParliamentMember(ParliamentMember::new(
            "West Member".into(),
            MemberId::new(1).unwrap(),
            "Party".into(),
            1,
            "Place".into(),
        ));
        assert_eq!(
            serde_json::to_value(SearchEntity::from(&mp)).unwrap()["match_source"],
            serde_json::json!({"kind": "name"})
        );
    }
}

#[cfg(test)]
mod search_defaults {
    use super::SearchEntitiesHttpRequest;
    use crate::domain::models::entity_search::EntitySearchRequest;

    #[test]
    fn defaults_to_point_eight_and_preserves_explicit_strictness() {
        for (json, expected) in [
            (r#"{"term":"John","max_entries":10}"#, 0.8_f32),
            (r#"{"term":"John","max_entries":10,"strictness":1}"#, 1.0),
            (r#"{"term":"John","max_entries":10,"strictness":0}"#, 0.0),
        ] {
            let request: SearchEntitiesHttpRequest = serde_json::from_str(json).unwrap();
            assert_eq!(
                request.try_into_domain().unwrap().strictness().to_bits(),
                expected.to_bits()
            );
        }
        assert_eq!(
            EntitySearchRequest::new_strict("John".into(), 10)
                .unwrap()
                .strictness()
                .to_bits(),
            1.0_f32.to_bits()
        );
    }
}
