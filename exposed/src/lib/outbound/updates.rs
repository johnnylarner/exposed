use super::ExposedDatabase;
use crate::domain::{
    models::{
        funder::{FunderId, FunderKind},
        updates::{RegistrationMonth, UpdateDeclaration, UpdateFunder, UpdatedMember},
    },
    repositories::updates::{UpdatesError, UpdatesRepo},
};
use std::str::FromStr;
impl From<sqlx::Error> for UpdatesError {
    fn from(error: sqlx::Error) -> Self {
        Self(error.to_string())
    }
}
impl UpdatesRepo for ExposedDatabase {
    async fn declarations(
        &self,
        month: &RegistrationMonth,
    ) -> Result<Vec<UpdateDeclaration>, UpdatesError> {
        let start = time::OffsetDateTime::from_unix_timestamp(month.start().timestamp())
            .map_err(|error| UpdatesError(error.to_string()))?;
        let end = time::OffsetDateTime::from_unix_timestamp(month.end().timestamp())
            .map_err(|error| UpdatesError(error.to_string()))?;
        let rows = sqlx::query!(
            "SELECT m.parliament_member_id, m.name, d.source_declaration_id,
                d.registration_date AS \"registration_date!\",
                f.id AS \"funder_id?\", COALESCE(n.preferred_name, f.funder_name) AS funder_name, f.funder_kind
             FROM exposed.declarations d
             JOIN exposed.members m ON m.id = d.member_id
             LEFT JOIN exposed.funding_entries e ON e.source_declaration_id = d.source_declaration_id
             LEFT JOIN exposed.funder_alias_groups g ON g.funder_id = e.funder_id
             LEFT JOIN exposed.funders f ON f.id = COALESCE(g.group_id, e.funder_id)
             LEFT JOIN exposed.funder_group_names n ON n.group_id = f.id
             WHERE d.registration_date >= $1 AND d.registration_date < $2
             ORDER BY d.source_declaration_id, e.id",
            start, end
        ).fetch_all(self.pool()).await?;
        let mut declarations: Vec<UpdateDeclaration> = Vec::new();
        for row in rows {
            if declarations
                .last()
                .is_none_or(|source| source.source_id != row.source_declaration_id)
            {
                declarations.push(UpdateDeclaration {
                    source_id: row.source_declaration_id,
                    registered_at: chrono::DateTime::from_timestamp(
                        row.registration_date.unix_timestamp(),
                        row.registration_date.nanosecond(),
                    )
                    .ok_or_else(|| {
                        UpdatesError("Stored registration date is out of range".into())
                    })?,
                    member: UpdatedMember {
                        id: row.parliament_member_id.to_string().parse().map_err(
                            |error: crate::domain::models::parliament_member::MemberIdError| {
                                UpdatesError(error.to_string())
                            },
                        )?,
                        name: row.name,
                    },
                    funders: Vec::new(),
                });
            }
            if let Some((id, name)) = row.funder_id.zip(row.funder_name) {
                let kind = row
                    .funder_kind
                    .as_deref()
                    .map_or(Ok(FunderKind::NotSpecified), FunderKind::from_str)
                    .map_err(|error| UpdatesError(error.to_string()))?;
                if let Some(declaration) = declarations.last_mut() {
                    declaration.funders.push(UpdateFunder {
                        id: FunderId::new(id),
                        name,
                        kind,
                    });
                }
            }
        }
        Ok(declarations)
    }
}

#[cfg(test)]
mod tests;
