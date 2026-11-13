//! Concrete implementation of the repository

use crate::{
    domain::{
        models::{
            entity_search::{EntitySearchRequest, PhoneticRank},
            membership::{MemberProfile, Membership, MembershipKind, MembershipStatusProvenance},
            parliament_member::{MemberId, ParliamentMember},
            search_similarity::SearchSimilarity,
            source_timestamp::SourceTimestamp,
        },
        repositories::parliament_member_repository::{
            ParliamentMemberRepo, ParliamentMemberRepoError,
        },
    },
    outbound::postgres::{ExposedDatabase, word_similarity_candidate_threshold},
};

impl ParliamentMemberRepo for ExposedDatabase {
    async fn get_stored_member_ids(&self) -> Result<Vec<MemberId>, ParliamentMemberRepoError> {
        sqlx::query!(
            "
            SELECT parliament_member_id
            FROM exposed.members m
            WHERE m.commons_imported_at IS NOT NULL
            ORDER BY parliament_member_id
            "
        )
        .fetch_all(self.pool())
        .await
        .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?
        .into_iter()
        .map(|row| {
            let parliament_id = u32::try_from(row.parliament_member_id)
                .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?;
            MemberId::new(parliament_id)
                .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))
        })
        .collect()
    }

    async fn get_members_by_text_search_score(
        &self,
        req: &EntitySearchRequest,
    ) -> Result<Vec<(ParliamentMember, SearchSimilarity)>, ParliamentMemberRepoError> {
        let mut tx = self
            .pool()
            .begin()
            .await
            .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?;
        sqlx::query!(
            "SELECT set_config('pg_trgm.word_similarity_threshold', $1, true)",
            word_similarity_candidate_threshold(req.strictness())
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?;
        let rows = sqlx::query!(
            "
            SELECT  
                m.name,
                m.parliament_member_id,
                m.party_name AS \"party_name?\",
                m.party_id AS \"party_id?\",
                m.membership_from AS \"constituency?\", m.membership_kind, m.membership_status_provenance, m.membership_start_date, m.membership_end_date, m.membership_status, m.membership_status_active,
                 exposed.word_similarity($1, m.name) as similarity_score
            FROM exposed.members m
            WHERE m.commons_imported_at IS NOT NULL AND m.name OPERATOR(exposed.%>) $1 AND exposed.word_similarity($1, m.name) >= $2
            ORDER BY similarity_score DESC, m.id
            LIMIT $3
            ",
            req.term(),
            req.strictness(),
            i64::from(req.max_entries())
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?;
        tx.commit()
            .await
            .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?;
        rows.into_iter()
            .map(|r| {
                let member = ParliamentMember {
                    profile: MemberProfile {
                        id: MemberId::new(
                            u32::try_from(r.parliament_member_id).map_err(database_error)?,
                        )
                        .map_err(database_error)?,
                        name: r.name,
                        party_name: r.party_name,
                        party_id: r
                            .party_id
                            .map(u32::try_from)
                            .transpose()
                            .map_err(database_error)?,
                        membership: Membership {
                            kind: r
                                .membership_kind
                                .parse::<MembershipKind>()
                                .map_err(database_error)?,
                            status_provenance: r
                                .membership_status_provenance
                                .parse::<MembershipStatusProvenance>()
                                .map_err(database_error)?,
                            membership_from: r.constituency,
                            membership_start_date: r
                                .membership_start_date
                                .map(SourceTimestamp::try_from)
                                .transpose()
                                .map_err(database_error)?,
                            membership_end_date: r
                                .membership_end_date
                                .map(SourceTimestamp::try_from)
                                .transpose()
                                .map_err(database_error)?,
                            membership_status: r.membership_status,
                            membership_status_active: r.membership_status_active,
                        },
                    },
                    captured_at: None,
                    capture_key: None,
                };
                let score = SearchSimilarity::from(r.similarity_score.unwrap_or(0_f32));
                Ok((member, score))
            })
            .collect()
    }

    async fn get_members_by_phonetic_search(
        &self,
        req: &EntitySearchRequest,
    ) -> Result<Vec<(ParliamentMember, PhoneticRank)>, ParliamentMemberRepoError> {
        let rows = sqlx::query!(
            r#"
            SELECT m.name, m.parliament_member_id, m.party_name AS "party_name?", m.party_id AS "party_id?",
                   m.membership_from AS "constituency?", m.membership_kind, m.membership_status_provenance, m.membership_start_date, m.membership_end_date, m.membership_status, m.membership_status_active,
                   correction.distance AS "distance!",
                   exposed.word_similarity(correction.corrected_query, m.name) AS similarity_score
            FROM exposed.members m
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
                    FROM unnest(exposed.person_name_words(m.name)) AS cw(word)
                    WHERE exposed.soundex(cw.word) = exposed.soundex(q.word)
                    ORDER BY distance, cw.word COLLATE "C"
                    LIMIT 1
                ) best ON best.distance <= 1
                HAVING count(best.word) = count(*) AND count(*) > 0
            ) correction
            WHERE m.commons_imported_at IS NOT NULL AND exposed.person_name_codes(m.name) @> exposed.person_name_codes($1)
              AND exposed.word_similarity(correction.corrected_query, m.name) >= $2
            ORDER BY correction.distance, similarity_score DESC, m.parliament_member_id
            LIMIT $3
            "#,
            req.term(),
            req.strictness(),
            i64::from(req.max_entries())
        )
        .fetch_all(self.pool())
        .await
        .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?;
        rows.into_iter()
            .map(|r| {
                let member = ParliamentMember {
                    profile: MemberProfile {
                        id: MemberId::new(
                            u32::try_from(r.parliament_member_id).map_err(database_error)?,
                        )
                        .map_err(database_error)?,
                        name: r.name,
                        party_name: r.party_name,
                        party_id: r
                            .party_id
                            .map(u32::try_from)
                            .transpose()
                            .map_err(database_error)?,
                        membership: Membership {
                            kind: r
                                .membership_kind
                                .parse::<MembershipKind>()
                                .map_err(database_error)?,
                            status_provenance: r
                                .membership_status_provenance
                                .parse::<MembershipStatusProvenance>()
                                .map_err(database_error)?,
                            membership_from: r.constituency,
                            membership_start_date: r
                                .membership_start_date
                                .map(SourceTimestamp::try_from)
                                .transpose()
                                .map_err(database_error)?,
                            membership_end_date: r
                                .membership_end_date
                                .map(SourceTimestamp::try_from)
                                .transpose()
                                .map_err(database_error)?,
                            membership_status: r.membership_status,
                            membership_status_active: r.membership_status_active,
                        },
                    },
                    captured_at: None,
                    capture_key: None,
                };
                let score = PhoneticRank::new(
                    u32::try_from(r.distance)
                        .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?,
                    SearchSimilarity::from(r.similarity_score.unwrap_or(0_f32)),
                );
                Ok((member, score))
            })
            .collect()
    }

    async fn upsert_members(
        &self,
        members: &[ParliamentMember],
    ) -> Result<(), ParliamentMemberRepoError> {
        let mut tx = self
            .pool()
            .begin()
            .await
            .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?;

        let mut members: Vec<_> = members.iter().collect();
        members.sort_by_key(|member| member.member_id());
        for m in members {
            super::member_identity::upsert(&mut tx, &m.profile, m.captured_at, m.capture_key, true)
                .await
                .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?;
        }
        tx.commit()
            .await
            .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod scoring {
    use sqlx::PgPool;

    use crate::{
        domain::{
            models::entity_search::EntitySearchRequest,
            repositories::parliament_member_repository::ParliamentMemberRepo,
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
        let results = db.get_members_by_text_search_score(&term).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results.first().unwrap().0.name(), "John McDonnell");

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
        let results = db.get_members_by_text_search_score(&term).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results.first().unwrap().0.name(), "John McDonnell");

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

        let term = EntitySearchRequest::new_strict("John McDonnell".into(), 5).unwrap();
        let results = db.get_members_by_text_search_score(&term).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results.first().unwrap().0.name(), "John McDonnell");

        Ok(())
    }
}

