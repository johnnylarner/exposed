use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{PgPool, Postgres, Row, Transaction, pool::PoolConnection};
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::imports::core::{
    ImportError, Result,
    declarations::{Accepted, Funding, Payment, merge_funder_metadata, same_payments},
    members::{Member, validate_configured_term},
    ports::{ImportStore, NotificationQueue},
    refresh::{Changes, RefreshState},
};

fn database(error: sqlx::Error) -> ImportError {
    ImportError::Storage(Box::new(error))
}
fn midnight(date: NaiveDate) -> DateTime<Utc> {
    date.and_time(chrono::NaiveTime::MIN).and_utc()
}

#[derive(Clone)]
pub(crate) struct PostgresStore {
    pool: PgPool,
}
impl PostgresStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl ImportStore for PostgresStore {
    type Lease = PoolConnection<Postgres>;
    async fn acquire(&self) -> Result<Self::Lease> {
        let mut connection = self.pool.acquire().await.map_err(database)?;
        let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock(20260925, 1)")
            .fetch_one(&mut *connection)
            .await
            .map_err(database)?;
        if !acquired {
            return Err(ImportError::Busy);
        }
        // Session locks must never be returned to the pool. Closing also handles cancellation.
        connection.close_on_drop();
        Ok(connection)
    }

    async fn member_batch(&self, term: NaiveDate, members: &[Member]) -> Result<Changes> {
        let mut tx = self.pool.begin().await.map_err(database)?;
        let dates: Vec<DateTime<Utc>> =
            sqlx::query_scalar("SELECT term_start FROM exposed.parliament_terms")
                .fetch_all(&mut *tx)
                .await
                .map_err(database)?;
        validate_configured_term(term, dates.into_iter().map(|d| d.date_naive()))?;
        let term_id: Uuid = sqlx::query_scalar("INSERT INTO exposed.parliament_terms (term_start) VALUES ($1) ON CONFLICT (term_start) DO UPDATE SET term_start = EXCLUDED.term_start RETURNING id")
            .bind(midnight(term)).fetch_one(&mut *tx).await.map_err(database)?;
        let mut changes = Changes::default();
        for member in members {
            let p = &member.profile;
            let previous = sqlx::query("SELECT name, party_id, party_name, latest_house, latest_membership_from, is_current_commons FROM exposed.members WHERE parliament_member_id = $1")
                .bind(p.id).fetch_optional(&mut *tx).await.map_err(database)?;
            match previous {
                None => changes.inserted += 1,
                Some(row)
                    if row.get::<String, _>("name") == p.name
                        && row.get::<Option<i32>, _>("party_id") == p.party_id
                        && row.get::<Option<String>, _>("party_name") == p.party_name
                        && row.get::<i16, _>("latest_house") == p.house
                        && row.get::<Option<String>, _>("latest_membership_from")
                            == p.membership_from
                        && row.get::<bool, _>("is_current_commons") == member.current =>
                {
                    changes.unchanged += 1
                }
                Some(_) => changes.updated += 1,
            }
            let member_id: Uuid = sqlx::query_scalar("INSERT INTO exposed.members (parliament_member_id, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons) VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT (parliament_member_id) DO UPDATE SET name=EXCLUDED.name, party_id=EXCLUDED.party_id, party_name=EXCLUDED.party_name, latest_house=EXCLUDED.latest_house, latest_membership_from=EXCLUDED.latest_membership_from, is_current_commons=EXCLUDED.is_current_commons RETURNING id")
                .bind(p.id).bind(&p.name).bind(p.party_id).bind(&p.party_name).bind(p.house).bind(&p.membership_from).bind(member.current)
                .fetch_one(&mut *tx).await.map_err(database)?;
            for period in &member.periods {
                sqlx::query("INSERT INTO exposed.member_terms (member_id,term_id,house,source_start_date,source_end_date,served_from,served_until) VALUES ($1,$2,1,$3,$4,$5,$4) ON CONFLICT (member_id,term_id,house,source_start_date) DO UPDATE SET source_end_date=EXCLUDED.source_end_date, served_from=EXCLUDED.served_from, served_until=EXCLUDED.served_until")
                    .bind(member_id).bind(term_id).bind(midnight(period.source_start)).bind(period.end.map(midnight)).bind(midnight(period.served_from))
                    .execute(&mut *tx).await.map_err(database)?;
            }
            let starts: Vec<_> = member
                .periods
                .iter()
                .map(|p| midnight(p.source_start))
                .collect();
            sqlx::query("DELETE FROM exposed.member_terms WHERE member_id=$1 AND term_id=$2 AND NOT (source_start_date = ANY($3))")
                .bind(member_id).bind(term_id).bind(starts).execute(&mut *tx).await.map_err(database)?;
        }
        tx.commit().await.map_err(database)?;
        Ok(changes)
    }

