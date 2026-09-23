//! Entity search refers to the landing page of the `exposed` application. Its purpose it to make it
//! easy for journalists to find [`parliament_members`] or [`funders`].
//!

mod error;
mod interface;

use crate::domain::models::entity_search::{Entity, EntitySearchError, EntitySearchRequest};
use crate::domain::models::funder::Funder;
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
    pub fn new(mp_repo: P, funder_repo: F) -> Self {
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
        let mps = self.mp_repo.get_members_by_text_search_score(&req).await?;
        let funders = self
            .funder_repo
            .get_funders_by_text_search_score(&req)
            .await?;

        let scored = merge_scores(&mps, &funders);
        Ok(scored)
    }
}

/// Uses two-pointer technique to merge to pre-sorted
/// search results together.
fn merge_scores(
    mps: &[(ParliamentMember, SearchSimilarity)],
    funders: &[(Funder, SearchSimilarity)],
) -> Vec<Entity> {
    let mut scores = Vec::new();

    let (mut l, mut r) = (0, 0);
    // Unwrap acceptable as index check in
    // while function
    while l < mps.len() && r < funders.len() {
        let mp: &(ParliamentMember, SearchSimilarity) = mps.get(l).unwrap();
        let fund: &(Funder, SearchSimilarity) = funders.get(r).unwrap();

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
        };
    }
    if l < mps.len() {
        for mp in mps[l..].iter() {
            scores.push(Entity::from(&mp.0));
        }
    }
    if r < funders.len() {
        for fund in funders[r..].iter() {
            scores.push(Entity::from(&fund.0));
        }
    }

    scores
}

#[cfg(test)]
mod merge_scores {
    use crate::domain::{
        models::{
            entity_search::Entity,
            funder::{Funder, FunderKind},
            parliament_member::ParliamentMember,
            search_similarity::SearchSimilarity,
        },
        services::entity_search::merge_scores,
    };

    #[test]
    fn works_for_only_mps() {
        let funders = Vec::new();
        let mps = vec![
            (
                ParliamentMember::new("johnny larner".into(), 1, "".into(), "".into()),
                SearchSimilarity::from(1_f32),
            ),
            (
                ParliamentMember::new("hades".into(), 1, "".into(), "".into()),
                SearchSimilarity::from(2_f32),
            ),
        ];

        let ranked = merge_scores(&mps, &funders);
        assert_eq!(
            ranked.get(0).unwrap(),
            &Entity::from(&ParliamentMember::new(
                "johnny larner".into(),
                1,
                "".into(),
                "".into(),
            ))
        );

        assert_eq!(
            ranked.get(1).unwrap(),
            &Entity::from(&ParliamentMember::new(
                "hades".into(),
                1,
                "".into(),
                "".into(),
            ))
        );
    }

    #[test]
    fn works_for_balanced_results() {
        let funders = vec![
            (
                Funder::new("heavenly ltd".into(), FunderKind::Company),
                SearchSimilarity::from(4_f32),
            ),
            (
                Funder::new("canna ltd".into(), FunderKind::Company),
                SearchSimilarity::from(2_f32),
            ),
        ];
        let mps = vec![
            (
                ParliamentMember::new("johnny larner".into(), 1, "".into(), "".into()),
                SearchSimilarity::from(4_f32),
            ),
            (
                ParliamentMember::new("hades".into(), 1, "".into(), "".into()),
                SearchSimilarity::from(3_f32),
            ),
        ];

        let ranked = merge_scores(&mps, &funders);
        assert_eq!(ranked.len(), 4);
        assert_eq!(
            ranked.get(0).unwrap(),
            &Entity::from(&ParliamentMember::new(
                "johnny larner".into(),
                1,
                "".into(),
                "".into(),
            ))
        );
        assert_eq!(
            ranked.get(1).unwrap(),
            &Entity::from(&Funder::new("heavenly ltd".into(), FunderKind::Company),)
        );
        assert_eq!(
            ranked.get(2).unwrap(),
            &Entity::from(&ParliamentMember::new(
                "hades".into(),
                1,
                "".into(),
                "".into(),
            ))
        );
        assert_eq!(
            ranked.get(3).unwrap(),
            &Entity::from(&Funder::new("canna ltd".into(), FunderKind::Company),)
        );
    }

    #[test]
    fn works_for_more_funders() {
        let funders = vec![
            (
                Funder::new("heavenly ltd".into(), FunderKind::Company),
                SearchSimilarity::from(4_f32),
            ),
            (
                Funder::new("canna ltd".into(), FunderKind::Company),
                SearchSimilarity::from(2_f32),
            ),
        ];
        let mps = vec![(
            ParliamentMember::new("johnny larner".into(), 1, "".into(), "".into()),
            SearchSimilarity::from(4_f32),
        )];

        let ranked = merge_scores(&mps, &funders);
        assert_eq!(ranked.len(), 3);
        assert_eq!(
            ranked.get(0).unwrap(),
            &Entity::from(&ParliamentMember::new(
                "johnny larner".into(),
                1,
                "".into(),
                "".into(),
            ))
        );
        assert_eq!(
            ranked.get(1).unwrap(),
            &Entity::from(&Funder::new("heavenly ltd".into(), FunderKind::Company),)
        );
        assert_eq!(
            ranked.get(2).unwrap(),
            &Entity::from(&Funder::new("canna ltd".into(), FunderKind::Company),)
        );
    }

    #[test]
    fn works_for_more_mps() {
        let funders = vec![(
            Funder::new("canna ltd".into(), FunderKind::Company),
            SearchSimilarity::from(2_f32),
        )];
        let mps = vec![
            (
                ParliamentMember::new("johnny larner".into(), 1, "".into(), "".into()),
                SearchSimilarity::from(4_f32),
            ),
            (
                ParliamentMember::new("hades".into(), 1, "".into(), "".into()),
                SearchSimilarity::from(2_f32),
            ),
        ];

        let ranked = merge_scores(&mps, &funders);
        assert_eq!(
            ranked.get(0).unwrap(),
            &Entity::from(&ParliamentMember::new(
                "johnny larner".into(),
                1,
                "".into(),
                "".into(),
            ))
        );
        assert_eq!(
            ranked.get(1).unwrap(),
            &Entity::from(&ParliamentMember::new(
                "hades".into(),
                1,
                "".into(),
                "".into(),
            ))
        );
        assert_eq!(
            ranked.get(2).unwrap(),
            &Entity::from(&Funder::new("canna ltd".into(), FunderKind::Company,))
        );
    }
}
