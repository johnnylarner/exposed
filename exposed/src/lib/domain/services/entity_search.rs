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

        let mut results = merge_scores(&mps, &funders.matches);
        if mps.is_empty() && !funders.has_qualifying_individual {
            let fallback_mps = self.mp_repo.get_members_by_phonetic_search(req).await?;
            let fallback_funders = self.funder_repo.get_funders_by_phonetic_search(req).await?;
            let remaining = usize::from(req.max_entries()).saturating_sub(funders.matches.len());
            let mut fallback: Vec<_> = fallback_mps
                .into_iter()
                .map(|(mp, rank)| (Entity::ParliamentMember(mp), rank))
                .chain(
                    fallback_funders
                        .into_iter()
                        .filter(|(funder, _)| {
                            !funders
                                .matches
                                .iter()
                                .any(|(primary, _)| primary.funder().id() == funder.funder().id())
                        })
                        .take(remaining)
                        .map(|(funder, rank)| (Entity::Funder(funder), rank)),
                )
                .collect();
            fallback.sort_by(|(left, a), (right, b)| {
                a.compare(b).then_with(|| match (left, right) {
                    (Entity::ParliamentMember(a), Entity::ParliamentMember(b)) => {
                        a.member_id().value().cmp(&b.member_id().value())
                    }
                    (Entity::Funder(a), Entity::Funder(b)) => {
                        a.funder().id().value().cmp(&b.funder().id().value())
                    }
                    (Entity::ParliamentMember(_), Entity::Funder(_)) => std::cmp::Ordering::Less,
                    (Entity::Funder(_), Entity::ParliamentMember(_)) => std::cmp::Ordering::Greater,
                })
            });
            results.extend(fallback.into_iter().map(|(entity, _)| entity));
        }
        Ok(results)
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

#[cfg(test)]
mod phonetic_regression {
    use super::*;
    use crate::outbound::ExposedDatabase;
    use sqlx::PgPool;

    #[sqlx::test(
        migrations = "../db/migrations",
        fixtures("../../../../../db/fixtures/add_members.sql")
    )]
    async fn jahn_finds_john_after_primary_organization(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!("INSERT INTO exposed.funders (funder_name, funder_kind) VALUES ('Jahn Foundation', 'Company')").execute(&pool).await?;
        let db = ExposedDatabase::from(pool);
        let results = Service::new(db.clone(), db)
            .search_entities(
                &EntitySearchRequest::new_with_strictness("jahn".into(), 10, 0.8).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            results.iter().map(Entity::name).collect::<Vec<_>>(),
            vec!["Jahn Foundation", "John McDonnell", "John Humphries"]
        );
        Ok(())
    }
}

#[cfg(test)]
mod fallback_contract {
    use super::*;
    use crate::domain::models::entity_search::SearchMatchSource;
    use crate::outbound::ExposedDatabase;
    use sqlx::PgPool;

    fn request(term: &str, limit: u8) -> EntitySearchRequest {
        EntitySearchRequest::new_with_strictness(term.into(), limit, 0.8).unwrap()
    }