    async fn cohort(&self, term: NaiveDate) -> Result<BTreeMap<i32, Uuid>> {
        let rows = sqlx::query("SELECT DISTINCT m.id,m.parliament_member_id FROM exposed.members m JOIN exposed.member_terms s ON s.member_id=m.id JOIN exposed.parliament_terms t ON t.id=s.term_id WHERE t.term_start=$1 AND s.house=1 ORDER BY m.parliament_member_id")
            .bind(midnight(term)).fetch_all(&self.pool).await.map_err(database)?;
        Ok(rows
            .into_iter()
            .map(|r| (r.get("parliament_member_id"), r.get("id")))
            .collect())
    }

    async fn publish(&self, member: Uuid, declarations: &[Accepted]) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(database)?;
        for accepted in declarations {
            write_declaration(&mut tx, member, accepted).await?;
        }
        tx.commit().await.map_err(database)
    }

    async fn refresh_state(&self, term: NaiveDate) -> Result<RefreshState> {
        let row = sqlx::query("SELECT * FROM exposed.import_refresh_state WHERE term_start=$1")
            .bind(midnight(term))
            .fetch_optional(&self.pool)
            .await
            .map_err(database)?;
        Ok(row.map_or_else(RefreshState::default, |r| RefreshState {
            last_completed: r.get("last_completed"),
            last_attempt: r.get("last_attempt"),
            next_attempt: r.get("next_attempt"),
            outcome: r.get("outcome"),
            rejected: r.get::<i32, _>("rejected") as usize,
            failures: r.get::<i32, _>("failures") as u32,
            new_members: r.get("new_members"),
        }))
    }
    async fn record_refresh(&self, term: NaiveDate, state: &RefreshState) -> Result<()> {
        sqlx::query("INSERT INTO exposed.import_refresh_state (term_start,last_completed,last_attempt,next_attempt,outcome,rejected,failures,new_members) VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT (term_start) DO UPDATE SET last_completed=EXCLUDED.last_completed,last_attempt=EXCLUDED.last_attempt,next_attempt=EXCLUDED.next_attempt,outcome=EXCLUDED.outcome,rejected=EXCLUDED.rejected,failures=EXCLUDED.failures,new_members=EXCLUDED.new_members")
            .bind(midnight(term)).bind(state.last_completed).bind(state.last_attempt).bind(state.next_attempt).bind(&state.outcome)
            .bind(i32::try_from(state.rejected).unwrap_or(i32::MAX)).bind(i32::try_from(state.failures).unwrap_or(i32::MAX)).bind(&state.new_members)
            .execute(&self.pool).await.map_err(database)?;
        Ok(())
    }
}

async fn write_funder(tx: &mut Transaction<'_, Postgres>, entry: &Funding) -> Result<Option<Uuid>> {
    let Some(name) = &entry.funder_name else {
        return Ok(None);
    };
    let previous = sqlx::query(
        "SELECT funder_kind,company_number FROM exposed.funders WHERE funder_name=$1 FOR UPDATE",
    )
    .bind(name)
    .fetch_optional(&mut **tx)
    .await
    .map_err(database)?;
    let (kind, number) = previous.map_or((None, None), |r| {
        (r.get("funder_kind"), r.get("company_number"))
    });
    let (kind, number) = merge_funder_metadata(entry, kind, number);
    let id = sqlx::query_scalar("INSERT INTO exposed.funders (funder_name,funder_kind,company_number) VALUES ($1,$2,$3) ON CONFLICT (funder_name) DO UPDATE SET funder_kind=EXCLUDED.funder_kind,company_number=EXCLUDED.company_number RETURNING id")
        .bind(name).bind(kind).bind(number).fetch_one(&mut **tx).await.map_err(database)?;
    Ok(Some(id))
}

