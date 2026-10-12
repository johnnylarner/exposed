//! Concrete implementation of the repository

use std::str::FromStr;

use crate::{
    domain::{
        models::{
            entity_search::{
                EntitySearchRequest, FunderSearchMatch, FunderSearchResults, PhoneticRank,
                SearchMatchSource,
            },
            funder::{Funder, FunderId, FunderKind},
            search_similarity::SearchSimilarity,
        },
        repositories::funder_repository::{FunderRepo, FunderRepoError},
    },
    outbound::postgres::{ExposedDatabase, word_similarity_candidate_threshold},
};

impl FunderRepo for ExposedDatabase {
    async fn get_funders_by_text_search_score(
        &self,
        req: &EntitySearchRequest,
    ) -> Result<FunderSearchResults, FunderRepoError> {
        let mut tx = self
            .pool()
            .begin()
            .await
            .map_err(|e| FunderRepoError::DatabaseError(e.to_string()))?;
        sqlx::query!(
            "SELECT set_config('pg_trgm.word_similarity_threshold', $1, true)",
            word_similarity_candidate_threshold(req.strictness())
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| FunderRepoError::DatabaseError(e.to_string()))?;
        let rows = sqlx::query!(
            "
            WITH candidates AS (
                SELECT id FROM exposed.funders WHERE funder_name %> $1
                UNION
                SELECT funder_id FROM exposed.funder_aliases WHERE funder_alias %> $1
            ), scored AS (
                SELECT f.id, f.funder_name AS name, f.funder_kind AS kind,
                    word_similarity($1, f.funder_name) AS canonical_score,
                    a.funder_alias, a.alias_score
                FROM candidates c
                JOIN exposed.funders f ON f.id = c.id
                LEFT JOIN LATERAL (
                    SELECT funder_alias, word_similarity($1, funder_alias) AS alias_score
                    FROM exposed.funder_aliases WHERE funder_id = f.id
                    ORDER BY alias_score DESC, funder_alias COLLATE \"C\" ASC
                    LIMIT 1
                ) a ON true
            ), qualified AS (
                SELECT id, name, kind,
                    CASE WHEN alias_score > canonical_score THEN funder_alias END AS matched_alias,
                    GREATEST(canonical_score, COALESCE(alias_score, 0)) AS similarity_score
                FROM scored
                WHERE GREATEST(canonical_score, COALESCE(alias_score, 0)) >= $2
            ), grouped AS (
                SELECT DISTINCT ON (COALESCE(g.group_id, q.id)) COALESCE(g.group_id, q.id) AS id,
                    representative.funder_name AS name, representative.funder_kind AS kind,
                    CASE WHEN q.id <> COALESCE(g.group_id, q.id) THEN COALESCE(q.matched_alias, q.name)
                        ELSE q.matched_alias END AS matched_alias, q.similarity_score
                FROM qualified q LEFT JOIN exposed.funder_alias_groups g ON g.funder_id = q.id
                JOIN exposed.funders representative ON representative.id = COALESCE(g.group_id, q.id)
                ORDER BY COALESCE(g.group_id, q.id), q.similarity_score DESC, q.id
            )
            SELECT id AS \"id!\", name, kind, matched_alias, similarity_score, EXISTS(SELECT FROM qualified WHERE kind = $4) AS \"has_individual!\"
            FROM grouped
            ORDER BY similarity_score DESC, id
            LIMIT $3
            ",
            req.term(),
            req.strictness(),
            i64::from(req.max_entries()),
            FunderKind::Individual.as_ref(),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| FunderRepoError::DatabaseError(e.to_string()))?;
        tx.commit()
            .await
            .map_err(|e| FunderRepoError::DatabaseError(e.to_string()))?;
        let has_qualifying_individual = rows.first().is_some_and(|row| row.has_individual);
        let matches = rows
            .into_iter()
            .map(|r| {
                let kind = r.kind.map_or(FunderKind::NotSpecified, |ref kind| {
                    FunderKind::from_str(kind).unwrap()
                });
                let funder = Funder::new(FunderId::new(r.id), r.name, kind);
                let score = SearchSimilarity::from(r.similarity_score.unwrap_or(0_f32));
                Ok((
                    FunderSearchMatch::new(
                        funder,
                        r.matched_alias.map_or(SearchMatchSource::Name, |name| {
                            SearchMatchSource::Alias { name }
                        }),
                    ),
                    score,
                ))
            })
            .collect::<Result<Vec<_>, FunderRepoError>>()?;
        Ok(FunderSearchResults {
            matches,
            has_qualifying_individual,
        })
    }
    async fn get_funders_by_phonetic_search(
        &self,
        req: &EntitySearchRequest,
    ) -> Result<Vec<(FunderSearchMatch, PhoneticRank)>, FunderRepoError> {
        let rows = sqlx::query!(r#"
            WITH names AS (
                SELECT id, funder_name AS candidate, NULL::text AS alias
                FROM exposed.funders
                WHERE funder_kind = $2 AND exposed.person_name_codes(funder_name) @> exposed.person_name_codes($1)
                UNION ALL
                SELECT f.id, a.funder_alias, a.funder_alias
                FROM exposed.funder_aliases a JOIN exposed.funders f ON f.id = a.funder_id
                WHERE f.funder_kind = $2 AND exposed.person_name_codes(a.funder_alias) @> exposed.person_name_codes($1)
            ), scored AS (
                SELECT names.*, correction.distance,
                       word_similarity(correction.corrected_query, candidate) AS similarity_score
                FROM names
                CROSS JOIN LATERAL (
                    SELECT string_agg(best.word, ' ' ORDER BY q.position) AS corrected_query,
                           sum(best.distance)::integer AS distance
                    FROM unnest(exposed.person_name_words($1)) WITH ORDINALITY AS q(word, position)
                    LEFT JOIN LATERAL (
                        SELECT cw.word, CASE
                            WHEN char_length(q.word) <= 255 AND char_length(cw.word) <= 255
                            THEN exposed.levenshtein_less_equal(q.word, cw.word, 1)
                            ELSE 2
                        END AS distance
                        FROM unnest(exposed.person_name_words(candidate)) AS cw(word)
                        WHERE exposed.soundex(cw.word) = exposed.soundex(q.word)
                        ORDER BY distance, cw.word COLLATE "C"
                        LIMIT 1
                    ) best ON best.distance <= 1
                    HAVING count(best.word) = count(*) AND count(*) > 0
                ) correction
            ), winners AS (
                SELECT DISTINCT ON (COALESCE(g.group_id, s.id)) COALESCE(g.group_id, s.id) AS id,
                    CASE WHEN s.id <> COALESCE(g.group_id, s.id) THEN COALESCE(s.alias, s.candidate)
                        ELSE s.alias END AS alias, s.distance, s.similarity_score
                FROM scored s LEFT JOIN exposed.funder_alias_groups g ON g.funder_id = s.id
                WHERE s.similarity_score >= $4
                ORDER BY COALESCE(g.group_id, s.id), s.distance, s.similarity_score DESC, s.id,
                    s.alias IS NOT NULL, s.alias COLLATE "C"
            )
            SELECT f.id, f.funder_name AS name, f.funder_kind AS kind, w.alias, w.distance AS "distance!",
                   w.similarity_score AS "similarity_score!"
            FROM winners w JOIN exposed.funders f ON f.id = w.id
            ORDER BY w.distance, w.similarity_score DESC, f.id
            LIMIT $3
        "#, req.term(), FunderKind::Individual.as_ref(), i64::from(req.max_entries()), req.strictness())
        .fetch_all(self.pool()).await.map_err(|e| FunderRepoError::DatabaseError(e.to_string()))?;
        rows.into_iter()
            .map(|row| {
                let rank = PhoneticRank::new(
                    u32::try_from(row.distance)
                        .map_err(|e| FunderRepoError::DatabaseError(e.to_string()))?,
                    SearchSimilarity::from(row.similarity_score),
                );
                Ok((
                    FunderSearchMatch::new(
                        Funder::new(
                            FunderId::new(row.id),
                            row.name,
                            row.kind
                                .as_deref()
                                .map_or(Ok(FunderKind::NotSpecified), FunderKind::from_str)
                                .map_err(|error| {
                                    FunderRepoError::DatabaseError(error.to_string())
                                })?,
                        ),
                        row.alias.map_or(SearchMatchSource::Name, |name| {
                            SearchMatchSource::Alias { name }
                        }),
                    ),
                    rank,
                ))
            })
            .collect()
    }
}

#[cfg(test)]
mod scoring {
    use sqlx::PgPool;

