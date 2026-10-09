//! Entity search refers to the landing page of the `exposed` application. Its purpose it to make it
//! easy for journalists to find [`parliament_members`] or [`funders`].
//!

mod error;
mod interface;

use crate::domain::models::entity_search::FunderSearchMatch;
use crate::domain::models::entity_search::{Entity, EntitySearchError, EntitySearchRequest};
use crate::domain::models::parliament_member::ParliamentMember;
use crate::domain::models::search_similarity::SearchSimilarity;
use crate::domain::repositories::funder_repository::FunderRepo;
use crate::domain::repositories::parliament_member_repository::ParliamentMemberRepo;
pub use crate::domain::services::entity_search::interface::EntitySearchService;

/// Allows users to search the databse using free text.
#[derive(Clone)]
pub struct Service<P, F> {
    mp_repo: P,
    funder_repo: F,
}

impl<P, F> Service<P, F> {
    /// Creates a new instance
    pub const fn new(mp_repo: P, funder_repo: F) -> Self {
        Self {
            mp_repo,
            funder_repo,
        }
    }
}

impl<P, F> EntitySearchService for Service<P, F>
where
    P: ParliamentMemberRepo,
    F: FunderRepo,
{
    /// Returns text search results for MPs and Funder entities.
    ///
    /// This function expects the repositories to return their results
    /// in order where a higher score means a higher similarity.
    async fn search_entities(
        &self,
        req: &EntitySearchRequest,
    ) -> Result<Vec<Entity>, EntitySearchError> {
        let mps = self.mp_repo.get_members_by_text_search_score(req).await?;
        let funders = self
            .funder_repo
            .get_funders_by_text_search_score(req)
            .await?;

        let scored = merge_scores(&mps, &funders);
        Ok(scored)
    }
}

/// Uses two-pointer technique to merge to pre-sorted
/// search results together.
fn merge_scores(
    mps: &[(ParliamentMember, SearchSimilarity)],
    funders: &[(FunderSearchMatch, SearchSimilarity)],
) -> Vec<Entity> {
    let mut scores = Vec::new();

    let (mut l, mut r) = (0, 0);
    // Unwrap acceptable as index check in
    // while function
    while l < mps.len() && r < funders.len() {
        let mp: &(ParliamentMember, SearchSimilarity) = mps.get(l).unwrap();
        let fund: &(FunderSearchMatch, SearchSimilarity) = funders.get(r).unwrap();

        match &mp.1.value().total_cmp(&fund.1.value()) {
            std::cmp::Ordering::Equal => {
                scores.push(Entity::from(&mp.0));
                scores.push(Entity::from(&fund.0));
                l += 1;
                r += 1;
            }
            std::cmp::Ordering::Greater => {
                scores.push(Entity::from(&mp.0));
                l += 1;
            }
            std::cmp::Ordering::Less => {
                scores.push(Entity::from(&fund.0));
                r += 1;
            }
        }
    }
    if l < mps.len() {
        for mp in &mps[l..] {
            scores.push(Entity::from(&mp.0));
        }
    }
    if r < funders.len() {
        for fund in &funders[r..] {
            scores.push(Entity::from(&fund.0));
        }
    }

    scores
}

#[cfg(test)]
mod merge_scores {
    use crate::domain::{
        models::{
            entity_search::{Entity, FunderSearchMatch, SearchMatchSource},
            funder::{Funder, FunderKind},
            parliament_member::{MemberId, ParliamentMember},
            search_similarity::SearchSimilarity,
        },
        services::entity_search::merge_scores,
    };

