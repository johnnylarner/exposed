//! General purpose postgres client

use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::{str::FromStr, time::Duration};

/// Postgres client that wraps [`sqlx::Pool`]
#[derive(Clone)]
pub struct ExposedDatabase {
    pool: PgPool,
}

impl ExposedDatabase {
    /// Configure the shared database adapter without opening a connection.
    ///
    /// # Errors
    /// Rejects an empty or invalid PostgreSQL connection string.
    pub fn new_lazy(conn_str: &str) -> Result<Self, sqlx::Error> {
        if conn_str.trim().is_empty() {
            return Err(sqlx::Error::Configuration(
                "connection_string must not be empty".into(),
            ));
        }
        let options = PgConnectOptions::from_str(conn_str)?.application_name("exposed");
        let pool = PgPoolOptions::new()
            .acquire_timeout(Duration::from_secs(10))
            .connect_lazy_with(options);
        Ok(Self { pool })
    }

    /// Creates a new instance of [`ExposedDatabase`]
    ///
    /// # Panics
    /// Invalid connection string
    #[must_use]
    pub async fn new(conn_str: &str) -> Self {
        let pool = PgPool::connect(conn_str).await.unwrap();

        Self { pool }
    }

    /// Get a reference to the underlying pool
    #[must_use]
    pub const fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[cfg(test)]
impl From<PgPool> for ExposedDatabase {
    fn from(value: PgPool) -> Self {
        Self { pool: value }
    }
}

#[cfg(test)]
mod test_scaffolding {
    use sqlx::PgPool;

    #[sqlx::test(migrations = "../db/migrations")]
    async fn migrations_work(pool: PgPool) -> sqlx::Result<()> {
        let res = sqlx::query!("SELECT name FROM members")
            .fetch_all(&pool)
            .await;
        assert!(res.is_ok());
        Ok(())
    }

    #[sqlx::test(
        migrations = "../db/migrations",
        fixtures(
            "../../../../db/fixtures/add_members.sql",
            "../../../../db/fixtures/add_declarations_and_funding_entries.sql"
        )
    )]
    async fn fixtures_work(pool: PgPool) -> sqlx::Result<()> {
        let res = sqlx::query!("SELECT name FROM members")
            .fetch_all(&pool)
            .await?;
        assert_eq!(res.len(), 2);

        let res = sqlx::query!("SELECT * FROM declarations")
            .fetch_all(&pool)
            .await?;
        assert_eq!(res.len(), 2);

        let res = sqlx::query!("SELECT * FROM funders")
            .fetch_all(&pool)
            .await?;
        assert_eq!(res.len(), 2);

        let res = sqlx::query!("SELECT * FROM funding_entries")
            .fetch_all(&pool)
            .await?;
        assert_eq!(res.len(), 2);

        Ok(())
    }
}