    #[sqlx::test(
        migrations = "../db/migrations",
        fixtures("../../../../../db/fixtures/add_members.sql")
    )]
    async fn primary_people_suppress_fallback_before_limits(pool: PgPool) -> sqlx::Result<()> {
        let db = ExposedDatabase::from(pool.clone());
        let service = Service::new(db.clone(), db);
        let company = uuid::Uuid::from_u128(1);
        let person = uuid::Uuid::from_u128(2);
        sqlx::query!("INSERT INTO exposed.funders (id, funder_name, funder_kind) VALUES ($1, 'Jahn', 'Company'), ($2, 'Jahn Person', 'Individual')", company, person).execute(&pool).await?;
        let results = service.search_entities(&request("jahn", 1)).await.unwrap();
        assert_eq!(
            results.iter().map(Entity::name).collect::<Vec<_>>(),
            vec!["Jahn"]
        );
        let results = service.search_entities(&request("jahn", 10)).await.unwrap();
        assert_eq!(
            results.iter().map(Entity::name).collect::<Vec<_>>(),
            vec!["Jahn", "Jahn Person"]
        );
        sqlx::query!("INSERT INTO exposed.funders (funder_name, funder_kind) VALUES ('Jahn Fallback', 'Individual')").execute(&pool).await?;
        let results = service.search_entities(&request("john", 10)).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(
            results
                .iter()
                .all(|entity| matches!(entity, Entity::ParliamentMember(_)))
        );
        Ok(())
    }

    #[sqlx::test(migrations = "../db/migrations")]
    async fn aliases_deduplicate_and_share_the_funder_cap(pool: PgPool) -> sqlx::Result<()> {
        let person = uuid::Uuid::from_u128(1);
        sqlx::query!("INSERT INTO exposed.funders (id, funder_name, funder_kind) VALUES ($1, 'Unrelated Identity', 'Individual')", person).execute(&pool).await?;
        for alias in ["John", "JOHN", "John Smith"] {
            sqlx::query!(
                "INSERT INTO exposed.funder_aliases (funder_id, funder_alias) VALUES ($1, $2)",
                person,
                alias
            )
            .execute(&pool)
            .await?;
        }
        sqlx::query!("INSERT INTO exposed.funders (funder_name, funder_kind) VALUES ('Jahn Foundation', 'Company'), ('John Brown', 'Individual'), ('John Company', 'Company')").execute(&pool).await?;
        let db = ExposedDatabase::from(pool);
        let service = Service::new(db.clone(), db);
        let results = service.search_entities(&request("jahn", 2)).await.unwrap();
        assert_eq!(
            results.iter().map(Entity::name).collect::<Vec<_>>(),
            vec!["Jahn Foundation", "Unrelated Identity"]
        );
        let Entity::Funder(found) = &results[1] else {
            panic!("expected funder");
        };
        assert_eq!(found.funder().id().value(), person);
        assert_eq!(
            found.source(),
            &SearchMatchSource::Alias {
                name: "JOHN".into()
            }
        );
        Ok(())
    }

    #[sqlx::test(migrations = "../db/migrations")]
    async fn corrects_each_candidate_with_bounded_word_edits(pool: PgPool) -> sqlx::Result<()> {
        let db = ExposedDatabase::from(pool.clone());
        for (query, candidate, threshold, expected) in [
            ("jahn smyth", "John Smith", 0.8, true),
            ("jahn unrelated", "John Smith", 0.8, false),
            ("jahn jahn", "John", 0.0, true),
            ("jahn", "Sir John Smith", 0.8, true),
            ("jahn", "Jonathan", 0.0, false),
            ("jahn", "Jane", 0.0, false),
            ("!!!", "John", 0.0, false),
            ("jahn, smyth!", "John Smith", 0.8, true),
            ("jahn smyth", "John Extra Smith", 0.8, false),
            ("jahn smyth", "John Extra Smith", 0.0, true),
        ] {
            sqlx::query!("DELETE FROM exposed.funders")
                .execute(&pool)
                .await?;
            sqlx::query!(
                "INSERT INTO exposed.funders (funder_name, funder_kind) VALUES ($1, 'Individual')",
                candidate
            )
            .execute(&pool)
            .await?;
            sqlx::query!("DELETE FROM exposed.members")
                .execute(&pool)
                .await?;
            sqlx::query!("INSERT INTO exposed.members (parliament_member_id, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons) VALUES (1, $1, 1, 'Party', 1, 'Constituency', true)", candidate).execute(&pool).await?;
            let req =
                EntitySearchRequest::new_with_strictness(query.into(), 10, threshold).unwrap();
            let members = db.get_members_by_phonetic_search(&req).await.unwrap();
            assert_eq!(
                !members.is_empty(),
                expected,
                "member: {query} / {candidate} / {threshold}"
            );
            let rows = db.get_funders_by_phonetic_search(&req).await.unwrap();
            assert_eq!(
                !rows.is_empty(),
                expected,
                "{query} / {candidate} / {threshold}"
            );
        }
        let long = "a".repeat(256);
        for (query, candidate) in [(long.clone(), "aaa".into()), ("aaa".into(), long)] {
            sqlx::query!("DELETE FROM exposed.funders")
                .execute(&pool)
                .await?;
            sqlx::query!(
                "INSERT INTO exposed.funders (funder_name, funder_kind) VALUES ($1, 'Individual')",
                candidate
            )
            .execute(&pool)
            .await?;
            sqlx::query!("DELETE FROM exposed.members")
                .execute(&pool)
                .await?;
            sqlx::query!("INSERT INTO exposed.members (parliament_member_id, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons) VALUES (1, $1, 1, 'Party', 1, 'Constituency', true)", candidate).execute(&pool).await?;
            let req = EntitySearchRequest::new_with_strictness(query, 10, 0.0).unwrap();
            assert!(
                db.get_members_by_phonetic_search(&req)
                    .await
                    .unwrap()
                    .is_empty()
            );
            assert!(
                db.get_funders_by_phonetic_search(&req)
                    .await
                    .unwrap()
                    .is_empty()
            );
        }
        Ok(())
    }

    #[sqlx::test(migrations = "../db/migrations")]
    async fn corrections_do_not_depend_on_other_candidates(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!("INSERT INTO exposed.funders (funder_name, funder_kind) VALUES ('John Smith', 'Individual')").execute(&pool).await?;
        let db = ExposedDatabase::from(pool.clone());
        let before = db
            .get_funders_by_phonetic_search(&request("jahn smyth", 10))
            .await
            .unwrap();
        assert_eq!(before.len(), 1);
        sqlx::query!("INSERT INTO exposed.funders (funder_name, funder_kind) VALUES ('Joan Smith', 'Individual')").execute(&pool).await?;
        let after = db
            .get_funders_by_phonetic_search(&request("jahn smyth", 10))
            .await
            .unwrap();
        let existing = after
            .iter()
            .find(|(found, _)| found.funder().id() == before[0].0.funder().id())
            .unwrap();
        assert_eq!(existing.1.compare(&before[0].1), std::cmp::Ordering::Equal);
        Ok(())
    }

    #[sqlx::test(migrations = "../db/migrations")]
    async fn filters_corrected_similarity_then_ranks_by_distance_before_limit(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        sqlx::query!("INSERT INTO exposed.funders (funder_name, funder_kind) VALUES ('John Smith', 'Individual'), ('Jahn Smith Extra Words', 'Individual'), ('Jahn Unrelated Middle Smyth', 'Individual')").execute(&pool).await?;
        let db = ExposedDatabase::from(pool);
        let rows = db
            .get_funders_by_phonetic_search(&request("jahn smyth", 1))
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0.funder().name(), "Jahn Smith Extra Words");
        Ok(())
    }
}