    #[test]
    fn works_for_only_mps() {
        let funders = Vec::new();
        let mps = vec![
            (
                ParliamentMember::new(
                    "johnny larner".into(),
                    MemberId::new(1).unwrap(),
                    String::new(),
                    1,
                    String::new(),
                ),
                SearchSimilarity::from(1_f32),
            ),
            (
                ParliamentMember::new(
                    "hades".into(),
                    MemberId::new(1).unwrap(),
                    String::new(),
                    1,
                    String::new(),
                ),
                SearchSimilarity::from(2_f32),
            ),
        ];

        let ranked = merge_scores(&mps, &funders);
        assert_eq!(
            ranked.first().unwrap(),
            &Entity::from(&ParliamentMember::new(
                "johnny larner".into(),
                MemberId::new(1).unwrap(),
                String::new(),
                1,
                String::new(),
            ))
        );

        assert_eq!(
            ranked.get(1).unwrap(),
            &Entity::from(&ParliamentMember::new(
                "hades".into(),
                MemberId::new(1).unwrap(),
                String::new(),
                1,
                String::new(),
            ))
        );
    }

    #[test]
    fn works_for_balanced_results() {
        let funders = vec![
            (
                FunderSearchMatch::new(
                    Funder::new(
                        crate::domain::models::funder::FunderId::new(uuid::Uuid::nil()),
                        "heavenly ltd".into(),
                        FunderKind::Company,
                    ),
                    SearchMatchSource::Name,
                ),
                SearchSimilarity::from(4_f32),
            ),
            (
                FunderSearchMatch::new(
                    Funder::new(
                        crate::domain::models::funder::FunderId::new(uuid::Uuid::nil()),
                        "canna ltd".into(),
                        FunderKind::Company,
                    ),
                    SearchMatchSource::Name,
                ),
                SearchSimilarity::from(2_f32),
            ),
        ];
        let mps = vec![
            (
                ParliamentMember::new(
                    "johnny larner".into(),
                    MemberId::new(1).unwrap(),
                    String::new(),
                    1,
                    String::new(),
                ),
                SearchSimilarity::from(4_f32),
            ),
            (
                ParliamentMember::new(
                    "hades".into(),
                    MemberId::new(1).unwrap(),
                    String::new(),
                    1,
                    String::new(),
                ),
                SearchSimilarity::from(3_f32),
            ),
        ];

        let ranked = merge_scores(&mps, &funders);
        assert_eq!(ranked.len(), 4);
        assert_eq!(
            ranked.first().unwrap(),
            &Entity::from(&ParliamentMember::new(
                "johnny larner".into(),
                MemberId::new(1).unwrap(),
                String::new(),
                1,
                String::new(),
            ))
        );
        assert_eq!(
            ranked.get(1).unwrap(),
            &Entity::from(&FunderSearchMatch::new(
                Funder::new(
                    crate::domain::models::funder::FunderId::new(uuid::Uuid::nil()),
                    "heavenly ltd".into(),
                    FunderKind::Company
                ),
                SearchMatchSource::Name
            ),)
        );
        assert_eq!(
            ranked.get(2).unwrap(),
            &Entity::from(&ParliamentMember::new(
                "hades".into(),
                MemberId::new(1).unwrap(),
                String::new(),
                1,
                String::new(),
            ))
        );
        assert_eq!(
            ranked.get(3).unwrap(),
            &Entity::from(&FunderSearchMatch::new(
                Funder::new(
                    crate::domain::models::funder::FunderId::new(uuid::Uuid::nil()),
                    "canna ltd".into(),
                    FunderKind::Company
                ),
                SearchMatchSource::Name
            ),)
        );
    }

