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
