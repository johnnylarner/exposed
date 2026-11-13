use std::path::PathBuf;

use schemars::JsonSchema;
use serde::Serialize;

use super::{
    CliSuccess, IntoResponse, Response,
    config::{DataConfig, LoaderConfig},
    response_help,
    stages::{StageArgs, StageInput},
};
use crate::{
    domain::{
        models::{
            entity_ingestion::IngestionKey,
            lords::{LordsIngestionStage, LordsLoadOutcome},
        },
        services::lords::{LordsFetcher, LordsLoader},
    },
    outbound::{ExposedDatabase, LordsApiClient, LordsCaptureFiles},
};

/// Top-level Lords register commands.
pub type LordsArgs = StageArgs<LordsIngestionStage>;

impl LordsArgs {
    /// Returns schemas without configuration, files, network, or a database.
    #[must_use]
    pub fn schema_response(&self) -> Option<Response> {
        matches!(self.input, StageInput::Schema).then(|| {
            Response::schema(&match self.stage {
                LordsIngestionStage::Fetch => response_help::inspection(fetch),
                LordsIngestionStage::Load => response_help::inspection(load),
            })
        })
    }
}

/// Runs a Lords capture or load with the standard structured CLI response.
pub async fn run_lords(args: LordsArgs) -> Response {
    if let Some(response) = args.schema_response() {
        return response;
    }
    let (config, key) = args.input.into_execution();
    match args.stage {
        LordsIngestionStage::Fetch => fetch(config, key).await.into_response(),
        LordsIngestionStage::Load => load(config, key).await.into_response(),
    }
}

#[derive(Serialize, JsonSchema)]
struct FetchResponse {
    ingestion_key: IngestionKey,
    /// Absolute directory containing the complete manifest and source pages.
    path: PathBuf,
    members: usize,
    pages: usize,
}

#[derive(Serialize, JsonSchema)]
struct LoadResponse {
    ingestion_key: IngestionKey,
    outcome: LordsLoadOutcome,
    members: usize,
}

async fn fetch(config: PathBuf, key: IngestionKey) -> anyhow::Result<CliSuccess<FetchResponse>> {
    let config = LoaderConfig::try_from(&config)?;
    let storage = LordsCaptureFiles::new(&config.data_dir, key.clone())?;
    let path = storage.path().to_owned();
    let capture = LordsFetcher::new(LordsApiClient::new()?, storage)
        .fetch(key.clone())
        .await?;
    Ok(CliSuccess::new(FetchResponse {
        ingestion_key: key,
        path,
        members: capture.members().len(),
        pages: capture.pages().len(),
    }))
}

async fn load(config: PathBuf, key: IngestionKey) -> anyhow::Result<CliSuccess<LoadResponse>> {
    let config = LoaderConfig::try_from(&config)?;
    let data = DataConfig::from_env()?;
    let storage = LordsCaptureFiles::new(&config.data_dir, key.clone())?;
    let database = ExposedDatabase::new(&data.connection_string).await?;
    let (outcome, members) = LordsLoader::new(storage, database).load().await?;
    Ok(CliSuccess::new(LoadResponse {
        ingestion_key: key,
        outcome,
        members,
    }))
}
