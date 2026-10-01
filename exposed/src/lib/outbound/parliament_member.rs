//! PostgreSQL member search and atomic Commons service reconciliation.

use anyhow::{Context, ensure};
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domain::{
        models::{
            entity_search::EntitySearchRequest,
            member_ingestion::{CommonsService, MemberRefresh},
            parliament_member::ParliamentMember,
            search_similarity::SearchSimilarity,
        },
        repositories::parliament_member_repository::{
            ParliamentMemberRepo, ParliamentMemberRepoError,
        },
    },
    outbound::postgres::ExposedDatabase,
};

impl ParliamentMemberRepo for ExposedDatabase {
    async fn refresh_members(
        &self,
        refresh: &MemberRefresh,
    ) -> Result<(), ParliamentMemberRepoError> {
        let result = async {
            let mut transaction = self.pool().begin().await?;
            let term_id = ensure_term(&mut transaction, refresh.term_start()).await?;
            for service in refresh.members() {
                write_member(&mut transaction, term_id, service)
                    .await
                    .with_context(|| {
                        format!("member {}", service.member().parliament_member_id())
                    })?;
            }
            transaction
                .commit()
                .await
                .context("commit member refresh")?;
            Ok::<_, anyhow::Error>(())
        }
        .await;
        result.map_err(|error| ParliamentMemberRepoError::DatabaseError(format!("{error:#}")))
    }

    async fn get_members_by_text_search_score(
        &self,
        req: &EntitySearchRequest,
    ) -> Result<Vec<(ParliamentMember, SearchSimilarity)>, ParliamentMemberRepoError> {
        sqlx::query!(
            "
            SELECT  
                m.name,
                m.parliament_member_id,
                m.party_id,
                m.party_name,
                m.latest_house,
                m.latest_membership_from as constituency,
                 word_similarity($1, m.name) as similarity_score,
                row_number() OVER (ORDER BY word_similarity($1, m.name) DESC) as rank
            FROM members m
            WHERE word_similarity($1, m.name) >= $2
            ORDER BY similarity_score DESC
            ",
            req.term(),
            req.strictness()
        )
        .fetch_all(self.pool())
        .await
        .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?
        .into_iter()
        .take_while(|r| {
            r.rank.unwrap_or_else(|| i64::from(req.max_entries() + 1))
                <= i64::from(req.max_entries())
        })
        .map(|r| {
            let member = ParliamentMember::new(
                r.name,
                r.parliament_member_id,
                r.party_id,
                r.party_name,
                r.latest_house,
                r.constituency,
            )
            .map_err(|error| ParliamentMemberRepoError::DatabaseError(error.to_string()))?;
            let score = SearchSimilarity::from(r.similarity_score.unwrap_or(0_f32));
            Ok((member, score))
        })
        .collect()
    }
}

const fn midnight(date: NaiveDate) -> DateTime<Utc> {
    date.and_time(chrono::NaiveTime::MIN).and_utc()
}

async fn ensure_term(
    transaction: &mut Transaction<'_, Postgres>,
    start: NaiveDate,
) -> anyhow::Result<Uuid> {
    let terms: Vec<(Uuid, DateTime<Utc>)> =
        sqlx::query_as("SELECT id, term_start FROM exposed.parliament_terms")
            .fetch_all(&mut **transaction)
            .await?;
    ensure!(
        terms.iter().all(|(_, date)| *date == midnight(start)),
        "database contains another term; this operation refreshes one configured Parliament"
    );
    if let Some((id, _)) = terms.first() {
        return Ok(*id);
    }
    let id = sqlx::query_scalar(
        "INSERT INTO exposed.parliament_terms (id, term_start) VALUES ($1, $2) RETURNING id",
    )
    .bind(Uuid::now_v7())
    .bind(midnight(start))
    .fetch_one(&mut **transaction)
    .await?;
    Ok(id)
}

async fn write_member(
    transaction: &mut Transaction<'_, Postgres>,
    term_id: Uuid,
    service: &CommonsService,
) -> anyhow::Result<()> {
    let member = service.member();
    let member_id: Uuid = sqlx::query_scalar(
        "INSERT INTO exposed.members (id, parliament_member_id, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (parliament_member_id) DO UPDATE SET name = EXCLUDED.name,
           party_id = EXCLUDED.party_id, party_name = EXCLUDED.party_name,
           latest_house = EXCLUDED.latest_house, latest_membership_from = EXCLUDED.latest_membership_from,
           is_current_commons = EXCLUDED.is_current_commons RETURNING id")
        .bind(Uuid::now_v7()).bind(member.parliament_member_id()).bind(member.name()).bind(member.party_id())
        .bind(member.party_name()).bind(member.latest_house()).bind(member.latest_membership_from()).bind(service.is_current_commons())
        .fetch_one(&mut **transaction).await?;
    for period in service.periods() {
        sqlx::query(
            "INSERT INTO exposed.member_terms (id, member_id, term_id, house, source_start_date, source_end_date, served_from, served_until)
             VALUES ($1, $2, $3, 1, $4, $5, $6, $5)
             ON CONFLICT (member_id, term_id, house, source_start_date) DO UPDATE SET
                source_end_date = EXCLUDED.source_end_date, served_from = EXCLUDED.served_from, served_until = EXCLUDED.served_until")
            .bind(Uuid::now_v7()).bind(member_id).bind(term_id).bind(midnight(period.source_start_date()))
            .bind(period.source_end_date().map(midnight)).bind(midnight(period.served_from())).execute(&mut **transaction).await?;
    }
    let starts: Vec<_> = service
        .periods()
        .iter()
        .map(|p| midnight(p.source_start_date()))
        .collect();
    sqlx::query("DELETE FROM exposed.member_terms WHERE member_id = $1 AND term_id = $2 AND (house <> 1 OR NOT (source_start_date = ANY($3)))")
        .bind(member_id).bind(term_id).bind(starts).execute(&mut **transaction).await?;
    Ok(())
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