    use crate::{
        domain::{
            models::entity_search::EntitySearchRequest, repositories::funder_repository::FunderRepo,
        },
        outbound::postgres::ExposedDatabase,
    };

    #[sqlx::test(
        migrations = "../db/migrations",
        fixtures(
            "../../../../db/fixtures/add_members.sql",
            "../../../../db/fixtures/add_declarations_and_funding_entries.sql"
        )
    )]
    async fn orders_correctly(pool: PgPool) -> sqlx::Result<()> {
        let db = ExposedDatabase::from(pool);

        let term = EntitySearchRequest::new_with_strictness("McDonald's".into(), 5, 0_f32).unwrap();
        let results = db
            .get_funders_by_text_search_score(&term)
            .await
            .unwrap()
            .matches;

        assert_eq!(results.len(), 2);
        assert_eq!(results.first().unwrap().0.funder().name(), "McDonald's");

        Ok(())
    }

    #[sqlx::test(
        migrations = "../db/migrations",
        fixtures(
            "../../../../db/fixtures/add_members.sql",
            "../../../../db/fixtures/add_declarations_and_funding_entries.sql"
        )
    )]
    async fn obeys_max_entries(pool: PgPool) -> sqlx::Result<()> {
        let db = ExposedDatabase::from(pool);

        let term = EntitySearchRequest::new_with_strictness("McDonald's".into(), 1, 0_f32).unwrap();
        let results = db
            .get_funders_by_text_search_score(&term)
            .await
            .unwrap()
            .matches;

        assert_eq!(results.len(), 1);
        assert_eq!(results.first().unwrap().0.funder().name(), "McDonald's");

        Ok(())
    }
    #[sqlx::test(
        migrations = "../db/migrations",
        fixtures(
            "../../../../db/fixtures/add_members.sql",
            "../../../../db/fixtures/add_declarations_and_funding_entries.sql"
        )
    )]
    async fn filters_similarity(pool: PgPool) -> sqlx::Result<()> {
        let db = ExposedDatabase::from(pool);

        let term =
            EntitySearchRequest::new_with_strictness("Aaron Banks".into(), 5, 1_f32).unwrap();
        let results = db
            .get_funders_by_text_search_score(&term)
            .await
            .unwrap()
            .matches;

        assert_eq!(results.len(), 1);
        assert_eq!(results.first().unwrap().0.funder().name(), "Aaron Banks");

        Ok(())
    }
}

