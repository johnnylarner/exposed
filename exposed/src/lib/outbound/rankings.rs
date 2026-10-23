use super::currency::CURRENCY_WHITESPACE;
use crate::{
    domain::{
        models::{
            entity_details::{
                DeclarationCategory, DeclaredFunding, FunderReference, MemberDeclaration,
            },
            funder::{FunderId, FunderKind},
            rankings::{RankedFunder, RankedMember, RankedParty, RankingDeclaration},
        },
        repositories::rankings::{RankingsError, RankingsRepo},
    },
    outbound::ExposedDatabase,
};
use std::str::FromStr;

impl From<sqlx::Error> for RankingsError {
    fn from(error: sqlx::Error) -> Self {
        Self(error.to_string())
    }
}

impl RankingsRepo for ExposedDatabase {
    async fn ranking_declarations(
        &self,
        start: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<RankingDeclaration>, RankingsError> {
        let start = time::OffsetDateTime::from_unix_timestamp(start.timestamp())
            .map_err(|error| RankingsError(error.to_string()))?;
        let rows = sqlx::query!(
            "SELECT m.parliament_member_id, m.name, m.party_id, m.party_name, m.latest_membership_from,
                d.source_declaration_id, d.category_id, d.category_name, d.registration_date,
                e.id AS \"entry_id?\", e.amount, NULLIF(btrim(e.currency, $2), '') AS currency,
                f.id AS \"funder_id?\", COALESCE(n.preferred_name, f.funder_name) AS funder_name, f.funder_kind
             FROM exposed.declarations d
             JOIN exposed.members m ON m.id = d.member_id
             LEFT JOIN exposed.funding_entries e ON e.source_declaration_id = d.source_declaration_id
             LEFT JOIN exposed.funder_alias_groups g ON g.funder_id = e.funder_id
             LEFT JOIN exposed.funders f ON f.id = COALESCE(g.group_id, e.funder_id)
             LEFT JOIN exposed.funder_group_names n ON n.group_id = f.id
             WHERE d.registration_date >= $1
             ORDER BY d.source_declaration_id, e.id",
            start, CURRENCY_WHITESPACE
        ).fetch_all(self.pool()).await?;
        let mut declarations: Vec<RankingDeclaration> = Vec::new();
        for row in rows {
            if declarations
                .last()
                .is_none_or(|source| source.declaration.source_id != row.source_declaration_id)
            {
                declarations.push(RankingDeclaration {
                    member: RankedMember {
                        id: row.parliament_member_id.to_string().parse().map_err(
                            |error: crate::domain::models::parliament_member::MemberIdError| {
                                RankingsError(error.to_string())
                            },
                        )?,
                        name: row.name,
                        party_name: row.party_name.clone(),
                        membership_from: row.latest_membership_from,
                    },
                    party: RankedParty {
                        id: row.party_id,
                        name: row.party_name,
                    },
                    declaration: MemberDeclaration {
                        source_id: row.source_declaration_id,
                        category: DeclarationCategory::from_source_id(row.category_id),
                        category_name: row.category_name,
                        registered_at: row
                            .registration_date
                            .map(|date| {
                                chrono::DateTime::from_timestamp(
                                    date.unix_timestamp(),
                                    date.nanosecond(),
                                )
                                .ok_or_else(|| {
                                    RankingsError("Stored registration date is out of range".into())
                                })
                            })
                            .transpose()?,
                        entries: Vec::new(),
                    },
                    funders: std::collections::BTreeMap::new(),
                });
            }
            if row.entry_id.is_some() {
                let funder = row
                    .funder_id
                    .zip(row.funder_name)
                    .map(|(id, name)| {
                        let kind = row
                            .funder_kind
                            .as_deref()
                            .map_or(Ok(FunderKind::NotSpecified), FunderKind::from_str)
                            .map_err(|error| RankingsError(error.to_string()))?;
                        Ok::<_, RankingsError>(RankedFunder {
                            id: FunderId::new(id),
                            name,
                            kind,
                        })
                    })
                    .transpose()?;
                if let Some(source) = declarations.last_mut() {
                    source.declaration.entries.push(DeclaredFunding {
                        funder: funder.as_ref().map(|funder| FunderReference {
                            id: funder.id,
                            name: funder.name.clone(),
                        }),
                        payment_type: None,
                        amount: row.amount,
                        currency: row.currency,
                    });
                    if let Some(funder) = funder {
                        source.funders.insert(funder.id.value(), funder);
                    }
                }
            }
        }
        Ok(declarations)
    }
}

#[cfg(test)]
mod tests;
