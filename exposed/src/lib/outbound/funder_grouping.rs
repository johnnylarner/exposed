//! Atomic projection of the pipeline's grouping policy into stored membership and evidence.

use sqlx::{Postgres, Transaction};

use crate::domain::models::{
    entity_ingestion::EntityIngestionError,
    funder::FunderId,
    funder_grouping::{GroupingFunder, build_funder_groups},
};

use super::ExposedDatabase;

impl ExposedDatabase {
    /// Rebuilds grouping for every persisted identity without replaying ingestion artifacts.
    ///
    /// # Errors
    /// Returns database or grouping-policy errors. A failed refresh preserves the previous projection.
    pub async fn refresh_funder_groups(&self) -> Result<usize, EntityIngestionError> {
        let mut tx = self.pool().begin().await.map_err(db_error)?;
        lock_declaration_loader(&mut tx).await?;
        let count = replace_funder_groups(&mut tx).await?;
        tx.commit().await.map_err(db_error)?;
        Ok(count)
    }
}

pub(super) async fn lock_declaration_loader(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<(), EntityIngestionError> {
    sqlx::query!("SELECT pg_advisory_xact_lock(hashtext('exposed.declaration-loader'))")
        .execute(&mut **tx)
        .await
        .map_err(db_error)?;
    Ok(())
}

pub(super) async fn replace_funder_groups(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<usize, EntityIngestionError> {
    let rows = sqlx::query!("SELECT id, funder_name FROM exposed.funders ORDER BY id")
        .fetch_all(&mut **tx)
        .await
        .map_err(db_error)?;
    let aliases = sqlx::query!("SELECT funder_id, funder_alias FROM exposed.funder_aliases ORDER BY funder_id, funder_alias")
        .fetch_all(&mut **tx).await.map_err(db_error)?;
    let mut by_id = std::collections::BTreeMap::<_, Vec<_>>::new();
    for alias in aliases {
        by_id
            .entry(alias.funder_id)
            .or_default()
            .push(alias.funder_alias);
    }
    let funders: Vec<_> = rows
        .into_iter()
        .map(|row| GroupingFunder {
            id: FunderId::new(row.id),
            name: row.funder_name,
            aliases: by_id.remove(&row.id).unwrap_or_default(),
        })
        .collect();
    let groups = build_funder_groups(&funders).map_err(db_error)?;
    sqlx::query!("DELETE FROM exposed.funder_alias_links")
        .execute(&mut **tx)
        .await
        .map_err(db_error)?;
    sqlx::query!("DELETE FROM exposed.funder_family_links")
        .execute(&mut **tx)
        .await
        .map_err(db_error)?;
    sqlx::query!("DELETE FROM exposed.funder_alias_groups")
        .execute(&mut **tx)
        .await
        .map_err(db_error)?;
    for (id, group) in &groups.memberships {
        sqlx::query!(
            "INSERT INTO exposed.funder_alias_groups (funder_id, group_id) VALUES ($1, $2)",
            id.value(),
            group.value()
        )
        .execute(&mut **tx)
        .await
        .map_err(db_error)?;
    }
    for link in groups.alias_links {
        sqlx::query!("INSERT INTO exposed.funder_alias_links (left_funder_id, right_funder_id, normalized_alias, left_alias, right_alias) VALUES ($1, $2, $3, $4, $5)", link.left_funder_id.value(), link.right_funder_id.value(), link.normalized_alias, link.left_alias, link.right_alias)
            .execute(&mut **tx).await.map_err(db_error)?;
    }
    for link in groups.family_links {
        sqlx::query!("INSERT INTO exposed.funder_family_links (left_funder_id, right_funder_id, family_key, left_name, right_name) VALUES ($1, $2, $3, $4, $5)", link.left_funder_id.value(), link.right_funder_id.value(), link.family_key, link.left_name, link.right_name)
            .execute(&mut **tx).await.map_err(db_error)?;
    }
    Ok(groups.memberships.len())
}

fn db_error(error: impl std::fmt::Display) -> EntityIngestionError {
    EntityIngestionError::DataError(error.to_string())
}