#[cfg(test)]
mod threshold_scoring {
    use crate::{
        domain::{
            models::entity_search::EntitySearchRequest,
            repositories::parliament_member_repository::ParliamentMemberRepo,
        },
        outbound::postgres::ExposedDatabase,
    };
    use sqlx::PgPool;

    #[sqlx::test(
        migrations = "../db/migrations",
        fixtures("../../../../db/fixtures/add_members.sql")
    )]
    async fn inclusive_thresholds_are_local(pool: PgPool) -> sqlx::Result<()> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_with(
                pool.connect_options()
                    .as_ref()
                    .clone()
                    .options([("search_path", "public")]),
            )
            .await?;
        sqlx::query!("SELECT exposed.word_similarity('initialize', 'initialize')")
            .fetch_one(&pool)
            .await?;
        let initial = sqlx::query!(
            "SELECT current_setting('pg_trgm.word_similarity_threshold') AS threshold"
        )
        .fetch_one(&pool)
        .await?
        .threshold;
        let db = ExposedDatabase::from(pool.clone());
        let boundary =
            sqlx::query!("SELECT exposed.word_similarity('John McD', 'John McDonnell') AS score")
                .fetch_one(&pool)
                .await?
                .score
                .unwrap();
        assert!(boundary > 0.0 && boundary < 1.0);
        for (term, threshold, count) in [
            ("zzzzzz", 0.0, 2),
            ("John McDonnell", 1.0, 1),
            ("John McD", boundary, 1),
        ] {
            let req = EntitySearchRequest::new_with_strictness(term.into(), 5, threshold).unwrap();
            let results = db.get_members_by_text_search_score(&req).await.unwrap();
            assert_eq!(results.len(), count);
            let actual = sqlx::query!(
                "SELECT current_setting('pg_trgm.word_similarity_threshold') AS threshold"
            )
            .fetch_one(&pool)
            .await?
            .threshold;
            assert_eq!(actual, initial);
        }
        Ok(())
    }
}

fn database_error(error: impl std::fmt::Display) -> ParliamentMemberRepoError {
    ParliamentMemberRepoError::DatabaseError(error.to_string())
}
