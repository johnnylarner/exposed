//! General purpose postgres client

use sqlx::PgPool;

/// Postgres client that wraps [`sqlx::Pool`]
#[derive(Clone)]
pub struct ExposedDatabase {
    pool: PgPool,
}

impl ExposedDatabase {
    /// Creates a new instance of [`ExposedDatabase`]
    ///
    /// # Errors
    /// Returns an error if the connection string is invalid or connecting fails.
    pub async fn new(conn_str: &str) -> sqlx::Result<Self> {
        let pool = PgPool::connect(conn_str).await?;

        Ok(Self { pool })
    }

    /// Get a reference to the underlying pool
    #[must_use]
    pub const fn pool(&self) -> &PgPool {
        &self.pool
    }
}

pub(super) fn word_similarity_candidate_threshold(strictness: f32) -> String {
    if strictness == 0.0 {
        return "0".into();
    }
    // pg_trgm's operator compares a double score before word_similarity rounds to real.
    f64::from(strictness)
        .midpoint(f64::from(strictness.next_down()))
        .to_string()
}

#[cfg(test)]
impl From<PgPool> for ExposedDatabase {
    fn from(value: PgPool) -> Self {
        Self { pool: value }
    }
}