    #[test]
    fn works_for_more_funders() {
        let funders = vec![
            (
                FunderSearchMatch::new(
                    Funder::new(
                        crate::domain::models::funder::FunderId::new(uuid::Uuid::nil()),
                        "heavenly ltd".into(),
                        FunderKind::Company,
                    ),
                    SearchMatchSource::Name,
                ),
                SearchSimilarity::from(4_f32),
            ),
            (
                FunderSearchMatch::new(
                    Funder::new(
                        crate::domain::models::funder::FunderId::new(uuid::Uuid::nil()),
                        "canna ltd".into(),
                        FunderKind::Company,
                    ),
                    SearchMatchSource::Name,
                ),
                SearchSimilarity::from(2_f32),
            ),
        ];
        let mps = vec![(
            ParliamentMember::new(
                "johnny larner".into(),
                MemberId::new(1).unwrap(),
                String::new(),
                1,
                String::new(),
            ),
            SearchSimilarity::from(4_f32),
        )];

        let ranked = merge_scores(&mps, &funders);
        assert_eq!(ranked.len(), 3);
        assert_eq!(
            ranked.first().unwrap(),
            &Entity::from(&ParliamentMember::new(
                "johnny larner".into(),
                MemberId::new(1).unwrap(),
                String::new(),
                1,
                String::new(),
            ))
        );
        assert_eq!(
            ranked.get(1).unwrap(),
            &Entity::from(&FunderSearchMatch::new(
                Funder::new(
                    crate::domain::models::funder::FunderId::new(uuid::Uuid::nil()),
                    "heavenly ltd".into(),
                    FunderKind::Company
                ),
                SearchMatchSource::Name
            ),)
        );
        assert_eq!(
            ranked.get(2).unwrap(),
            &Entity::from(&FunderSearchMatch::new(
                Funder::new(
                    crate::domain::models::funder::FunderId::new(uuid::Uuid::nil()),
                    "canna ltd".into(),
                    FunderKind::Company
                ),
                SearchMatchSource::Name
            ),)
        );
    }

    #[test]
    fn works_for_more_mps() {
        let funders = vec![(
            FunderSearchMatch::new(
                Funder::new(
                    crate::domain::models::funder::FunderId::new(uuid::Uuid::nil()),
                    "canna ltd".into(),
                    FunderKind::Company,
                ),
                SearchMatchSource::Name,
            ),
            SearchSimilarity::from(2_f32),
        )];
        let mps = vec![
            (
                ParliamentMember::new(
                    "johnny larner".into(),
                    MemberId::new(1).unwrap(),
                    String::new(),
                    1,
                    String::new(),
                ),
                SearchSimilarity::from(4_f32),
            ),
            (
                ParliamentMember::new(
                    "hades".into(),
                    MemberId::new(1).unwrap(),
                    String::new(),
                    1,
                    String::new(),
                ),
                SearchSimilarity::from(2_f32),
            ),
        ];

        let ranked = merge_scores(&mps, &funders);
        assert_eq!(
            ranked.first().unwrap(),
            &Entity::from(&ParliamentMember::new(
                "johnny larner".into(),
                MemberId::new(1).unwrap(),
                String::new(),
                1,
                String::new(),
            ))
        );
        assert_eq!(
            ranked.get(1).unwrap(),
            &Entity::from(&ParliamentMember::new(
                "hades".into(),
                MemberId::new(1).unwrap(),
                String::new(),
                1,
                String::new(),
            ))
        );
        assert_eq!(
            ranked.get(2).unwrap(),
            &Entity::from(&FunderSearchMatch::new(
                Funder::new(
                    crate::domain::models::funder::FunderId::new(uuid::Uuid::nil()),
                    "canna ltd".into(),
                    FunderKind::Company,
                ),
                SearchMatchSource::Name
            ))
        );
    }
}

#[cfg(test)]
mod provenance {
    use super::{FunderSearchMatch, ParliamentMember, SearchSimilarity, merge_scores};
    use crate::domain::models::parliament_member::MemberId;
    use crate::domain::models::{
        entity_search::{Entity, SearchMatchSource},
        funder::{Funder, FunderId, FunderKind},
    };

    #[test]
    fn retains_alias_source_when_interleaving_equal_scores() {
        let source = SearchMatchSource::Alias {
            name: "West Midlands".into(),
        };
        let funder = FunderSearchMatch::new(
            Funder::new(
                FunderId::new(uuid::Uuid::nil()),
                "Unite".into(),
                FunderKind::TradeUnion,
            ),
            source.clone(),
        );
        let mp = ParliamentMember::new(
            "West Member".into(),
            MemberId::new(1).unwrap(),
            "Party".into(),
            1,
            "Place".into(),
        );
        let results = merge_scores(
            &[(mp.clone(), SearchSimilarity::from(1.0))],
            &[(funder, SearchSimilarity::from(1.0))],
        );
        assert_eq!(results[0], Entity::ParliamentMember(mp));
        let Entity::Funder(result) = &results[1] else {
            panic!("expected funder");
        };
        assert_eq!(result.source(), &source);
    }
}