#[cfg(test)]
mod alias_scoring {
    use crate::{
        domain::{
            models::entity_search::{EntitySearchRequest, SearchMatchSource},
            repositories::funder_repository::FunderRepo,
        },
        outbound::postgres::ExposedDatabase,
    };
    use sqlx::PgPool;

    #[sqlx::test(migrations = "../db/migrations")]
    async fn provenance_thresholds_and_identity_limit(pool: PgPool) -> sqlx::Result<()> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_with(pool.connect_options().as_ref().clone())
            .await?;
        let first = uuid::Uuid::from_u128(1);
        let second = uuid::Uuid::from_u128(2);
        sqlx::query!("INSERT INTO exposed.funders (id, funder_name) VALUES ($1, 'Unite the Union'), ($2, 'West Midlands')", first, second).execute(&pool).await?;
        for alias in [
            "West Midlands",
            "WEST MIDLANDS",
            "West Midlands Union",
            "West Midlands Branch",
        ] {
            sqlx::query!(
                "INSERT INTO exposed.funder_aliases (funder_id, funder_alias) VALUES ($1, $2)",
                first,
                alias
            )
            .execute(&pool)
            .await?;
        }
        sqlx::query!("INSERT INTO exposed.funder_aliases (funder_id, funder_alias) VALUES ($1, 'West Midlands')", second).execute(&pool).await?;
        let before = sqlx::query!(
            "SELECT current_setting('pg_trgm.word_similarity_threshold') AS threshold"
        )
        .fetch_one(&pool)
        .await?
        .threshold;
        let db = ExposedDatabase::from(pool.clone());
        db.refresh_funder_groups().await.unwrap();
        let request = |term: &str, limit, threshold| {
            EntitySearchRequest::new_with_strictness(term.into(), limit, threshold).unwrap()
        };
        let rows = db
            .get_funders_by_text_search_score(&request("West Midlands", 2, 1.0))
            .await
            .unwrap()
            .matches;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0.funder().id().value(), first);
        assert_eq!(
            rows[0].0.source(),
            &SearchMatchSource::Alias {
                name: "WEST MIDLANDS".into()
            }
        );
        assert_eq!(rows[0].1.value().to_bits(), 1.0_f32.to_bits());
        let rows = db
            .get_funders_by_text_search_score(&request("Unite the Union", 2, 1.0))
            .await
            .unwrap()
            .matches;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0.source(), &SearchMatchSource::Name);
        assert_eq!(
            db.get_funders_by_text_search_score(&request("zzzzzz", 2, 0.0))
                .await
                .unwrap()
                .matches
                .len(),
            1
        );
        assert_eq!(
            db.get_funders_by_text_search_score(&request("West Midlands", 1, 1.0))
                .await
                .unwrap()
                .matches
                .len(),
            1
        );
        let boundary = sqlx::query!("SELECT word_similarity('West Mid', 'West Midlands') AS score")
            .fetch_one(&pool)
            .await?
            .score
            .unwrap();
        assert!(boundary > 0.0 && boundary < 1.0);
        assert_eq!(
            db.get_funders_by_text_search_score(&request("West Mid", 2, boundary))
                .await
                .unwrap()
                .matches
                .len(),
            1
        );
        let after = sqlx::query!(
            "SELECT current_setting('pg_trgm.word_similarity_threshold') AS threshold"
        )
        .fetch_one(&pool)
        .await?
        .threshold;
        assert_eq!(before, after);
        Ok(())
    }
}

