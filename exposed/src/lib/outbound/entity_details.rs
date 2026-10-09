use crate::{
    domain::{
        models::{
            entity_details::{
                DeclaredFunding, FunderFunding, FunderProfile, FunderReference, MemberDetails,
                MemberProfile, RecentDeclaration, RecipientAllocation,
            },
            funder::{Funder, FunderId, FunderKind},
            parliament_member::MemberId,
        },
        repositories::entity_details::{EntityDetailsRepo, EntityDetailsRepoError},
    },
    outbound::postgres::ExposedDatabase,
};
use std::str::FromStr;

const CURRENCY_WHITESPACE: &str = "\t\n\u{000b}\u{000c}\r \u{0085}\u{00a0}\u{1680}\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}";

impl EntityDetailsRepo for ExposedDatabase {
    async fn member(
        &self,
        id: MemberId,
        recent_limit: usize,
    ) -> Result<Option<MemberDetails>, EntityDetailsRepoError> {
        let mut tx = self.pool().begin().await?;
        sqlx::query!("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *tx)
            .await?;
        let row = sqlx::query!(
            "SELECT parliament_member_id, name, party_id, party_name, latest_membership_from, is_current_commons,
                (SELECT count(*) FROM exposed.declarations d WHERE d.member_id = m.id) AS \"declaration_count!\"
             FROM exposed.members m WHERE parliament_member_id = $1", i32::try_from(id.value()).map_err(|error| EntityDetailsRepoError::DatabaseError(error.to_string()))?
        ).fetch_optional(&mut *tx).await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let member = MemberProfile {
            id,
            name: row.name,
            party_id: row.party_id,
            party_name: row.party_name,
            membership_from: row.latest_membership_from,
            is_current_commons: row.is_current_commons,
        };
        let rows = sqlx::query!(
            "WITH recent AS (
                SELECT d.source_declaration_id, d.category_name, d.registration_date
                FROM exposed.declarations d JOIN exposed.members m ON m.id = d.member_id
                WHERE m.parliament_member_id = $1
                ORDER BY d.registration_date DESC NULLS LAST, d.source_declaration_id DESC LIMIT $2
             )
             SELECT d.source_declaration_id, d.category_name, d.registration_date,
                e.id AS \"entry_id?\", e.amount, e.currency, e.payment_type, f.id AS \"funder_id?\", f.funder_name AS \"funder_name?\"
             FROM recent d LEFT JOIN exposed.funding_entries e ON e.source_declaration_id = d.source_declaration_id
             LEFT JOIN exposed.funders f ON f.id = e.funder_id
             ORDER BY d.registration_date DESC NULLS LAST, d.source_declaration_id DESC, e.id",
             i32::try_from(id.value()).map_err(|error| EntityDetailsRepoError::DatabaseError(error.to_string()))?,
             i64::try_from(recent_limit).map_err(|error| EntityDetailsRepoError::DatabaseError(error.to_string()))?
        ).fetch_all(&mut *tx).await?;
        let mut declarations: Vec<RecentDeclaration> = Vec::new();
        for row in rows {
            if declarations
                .last()
                .is_none_or(|declaration| declaration.source_id != row.source_declaration_id)
            {
                declarations.push(RecentDeclaration {
                    source_id: row.source_declaration_id,
                    category_name: row.category_name,
                    registered_at: row
                        .registration_date
                        .map(|date| {
                            chrono::DateTime::from_timestamp(
                                date.unix_timestamp(),
                                date.nanosecond(),
                            )
                            .ok_or_else(|| {
                                EntityDetailsRepoError::DatabaseError(
                                    "Stored registration date is out of range".into(),
                                )
                            })
                        })
                        .transpose()?,
                    entries: Vec::new(),
                });
            }
            if row.entry_id.is_some() {
                let funder = row
                    .funder_id
                    .zip(row.funder_name)
                    .map(|(id, name)| FunderReference {
                        id: FunderId::new(id),
                        name,
                    });
                if let Some(declaration) = declarations.last_mut() {
                    declaration.entries.push(DeclaredFunding {
                        funder,
                        amount: row.amount,
                        currency: row.currency,
                        payment_type: row.payment_type,
                    });
                }
            }
        }
        tx.commit().await?;
        Ok(Some(MemberDetails {
            member,
            declarations,
            declaration_count: row.declaration_count,
            declaration_limit: recent_limit,
        }))
    }

    async fn funder_funding(
        &self,
        id: FunderId,
    ) -> Result<Option<FunderFunding>, EntityDetailsRepoError> {
        let mut tx = self.pool().begin().await?;
        sqlx::query!("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *tx)
            .await?;
        let row = sqlx::query!(
            "SELECT funder_name, funder_kind, company_number FROM exposed.funders WHERE id = $1",
            id.value()
        )
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let kind = row
            .funder_kind
            .as_deref()
            .map_or(Ok(FunderKind::NotSpecified), FunderKind::from_str)
            .map_err(|error| EntityDetailsRepoError::DatabaseError(error.to_string()))?;
        let aliases = sqlx::query_scalar!(
            "SELECT funder_alias FROM exposed.funder_aliases WHERE funder_id = $1 ORDER BY funder_alias COLLATE \"C\"",
            id.value()
        )
        .fetch_all(&mut *tx)
        .await?;
        let profile = FunderProfile {
            funder: Funder::new(id, row.funder_name, kind),
            company_number: row.company_number,
            aliases,
        };
        let rows = sqlx::query!(
            "WITH funding AS (
                SELECT source_declaration_id, amount, NULLIF(btrim(currency, $2), '') AS currency
                FROM exposed.funding_entries WHERE funder_id = $1
             )
             SELECT m.parliament_member_id, m.name, m.party_id, m.party_name, m.latest_membership_from, m.is_current_commons,
                e.currency,
                CASE WHEN e.currency IS NOT NULL THEN sum(e.amount) END AS known_total,
                count(*) AS \"entry_count!\", count(*) FILTER (WHERE e.amount IS NULL) AS \"unknown_amount_count!\"
             FROM funding e JOIN exposed.declarations d ON d.source_declaration_id = e.source_declaration_id
             JOIN exposed.members m ON m.id = d.member_id
             GROUP BY m.id, e.currency
             ORDER BY m.parliament_member_id, currency NULLS LAST", id.value(), CURRENCY_WHITESPACE
        ).fetch_all(&mut *tx).await?;
        let allocations = rows
            .into_iter()
            .map(|row| {
                let id = row
                    .parliament_member_id
                    .to_string()
                    .parse::<MemberId>()
                    .map_err(|error| EntityDetailsRepoError::DatabaseError(error.to_string()))?;
                Ok(RecipientAllocation {
                    member: MemberProfile {
                        id,
                        name: row.name,
                        party_id: row.party_id,
                        party_name: row.party_name,
                        membership_from: row.latest_membership_from,
                        is_current_commons: row.is_current_commons,
                    },
                    currency: row.currency,
                    known_total: row.known_total,
                    entry_count: row.entry_count,
                    unknown_amount_count: row.unknown_amount_count,
                })
            })
            .collect::<Result<Vec<_>, EntityDetailsRepoError>>()?;
        tx.commit().await?;
        Ok(Some(FunderFunding {
            profile,
            allocations,
        }))
    }
}

#[cfg(test)]
mod tests;

impl From<sqlx::Error> for EntityDetailsRepoError {
    fn from(error: sqlx::Error) -> Self {
        Self::DatabaseError(error.to_string())
    }
}
