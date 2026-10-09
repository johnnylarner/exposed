use crate::domain::{
    models::{
        declaration_resolution::{ComparisonRow, ScoredPair, ScoredPairs, ScoringInput},
        entity_ingestion::EntityIngestionError,
    },
    repositories::declaration_resolution::FunderScorer,
};
use polars::prelude::*;
use serde::Serialize;
use std::sync::Arc;
use weldrs::{
    comparison::{Comparison, ComparisonBuilder},
    comparison_vectors::compute_comparison_vectors,
    predict::predict_direct,
};

mod blocking;
#[cfg(test)]
mod tests;

use blocking::{Candidate, CandidateRule, bounded_candidates};

/// Native scoring adapter for bounded funder profile candidates.
pub struct WeldrsFunderScorer {
    model: Arc<NativeModel>,
}

#[derive(Serialize)]
struct NativeModel {
    version: u32,
    prior: f64,
    comparisons: [Comparison; 2],
    blocking: [CandidateRule; 5],
    missing_address: MissingAddressPolicy,
}
#[derive(Serialize)]
enum MissingAddressPolicy {
    EitherMissingIsNeutral,
}
#[derive(Serialize)]
enum DistanceEngine {
    WeldrsSimdUtf8Bytes,
}
#[derive(Serialize)]
struct NativeProvenance<'a> {
    model: &'a NativeModel,
    model_version: &'static str,
    weldrs_version: &'static str,
    runtime: NativeRuntime,
    calibrated: bool,
}
#[derive(Serialize)]
struct NativeRuntime {
    library: &'static str,
    adapter_version: u32,
    distance_engine: DistanceEngine,
}

fn error(message: impl std::fmt::Display) -> EntityIngestionError {
    EntityIngestionError::DataError(format!("native funder scorer: {message}"))
}

impl WeldrsFunderScorer {
    /// Constructs the immutable frozen funder model.
    ///
    /// # Errors
    /// Returns an error if comparison construction or frozen parameters are invalid.
    pub fn new() -> Result<Self, EntityIngestionError> {
        let mut name = ComparisonBuilder::new("name")
            .null_level()
            .exact_match_level()
            .levenshtein_level(2)
            .else_level()
            .build()
            .map_err(error)?;
        let mut address = ComparisonBuilder::new("address")
            .null_level()
            .exact_match_level()
            .else_level()
            .build()
            .map_err(error)?;
        set_parameters(&mut name, &[(0.95, 0.001), (0.04, 0.009), (0.01, 0.99)])?;
        set_parameters(&mut address, &[(0.9, 0.00001), (0.1, 0.99999)])?;
        Ok(Self {
            model: Arc::new(NativeModel {
                version: 2,
                prior: 0.0001,
                comparisons: [name, address],
                blocking: CandidateRule::ALL,
                missing_address: MissingAddressPolicy::EitherMissingIsNeutral,
            }),
        })
    }
}

fn set_parameters(
    comparison: &mut Comparison,
    parameters: &[(f64, f64)],
) -> Result<(), EntityIngestionError> {
    let levels = comparison.non_null_levels_mut();
    if levels.len() != parameters.len() {
        return Err(error("incomplete comparison parameters"));
    }
    for (level, &(m, u)) in levels.into_iter().zip(parameters) {
        if [m, u]
            .into_iter()
            .any(|p| !p.is_finite() || p <= 0.0 || p >= 1.0)
        {
            return Err(error("invalid comparison probability"));
        }
        level.m_probability = Some(m);
        level.u_probability = Some(u);
    }
    Ok(())
}

impl FunderScorer for WeldrsFunderScorer {
    async fn score(&self, input: &ScoringInput) -> Result<ScoredPairs, EntityIngestionError> {
        let rows = input.rows.clone();
        let budget = input.candidate_budget;
        let model = Arc::clone(&self.model);
        let pairs = tokio::task::spawn_blocking(move || infer(rows, budget, &model))
            .await
            .map_err(error)??;
        let provenance = serde_json::to_value(NativeProvenance {
            model: &self.model,
            model_version: "funder-frozen-v2",
            weldrs_version: "0.2.2",
            runtime: NativeRuntime {
                library: "weldrs",
                adapter_version: 1,
                distance_engine: DistanceEngine::WeldrsSimdUtf8Bytes,
            },
            calibrated: false,
        })
        .map_err(error)?;
        ScoredPairs::checked(pairs, provenance, input)
    }
}