#[cfg(test)]
mod alias_groups {
    use crate::{
        domain::{
            models::entity_search::EntitySearchRequest, repositories::funder_repository::FunderRepo,
        },
        outbound::postgres::ExposedDatabase,
    };
    use sqlx::PgPool;

    #[sqlx::test(migrations = "../db/migrations")]
    async fn long_source_aliases_refresh_without_truncating_evidence(
        pool: PgPool,
    ) -> anyhow::Result<()> {
        use sha2::{Digest, Sha256};
        use std::fmt::Write;

        let ids = [uuid::Uuid::from_u128(1), uuid::Uuid::from_u128(2)];
        let mut alias = String::new();
        for index in 0_u32..20 {
            write!(alias, "{:x}", Sha256::digest(index.to_be_bytes()))?;
        }
        assert_eq!(alias.len(), 1280);
        for (id, name) in ids.iter().zip(["First source", "Second source"]) {
            sqlx::query!(
                "INSERT INTO exposed.funders (id, funder_name) VALUES ($1, $2)",
                id,
                name
            )
            .execute(&pool)
            .await?;
            sqlx::query!(
                "INSERT INTO exposed.funder_aliases (funder_id, funder_alias) VALUES ($1, $2)",
                id,
                alias
            )
            .execute(&pool)
            .await?;
        }
        let db = ExposedDatabase::from(pool.clone());
        assert_eq!(db.refresh_funder_groups().await?, 2);
        let links = sqlx::query!("SELECT left_funder_id, right_funder_id, normalized_alias, left_alias, right_alias FROM exposed.funder_alias_links").fetch_all(&pool).await?;
        assert_eq!(links.len(), 1);
        assert_eq!([links[0].left_funder_id, links[0].right_funder_id], ids);
        assert_eq!(links[0].normalized_alias, alias);
        assert_eq!(links[0].left_alias, alias);
        assert_eq!(links[0].right_alias, alias);
        let memberships = sqlx::query!(
            "SELECT funder_id, group_id FROM exposed.funder_alias_groups ORDER BY funder_id"
        )
        .fetch_all(&pool)
        .await?;
        assert_eq!(memberships.len(), 2);
        assert!(memberships.iter().all(|row| row.group_id == ids[0]));
        Ok(())
    }