async fn write_declaration(
    tx: &mut Transaction<'_, Postgres>,
    member: Uuid,
    accepted: &Accepted,
) -> Result<()> {
    let d = &accepted.declaration.draft;
    sqlx::query("INSERT INTO exposed.declarations (source_declaration_id,member_id,category_id,category_name,registration_date,fetched_at) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (source_declaration_id) DO UPDATE SET member_id=EXCLUDED.member_id,category_id=EXCLUDED.category_id,category_name=EXCLUDED.category_name,registration_date=EXCLUDED.registration_date,fetched_at=EXCLUDED.fetched_at")
        .bind(d.id).bind(member).bind(d.category_id).bind(&d.category_name).bind(d.registration_date.map(midnight)).bind(accepted.fetched_at)
        .execute(&mut **tx).await.map_err(database)?;
    let mut payments = Vec::new();
    for entry in &d.funding {
        payments.push(Payment {
            funder_id: write_funder(tx, entry).await?,
            amount: entry.amount.clone(),
            currency: entry.currency.clone(),
            payment_type: entry.payment_type.clone(),
        });
    }
    let rows = sqlx::query("SELECT funder_id,amount,currency,payment_type FROM exposed.funding_entries WHERE source_declaration_id=$1")
        .bind(d.id).fetch_all(&mut **tx).await.map_err(database)?;
    let previous: Vec<_> = rows
        .into_iter()
        .map(|r| Payment {
            funder_id: r.get("funder_id"),
            amount: r.get("amount"),
            currency: r.get("currency"),
            payment_type: r.get("payment_type"),
        })
        .collect();
    if same_payments(&payments, &previous) {
        return Ok(());
    }
    sqlx::query("DELETE FROM exposed.funding_entries WHERE source_declaration_id=$1")
        .bind(d.id)
        .execute(&mut **tx)
        .await
        .map_err(database)?;
    for p in payments {
        sqlx::query("INSERT INTO exposed.funding_entries (source_declaration_id,funder_id,amount,currency,payment_type) VALUES ($1,$2,$3,$4,$5)")
            .bind(d.id).bind(p.funder_id).bind(p.amount).bind(p.currency).bind(p.payment_type).execute(&mut **tx).await.map_err(database)?;
    }
    Ok(())
}

impl NotificationQueue for PostgresStore {
    async fn enqueue(&self, key: &str, message: &str) -> Result<()> {
        sqlx::query("INSERT INTO exposed.import_notifications (event_key,message) VALUES ($1,$2) ON CONFLICT (event_key) DO NOTHING")
            .bind(key).bind(message).execute(&self.pool).await.map_err(database)?;
        Ok(())
    }
    async fn pending(&self, now: DateTime<Utc>) -> Result<Option<(Uuid, String, i32)>> {
        let row = sqlx::query("UPDATE exposed.import_notifications SET next_attempt=$1 + INTERVAL '2 minutes' WHERE id=(SELECT id FROM exposed.import_notifications WHERE delivered_at IS NULL AND next_attempt <= $1 ORDER BY next_attempt,id FOR UPDATE SKIP LOCKED LIMIT 1) RETURNING id,message,attempts")
            .bind(now).fetch_optional(&self.pool).await.map_err(database)?;
        Ok(row.map(|r| (r.get("id"), r.get("message"), r.get("attempts"))))
    }
    async fn delivered(&self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE exposed.import_notifications SET delivered_at=now() WHERE id=$1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(database)?;
        Ok(())
    }
    async fn retry_notification(&self, id: Uuid, next: DateTime<Utc>) -> Result<()> {
        sqlx::query("UPDATE exposed.import_notifications SET attempts=attempts+1,next_attempt=$2 WHERE id=$1")
            .bind(id).bind(next).execute(&self.pool).await.map_err(database)?;
        Ok(())
    }
}