fn infer(
    mut rows: Vec<ComparisonRow>,
    budget: usize,
    model: &NativeModel,
) -> Result<Vec<ScoredPair>, EntityIngestionError> {
    rows.sort_by(|a, b| a.key.cmp(&b.key));
    let candidates = bounded_candidates(&rows, &model.blocking, budget)?;
    let mut pairs = Vec::new();
    let mut batch = Vec::with_capacity(4096);
    for candidate in candidates {
        batch.push(candidate);
        if batch.len() == 4096 {
            pairs.extend(score_batch(&rows, &batch, model)?);
            batch.clear();
        }
    }
    if !batch.is_empty() {
        pairs.extend(score_batch(&rows, &batch, model)?);
    }
    Ok(pairs)
}

fn score_batch(
    rows: &[ComparisonRow],
    batch: &[Candidate],
    model: &NativeModel,
) -> Result<Vec<ScoredPair>, EntityIngestionError> {
    let left = batch
        .iter()
        .map(|pair| &rows[pair.left])
        .collect::<Vec<_>>();
    let right = batch
        .iter()
        .map(|pair| &rows[pair.right])
        .collect::<Vec<_>>();
    let frame = df!(
        "key_l" => left.iter().map(|r| r.key.as_str()).collect::<Vec<_>>(),
        "key_r" => right.iter().map(|r| r.key.as_str()).collect::<Vec<_>>(),
        "name_l" => left.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
        "name_r" => right.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
        "address_l" => left.iter().map(|r| r.address.as_deref()).collect::<Vec<_>>(),
        "address_r" => right.iter().map(|r| r.address.as_deref()).collect::<Vec<_>>()
    )
    .map_err(error)?;
    let vectors =
        compute_comparison_vectors(frame.lazy(), &model.comparisons, "gamma_").map_err(error)?;
    let vectors = match model.missing_address {
        MissingAddressPolicy::EitherMissingIsNeutral => vectors.with_column(
            when(col("address_l").is_null().or(col("address_r").is_null()))
                .then(lit(-1i8))
                .otherwise(col("gamma_address"))
                .alias("gamma_address"),
        ),
    };
    let output = predict_direct(
        vectors.collect().map_err(error)?,
        &model.comparisons,
        model.prior,
        "gamma_",
        "bf_",
        None,
        None,
    )
    .map_err(error)?;
    if output.height() != batch.len() {
        return Err(error("native prediction changed candidate count"));
    }
    let keys_l = output
        .column("key_l")
        .map_err(error)?
        .str()
        .map_err(error)?;
    let keys_r = output
        .column("key_r")
        .map_err(error)?
        .str()
        .map_err(error)?;
    let probabilities = output
        .column("match_probability")
        .map_err(error)?
        .f64()
        .map_err(error)?;
    let names = output
        .column("gamma_name")
        .map_err(error)?
        .i8()
        .map_err(error)?;
    let addresses = output
        .column("gamma_address")
        .map_err(error)?
        .i8()
        .map_err(error)?;
    batch
        .iter()
        .enumerate()
        .map(|(i, pair)| {
            if keys_l.get(i) != Some(rows[pair.left].key.as_str())
                || keys_r.get(i) != Some(rows[pair.right].key.as_str())
            {
                return Err(error("native prediction changed candidate endpoints"));
            }
            Ok(ScoredPair {
                left: rows[pair.left].key.clone(),
                right: rows[pair.right].key.clone(),
                probability: probabilities
                    .get(i)
                    .ok_or_else(|| error("missing prediction probability"))?,
                name_level: i32::from(
                    names
                        .get(i)
                        .ok_or_else(|| error("missing name comparison"))?,
                ),
                address_level: i32::from(
                    addresses
                        .get(i)
                        .ok_or_else(|| error("missing address comparison"))?,
                ),
            })
        })
        .collect()
}