    #[sqlx::test(migrations = "../db/migrations")]
    async fn transitive_groups_cycles_limits_and_removal(pool: PgPool) -> sqlx::Result<()> {
        let ids: Vec<_> = (1..=4).map(uuid::Uuid::from_u128).collect();
        for (id, name) in ids.iter().zip(["Alpha", "Beta", "Gamma", "Alpha separate"]) {
            sqlx::query!(
                "INSERT INTO exposed.funders (id, funder_name) VALUES ($1, $2)",
                id,
                name
            )
            .execute(&pool)
            .await?;
        }
        for (id, alias) in [
            (ids[0], "Bridge one"),
            (ids[1], " BRIDGE\tONE "),
            (ids[1], "Bridge two"),
            (ids[2], "Bridge two"),
            (ids[2], "ALPHA"),
        ] {
            sqlx::query!(
                "INSERT INTO exposed.funder_aliases (funder_id, funder_alias) VALUES ($1, $2)",
                id,
                alias
            )
            .execute(&pool)
            .await?;
        }
        let db = ExposedDatabase::from(pool.clone());
        db.refresh_funder_groups().await.unwrap();
        let request = |term: &str, limit| {
            EntitySearchRequest::new_with_strictness(term.into(), limit, 1.0).unwrap()
        };
        let groups = sqlx::query!("SELECT funder_id AS \"id!\", group_id AS \"group_id!\" FROM exposed.funder_alias_groups ORDER BY funder_id")
            .fetch_all(&pool).await?;
        assert_eq!(groups.len(), 4);
        assert!(groups[..3].iter().all(|row| row.group_id == ids[0]));
        assert_eq!(groups[3].group_id, ids[3]);
        let matched = db
            .get_funders_by_text_search_score(&request("Gamma", 2))
            .await
            .unwrap()
            .matches;
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].0.funder().id().value(), ids[0]);
        assert_eq!(matched[0].0.funder().name(), "Alpha");
        assert_eq!(
            matched[0].0.source(),
            &crate::domain::models::entity_search::SearchMatchSource::Alias {
                name: "Gamma".into()
            }
        );
        let limited = db
            .get_funders_by_text_search_score(&request("Alpha", 2))
            .await
            .unwrap()
            .matches;
        assert_eq!(limited.len(), 2);
        assert_eq!(limited[1].0.funder().id().value(), ids[3]);
        sqlx::query!(
            "DELETE FROM exposed.funder_aliases WHERE funder_id = $1",
            ids[2]
        )
        .execute(&pool)
        .await?;
        db.refresh_funder_groups().await.unwrap();
        let groups = sqlx::query!("SELECT group_id AS \"group_id!\" FROM exposed.funder_alias_groups WHERE funder_id = $1", ids[2]).fetch_one(&pool).await?;
        assert_eq!(groups.group_id, ids[2]);
        assert_eq!(
            db.get_funders_by_text_search_score(&request("Gamma", 2))
                .await
                .unwrap()
                .matches[0]
                .0
                .funder()
                .id()
                .value(),
            ids[2]
        );
        Ok(())
    }

    #[sqlx::test(migrations = "../db/migrations")]
    async fn phonetic_candidates_deduplicate_before_limit(pool: PgPool) -> sqlx::Result<()> {
        let ids: Vec<_> = (1..=3).map(uuid::Uuid::from_u128).collect();
        for (id, name) in ids.iter().zip(["John Smith", "John Smyth", "John Brown"]) {
            sqlx::query!("INSERT INTO exposed.funders (id, funder_name, funder_kind) VALUES ($1, $2, 'Individual')", id, name).execute(&pool).await?;
        }
        for id in &ids[..2] {
            sqlx::query!("INSERT INTO exposed.funder_aliases (funder_id, funder_alias) VALUES ($1, 'Shared person')", id).execute(&pool).await?;
        }
        let db = ExposedDatabase::from(pool);
        db.refresh_funder_groups().await.unwrap();
        let request = EntitySearchRequest::new_with_strictness("jahn".into(), 2, 0.8).unwrap();
        let rows = db.get_funders_by_phonetic_search(&request).await.unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0.funder().id().value(), ids[0]);
        assert_eq!(rows[1].0.funder().id().value(), ids[2]);
        Ok(())
    }
}

#[cfg(test)]
mod reviewed_families {
    use crate::{
        domain::{
            models::{entity_search::EntitySearchRequest, funder::FunderId},
            repositories::{entity_details::EntityDetailsRepo, funder_repository::FunderRepo},
        },
        outbound::postgres::ExposedDatabase,
    };
    use sqlx::PgPool;

