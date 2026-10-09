use crate::domain::{
    models::{
        declaration_resolution::{ComparisonRow, ScoredPair, ScoredPairs, ScoringInput},
        entity_ingestion::EntityIngestionError,
    },
    repositories::declaration_resolution::FunderScorer,
};
use duckdb::{Connection, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

const MODEL: &str = include_str!("../../../resources/funder-resolution/model.json");
const ARTIFACT: &str = include_str!("../../../resources/funder-resolution/artifact.json");
const SCHEMA: &str = include_str!("../../../resources/funder-resolution/schema.sql");
const COUNT: &str = include_str!("../../../resources/funder-resolution/count.sql");
const CONCAT: &str = include_str!("../../../resources/funder-resolution/concat.sql");
const CANDIDATES: &str = include_str!("../../../resources/funder-resolution/candidates.sql");
const PREDICT: &str = include_str!("../../../resources/funder-resolution/predict.sql");
const OUTPUT: &str = include_str!("../../../resources/funder-resolution/output.sql");

/// Executes the bundled frozen Splink program in a private embedded `DuckDB` connection.
pub struct DuckDbFunderScorer {
    model: Arc<FrozenModel>,
}

impl DuckDbFunderScorer {
    /// Loads the embedded model and verifies its generated SQL digests.
    ///
    /// # Errors
    /// Returns an error when the bundled model and generated program disagree.
    pub fn new() -> Result<Self, EntityIngestionError> {
        Ok(Self {
            model: Arc::new(FrozenModel::bundled()?),
        })
    }
}

impl FunderScorer for DuckDbFunderScorer {
    async fn score(&self, input: &ScoringInput) -> Result<ScoredPairs, EntityIngestionError> {
        let model = self.model.clone();
        let rows = input.rows.clone();
        let budget = input.candidate_budget;
        let (pairs, metadata) = tokio::task::spawn_blocking(move || {
            let (pairs, version) = model.execute(&rows, budget)?;
            Ok::<_, EntityIngestionError>((pairs, model.provenance(version)?))
        })
        .await
        .map_err(scoring_error)??;
        ScoredPairs::checked(pairs, metadata, input)
    }
}

struct FrozenModel {
    parameters: ModelParameters,
    artifact: GeneratedArtifact,
}

#[derive(Deserialize)]
struct ModelParameters {
    model_version: String,
    splink_version: String,
    duckdb_version: String,
    calibrated: bool,
    settings: serde_json::Value,
}

#[derive(Deserialize)]
struct GeneratedArtifact {
    format_version: u32,
    model_sha256: String,
    sql_sha256: String,
    generator: Generator,
}

#[derive(Deserialize, Serialize)]
struct Generator {
    splink_version: String,
    duckdb_version: String,
}

#[derive(Serialize)]
struct ModelProvenance<'a> {
    backend: &'static str,
    model_version: &'a str,
    duckdb_version: String,
    calibrated: bool,
    settings: &'a serde_json::Value,
    generator: &'a Generator,
    model_sha256: &'a str,
    sql_sha256: &'a str,
}

impl FrozenModel {
    fn bundled() -> Result<Self, EntityIngestionError> {
        let parameters: ModelParameters = serde_json::from_str(MODEL).map_err(scoring_error)?;
        let artifact: GeneratedArtifact = serde_json::from_str(ARTIFACT).map_err(scoring_error)?;
        let mut sql_digest = Sha256::new();
        for (index, sql) in [SCHEMA, COUNT, CONCAT, CANDIDATES, PREDICT, OUTPUT]
            .into_iter()
            .enumerate()
        {
            if index > 0 {
                sql_digest.update(b"\0");
            }
            sql_digest.update(sql.as_bytes());
        }
        if artifact.format_version != 1
            || artifact.model_sha256 != format!("{:x}", Sha256::digest(MODEL.as_bytes()))
            || artifact.sql_sha256 != format!("{:x}", sql_digest.finalize())
            || artifact.generator.splink_version != parameters.splink_version
            || artifact.generator.duckdb_version != parameters.duckdb_version
        {
            return Err(scoring_error(
                "frozen model or SQL changed; regenerate with resolution/generate.py",
            ));
        }
        Ok(Self {
            parameters,
            artifact,
        })
    }

    fn execute(
        &self,
        rows: &[ComparisonRow],
        budget: usize,
    ) -> Result<(Vec<ScoredPair>, String), EntityIngestionError> {
        let connection = Connection::open_in_memory().map_err(scoring_error)?;
        let version = connection.version().map_err(scoring_error)?;
        let version = version.strip_prefix('v').unwrap_or(&version).to_owned();
        if version != self.parameters.duckdb_version {
            return Err(scoring_error(format!(
                "frozen SQL requires DuckDB {}; found {version}",
                self.parameters.duckdb_version
            )));
        }
        connection.execute_batch(SCHEMA).map_err(scoring_error)?;
        {
            let mut profiles = connection
                .prepare("INSERT INTO profile_rows VALUES (?, ?, ?, ?)")
                .map_err(scoring_error)?;
            let mut keys = connection
                .prepare("INSERT INTO profile_blocking_keys VALUES (?, ?, ?)")
                .map_err(scoring_error)?;
            for row in rows {
                profiles
                    .execute(params![
                        row.key,
                        row.name,
                        row.address,
                        row.needs_resolution
                    ])
                    .map_err(scoring_error)?;
                for (ordinal, key) in row.blocking_keys.iter().enumerate() {
                    keys.execute(params![row.key, ordinal, key])
                        .map_err(scoring_error)?;
                }
            }
        }
        let count: usize = connection
            .query_row(COUNT, [], |row| row.get(0))
            .map_err(scoring_error)?;
        if count > budget {
            return Err(scoring_error(format!(
                "candidate budget {budget} exceeded by {count} profile pairs; raise candidate_budget explicitly"
            )));
        }
        let mut pairs = Vec::new();
        if count > 0 {
            for sql in [CONCAT, CANDIDATES, PREDICT] {
                connection.execute_batch(sql).map_err(scoring_error)?;
            }
            let mut statement = connection.prepare(OUTPUT).map_err(scoring_error)?;
            let scored = statement
                .query_map([], |row| {
                    Ok(ScoredPair {
                        left: row.get(0)?,
                        right: row.get(1)?,
                        probability: row.get(2)?,
                        name_level: row.get(3)?,
                        address_level: row.get(4)?,
                    })
                })
                .map_err(scoring_error)?;
            for pair in scored {
                pairs.push(pair.map_err(scoring_error)?);
            }
        }
        if pairs.len() != count {
            return Err(scoring_error(
                "frozen SQL did not return every generated candidate",
            ));
        }
        Ok((pairs, version))
    }

    fn provenance(&self, version: String) -> Result<serde_json::Value, EntityIngestionError> {
        serde_json::to_value(ModelProvenance {
            backend: "duckdb",
            model_version: &self.parameters.model_version,
            duckdb_version: version,
            calibrated: self.parameters.calibrated,
            settings: &self.parameters.settings,
            generator: &self.artifact.generator,
            model_sha256: &self.artifact.model_sha256,
            sql_sha256: &self.artifact.sql_sha256,
        })
        .map_err(scoring_error)
    }
}

fn scoring_error(error: impl std::fmt::Display) -> EntityIngestionError {
    EntityIngestionError::DataError(format!("DuckDB funder scoring failed: {error}"))
}

#[cfg(test)]
mod tests;
