use crate::{
    domain::{
        models::{
            entity_details::{
                DeclarationCategory, DeclaredFunding, FunderAliasLink, FunderDeclaration,
                FunderFamilyLink, FunderFunding, FunderProfile, FunderReference, MemberDeclaration,
                MemberHistory, MemberProfile,
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
    async fn member(&self, id: MemberId) -> Result<Option<MemberHistory>, EntityDetailsRepoError> {
        let mut tx = self.pool().begin().await?;
        sqlx::query!("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *tx)
            .await?;
        let row = sqlx::query!(
            "SELECT parliament_member_id, name, party_id, party_name, latest_membership_from, is_current_commons
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
            "SELECT d.source_declaration_id, d.category_id, d.category_name, d.registration_date,
                e.id AS \"entry_id?\", e.amount, NULLIF(btrim(e.currency, $2), '') AS currency,
                e.payment_type, f.id AS \"funder_id?\", f.funder_name AS \"funder_name?\"
             FROM exposed.declarations d JOIN exposed.members m ON m.id = d.member_id
             LEFT JOIN exposed.funding_entries e ON e.source_declaration_id = d.source_declaration_id
             LEFT JOIN exposed.funders f ON f.id = e.funder_id
             WHERE m.parliament_member_id = $1
             ORDER BY d.registration_date DESC NULLS LAST, d.source_declaration_id DESC, e.id",
             i32::try_from(id.value()).map_err(|error| EntityDetailsRepoError::DatabaseError(error.to_string()))?,
             CURRENCY_WHITESPACE
        ).fetch_all(&mut *tx).await?;
        let mut declarations: Vec<MemberDeclaration> = Vec::new();
        for row in rows {
            if declarations
                .last()
                .is_none_or(|declaration| declaration.source_id != row.source_declaration_id)
            {
                declarations.push(MemberDeclaration {
                    source_id: row.source_declaration_id,
                    category: DeclarationCategory::from_source_id(row.category_id),
                    category_name: row.category_name,
                    registered_at: row.registration_date.map(registration_date).transpose()?,
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
        Ok(Some(MemberHistory {
            member,
            declarations,
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
        let group_id = sqlx::query_scalar!(
            "SELECT COALESCE(g.group_id, f.id) AS \"group_id!\" FROM exposed.funders f
             LEFT JOIN exposed.funder_alias_groups g ON g.funder_id = f.id WHERE f.id = $1",
            id.value()
        )
        .fetch_optional(&mut *tx)
        .await?;
        let Some(group_id) = group_id else {
            return Ok(None);
        };
        let stored_member_ids = sqlx::query_scalar!(
            "SELECT funder_id AS \"funder_id!\" FROM exposed.funder_alias_groups WHERE group_id = $1
             UNION SELECT $1::uuid ORDER BY 1",
            group_id
        )
        .fetch_all(&mut *tx)
        .await?;
        let row = sqlx::query!(
            "SELECT COALESCE(n.preferred_name, f.funder_name) AS \"funder_name!\", f.funder_kind,
                (SELECT CASE WHEN count(DISTINCT member.company_number) = 1
                    THEN min(member.company_number) END FROM exposed.funders member
                 WHERE member.id = ANY($2)) AS company_number
             FROM exposed.funders f LEFT JOIN exposed.funder_group_names n ON n.group_id = f.id WHERE f.id = $1",
            group_id,
            &stored_member_ids
        )
        .fetch_one(&mut *tx)
        .await?;
        let kind = row
            .funder_kind
            .as_deref()
            .map_or(Ok(FunderKind::NotSpecified), FunderKind::from_str)
            .map_err(|error| EntityDetailsRepoError::DatabaseError(error.to_string()))?;
        let aliases = sqlx::query_scalar!(
            "WITH names AS (
                SELECT funder_id, funder_alias FROM exposed.funder_aliases
                UNION SELECT id, funder_name FROM exposed.funders
             )
             SELECT DISTINCT funder_alias COLLATE \"C\" AS \"funder_alias!\" FROM names
             WHERE funder_id = ANY($1)
             ORDER BY funder_alias COLLATE \"C\"",
            &stored_member_ids
        )
        .fetch_all(&mut *tx)
        .await?;
        let member_ids = stored_member_ids
            .iter()
            .copied()
            .map(FunderId::new)
            .collect();
        let alias_links = sqlx::query!(
            "SELECT links.left_funder_id AS \"left_funder_id!\", links.right_funder_id AS \"right_funder_id!\",
                links.normalized_alias AS \"normalized_alias!\", links.left_alias AS \"left_alias!\", links.right_alias AS \"right_alias!\"
             FROM exposed.funder_alias_links links
             WHERE links.left_funder_id = ANY($1)
             ORDER BY links.left_funder_id, links.right_funder_id, links.normalized_alias COLLATE \"C\",
                links.left_alias COLLATE \"C\", links.right_alias COLLATE \"C\"", &stored_member_ids
        ).fetch_all(&mut *tx).await?.into_iter().map(|link| FunderAliasLink {
            left_funder_id: FunderId::new(link.left_funder_id),
            right_funder_id: FunderId::new(link.right_funder_id),
            normalized_alias: link.normalized_alias,
            left_alias: link.left_alias,
            right_alias: link.right_alias,
        }).collect();
        let family_links = sqlx::query!(
            "SELECT links.left_funder_id AS \"left_funder_id!\", links.right_funder_id AS \"right_funder_id!\",
                links.family_key AS \"family_key!\", links.left_name AS \"left_name!\", links.right_name AS \"right_name!\"
             FROM exposed.funder_family_links links
             WHERE links.left_funder_id = ANY($1)
             ORDER BY links.family_key COLLATE \"C\", links.left_funder_id, links.right_funder_id",
            &stored_member_ids
        ).fetch_all(&mut *tx).await?.into_iter().map(|link| FunderFamilyLink {
            left_funder_id: FunderId::new(link.left_funder_id),
            right_funder_id: FunderId::new(link.right_funder_id),
            family_key: link.family_key,
            left_name: link.left_name,
            right_name: link.right_name,
        }).collect();
        let profile = FunderProfile {
            funder: Funder::new(id, row.funder_name, kind),
            company_number: row.company_number,
            aliases,
            member_ids,
            alias_links,
            family_links,
        };
        let rows = sqlx::query!(
            "SELECT m.parliament_member_id, m.name, m.party_id, m.party_name,
                m.latest_membership_from, m.is_current_commons,
                d.source_declaration_id, d.category_id, d.category_name, d.registration_date,
                e.amount, NULLIF(btrim(e.currency, $2), '') AS currency, e.payment_type
             FROM exposed.funding_entries e
             JOIN exposed.declarations d ON d.source_declaration_id = e.source_declaration_id
             JOIN exposed.members m ON m.id = d.member_id
             WHERE e.funder_id = ANY($1)
             ORDER BY d.registration_date DESC NULLS LAST, d.source_declaration_id DESC, e.id",
            &stored_member_ids,
            CURRENCY_WHITESPACE
        )
        .fetch_all(&mut *tx)
        .await?;
        let mut declarations: Vec<FunderDeclaration> = Vec::new();
        for row in rows {
            if declarations
                .last()
                .is_none_or(|source| source.declaration.source_id != row.source_declaration_id)
            {
                declarations.push(FunderDeclaration {
                    member: MemberProfile {
                        id: row
                            .parliament_member_id
                            .to_string()
                            .parse::<MemberId>()
                            .map_err(|error| {
                                EntityDetailsRepoError::DatabaseError(error.to_string())
                            })?,
                        name: row.name,
                        party_id: row.party_id,
                        party_name: row.party_name,
                        membership_from: row.latest_membership_from,
                        is_current_commons: row.is_current_commons,
                    },
                    declaration: MemberDeclaration {
                        source_id: row.source_declaration_id,
                        category: DeclarationCategory::from_source_id(row.category_id),
                        category_name: row.category_name,
                        registered_at: row.registration_date.map(registration_date).transpose()?,
                        entries: Vec::new(),
                    },
                });
            }
            if let Some(source) = declarations.last_mut() {
                source.declaration.entries.push(DeclaredFunding {
                    funder: Some(FunderReference {
                        id,
                        name: profile.funder.name().to_owned(),
                    }),
                    amount: row.amount,
                    currency: row.currency,
                    payment_type: row.payment_type,
                });
            }
        }
        tx.commit().await?;
        Ok(Some(FunderFunding {
            profile,
            declarations,
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

fn registration_date(
    date: time::OffsetDateTime,
) -> Result<chrono::DateTime<chrono::Utc>, EntityDetailsRepoError> {
    chrono::DateTime::from_timestamp(date.unix_timestamp(), date.nanosecond()).ok_or_else(|| {
        EntityDetailsRepoError::DatabaseError("Stored registration date is out of range".into())
    })
}
