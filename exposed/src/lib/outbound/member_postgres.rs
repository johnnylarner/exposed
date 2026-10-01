//! PostgreSQL reconciliation of one complete, prevalidated Commons refresh.
use crate::domain::{
    models::member_ingestion::{MemberImport, MemberImportSummary, MemberRecord, MemberRefresh},
    repositories::member_writer::{MemberWriteError, MemberWriter},
};
use anyhow::{Context, ensure};
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{Connection, PgConnection, Postgres, Row, Transaction, postgres::PgConnectOptions};
use std::{str::FromStr, time::Duration};
use uuid::Uuid;

/// Offline member writer; construction validates configuration without connecting.
#[derive(Clone)]
pub struct MemberDatabase {
    options: PgConnectOptions,
}
impl MemberDatabase {
    /// Store validated connection settings for a later atomic refresh.
    ///
    /// # Errors
    /// Rejects an empty or invalid PostgreSQL connection string.
    pub fn new(connection_string: &str) -> Result<Self, MemberWriteError> {
        if connection_string.trim().is_empty() {
            return Err(MemberWriteError(
                "connection_string must not be empty".into(),
            ));
        }
        let options = PgConnectOptions::from_str(connection_string)
            .map_err(|_| MemberWriteError("invalid PostgreSQL connection string".into()))?
            .application_name("exposed-member-load");
        Ok(Self { options })
    }

    async fn write(&self, refresh: &MemberRefresh) -> anyhow::Result<MemberImportSummary> {
        let mut connection = tokio::time::timeout(
            Duration::from_secs(10),
            PgConnection::connect_with(&self.options),
        )
        .await
        .context("PostgreSQL connection timed out")?
        .context("connect to PostgreSQL")?;
        let mut transaction = connection.begin().await?;
        let term_id = ensure_term(&mut transaction, refresh.term_start).await?;
        let mut summary = MemberImportSummary {
            excluded_candidates: refresh.excluded_candidates,
            ..MemberImportSummary::default()
        };
        for import in &refresh.members {
            let change = write_member(&mut transaction, term_id, import)
                .await
                .with_context(|| format!("member {}", import.member.parliament_member_id))?;
            match change {
                ProfileChange::Inserted => summary.inserted += 1,
                ProfileChange::Updated => summary.updated += 1,
                ProfileChange::Unchanged => summary.unchanged += 1,
            }
            summary.members += 1;
            if import.member.is_current_commons {
                summary.current_commons += 1;
            } else {
                summary.former_commons += 1;
            }
            summary.service_periods += import.periods.len();
        }
        transaction
            .commit()
            .await
            .context("commit member refresh")?;
        Ok(summary)
    }
}
impl MemberWriter for MemberDatabase {
    async fn refresh_members(
        &self,
        refresh: &MemberRefresh,
    ) -> Result<MemberImportSummary, MemberWriteError> {
        self.write(refresh)
            .await
            .map_err(|error| MemberWriteError(format!("{error:#}")))
    }
}

enum ProfileChange {
    Inserted,
    Updated,
    Unchanged,
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
    import: &MemberImport,
) -> anyhow::Result<ProfileChange> {
    let member = &import.member;
    let previous = sqlx::query("SELECT parliament_member_id, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons FROM exposed.members WHERE parliament_member_id = $1")
        .bind(member.parliament_member_id).fetch_optional(&mut **transaction).await?;
    let change = if let Some(row) = previous {
        let stored = MemberRecord {
            parliament_member_id: row.try_get("parliament_member_id")?,
            name: row.try_get("name")?,
            party_id: row.try_get("party_id")?,
            party_name: row.try_get("party_name")?,
            latest_house: row.try_get("latest_house")?,
            latest_membership_from: row.try_get("latest_membership_from")?,
            is_current_commons: row.try_get("is_current_commons")?,
        };
        if stored == *member {
            ProfileChange::Unchanged
        } else {
            ProfileChange::Updated
        }
    } else {
        ProfileChange::Inserted
    };
    let member_id: Uuid = sqlx::query_scalar(
        "INSERT INTO exposed.members (id, parliament_member_id, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (parliament_member_id) DO UPDATE SET name = EXCLUDED.name,
           party_id = EXCLUDED.party_id, party_name = EXCLUDED.party_name,
           latest_house = EXCLUDED.latest_house, latest_membership_from = EXCLUDED.latest_membership_from,
           is_current_commons = EXCLUDED.is_current_commons RETURNING id")
        .bind(Uuid::now_v7()).bind(member.parliament_member_id).bind(&member.name).bind(member.party_id)
        .bind(&member.party_name).bind(member.latest_house).bind(&member.latest_membership_from).bind(member.is_current_commons)
        .fetch_one(&mut **transaction).await?;
    for period in &import.periods {
        sqlx::query(
            "INSERT INTO exposed.member_terms (id, member_id, term_id, house, source_start_date, source_end_date, served_from, served_until)
             VALUES ($1, $2, $3, 1, $4, $5, $6, $5)
             ON CONFLICT (member_id, term_id, house, source_start_date) DO UPDATE SET
                source_end_date = EXCLUDED.source_end_date, served_from = EXCLUDED.served_from, served_until = EXCLUDED.served_until")
            .bind(Uuid::now_v7()).bind(member_id).bind(term_id).bind(midnight(period.source_start_date))
            .bind(period.source_end_date.map(midnight)).bind(midnight(period.served_from)).execute(&mut **transaction).await?;
    }
    let starts: Vec<_> = import
        .periods
        .iter()
        .map(|p| midnight(p.source_start_date))
        .collect();
    sqlx::query("DELETE FROM exposed.member_terms WHERE member_id = $1 AND term_id = $2 AND (house <> 1 OR NOT (source_start_date = ANY($3)))")
        .bind(member_id).bind(term_id).bind(starts).execute(&mut **transaction).await?;
    Ok(change)
}