    #[sqlx::test(
        migrations = "../db/migrations",
        fixtures("../../../../db/fixtures/add_members.sql")
    )]
    async fn reviewed_families_combine_all_names_and_preserve_distinct_evidence(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        sqlx::query!("INSERT INTO exposed.declarations (source_declaration_id, member_id, category_id, category_name, fetched_at) SELECT 9999, id, 1, 'Employment and earnings', now() FROM exposed.members ORDER BY id LIMIT 1").execute(&pool).await?;
        let db = ExposedDatabase::from(pool.clone());
        let families = [
            (
                "bbc",
                "BBC",
                vec![
                    "BBC",
                    "BBC Contributor Payments",
                    "BBC Home Services",
                    "BBC  Studios",
                    "BBC Studios Ltd",
                    "BBC Wales Cymru",
                ],
            ),
            (
                "gb_news",
                "GB News",
                vec!["GB News", "GB News Limited", "GB News Ltd"],
            ),
            (
                "premier_league",
                "Premier League",
                vec![
                    "Premier League",
                    "FA Premier League Ltd",
                    "The FA Premier League Ltd",
                    "The Premier League",
                    "Premier League International Holdings Ltd",
                ],
            ),
        ];
        let mut next_id = 1;
        for (family_key, term, names) in families {
            let mut ids = Vec::new();
            for name in &names {
                let id = uuid::Uuid::from_u128(next_id);
                next_id += 1;
                ids.push(id);
                sqlx::query!(
                    "INSERT INTO exposed.funders (id, funder_name) VALUES ($1, $2)",
                    id,
                    name
                )
                .execute(&pool)
                .await?;
                sqlx::query!("INSERT INTO exposed.funding_entries (source_declaration_id, funder_id, amount, currency) VALUES (9999, $1, 10, 'GBP')", id).execute(&pool).await?;
            }
            let external = uuid::Uuid::from_u128(next_id);
            next_id += 1;
            let external_name = format!("Alias-connected {term} source");
            sqlx::query!(
                "INSERT INTO exposed.funders (id, funder_name) VALUES ($1, $2)",
                external,
                external_name
            )
            .execute(&pool)
            .await?;
            sqlx::query!(
                "INSERT INTO exposed.funder_aliases (funder_id, funder_alias) VALUES ($1, $2)",
                external,
                term
            )
            .execute(&pool)
            .await?;
            sqlx::query!("INSERT INTO exposed.funding_entries (source_declaration_id, funder_id, amount, currency) VALUES (9999, $1, 5, 'GBP')", external).execute(&pool).await?;
            ids.push(external);
            db.refresh_funder_groups().await.unwrap();
            let request = EntitySearchRequest::new_with_strictness(term.into(), 10, 0.8).unwrap();
            let results = db
                .get_funders_by_text_search_score(&request)
                .await
                .unwrap()
                .matches;
            assert_eq!(results.len(), 1, "{term}");
            assert_eq!(results[0].0.funder().id().value(), ids[0]);
            for id in &ids {
                let details = db
                    .funder_funding(FunderId::new(*id))
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(details.profile.funder.id().value(), *id);
                assert_eq!(
                    details
                        .profile
                        .member_ids
                        .iter()
                        .map(|id| id.value())
                        .collect::<Vec<_>>(),
                    ids
                );
                assert_eq!(details.declarations.len(), 1);
                let entries = &details.declarations[0].declaration.entries;
                assert_eq!(entries.len(), ids.len());
                assert_eq!(
                    entries
                        .iter()
                        .filter_map(|entry| entry.amount.as_ref())
                        .sum::<bigdecimal::BigDecimal>(),
                    bigdecimal::BigDecimal::from(i64::try_from(names.len() * 10 + 5).unwrap())
                );
                assert_eq!(
                    details.profile.family_links.len(),
                    names.len() * (names.len() - 1) / 2
                );
                assert!(
                    details
                        .profile
                        .family_links
                        .iter()
                        .all(|link| link.family_key == family_key
                            && link.left_funder_id.value() != external
                            && link.right_funder_id.value() != external)
                );
                assert!(
                    details
                        .profile
                        .family_links
                        .iter()
                        .any(|link| link.left_name == names[0] && link.right_name == names[1])
                );
                assert!(details.profile.alias_links.iter().any(
                    |link| link.right_funder_id.value() == external && link.right_alias == term
                ));
                assert!(
                    details
                        .profile
                        .alias_links
                        .iter()
                        .all(|link| link.normalized_alias == term.to_lowercase())
                );
                for name in &names {
                    assert!(details.profile.aliases.contains(&(*name).into()));
                }
            }
        }
        sqlx::query!("INSERT INTO exposed.funders (funder_name) VALUES ('Labour Party'), ('Mauritius Labour Party')").execute(&pool).await?;
        db.refresh_funder_groups().await.unwrap();
        let request =
            EntitySearchRequest::new_with_strictness("Labour Party".into(), 10, 0.8).unwrap();
        let results = db
            .get_funders_by_text_search_score(&request)
            .await
            .unwrap()
            .matches;
        assert_eq!(results.len(), 2);
        for (matched, _) in results {
            let profile = db
                .funder_funding(matched.funder().id())
                .await
                .unwrap()
                .unwrap()
                .profile;
            assert_eq!(profile.member_ids.len(), 1);
            assert!(profile.family_links.is_empty());
        }
        Ok(())
    }
}
