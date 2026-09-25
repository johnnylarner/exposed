//! General purpose postgres client

use sqlx::PgPool;

/// Postgres client that wraps [sqlx::Pool]
#[derive(Clone)]
pub struct ExposedDatabase {
    pool: PgPool,
}

impl ExposedDatabase {
    /// Creates a new instance of [ExposedDatabase]
    pub async fn new(conn_str: &str) -> Self {
        let pool = PgPool::connect(conn_str).await.unwrap();

        Self { pool }
    }

    /// Get a reference to the underlying pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

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
