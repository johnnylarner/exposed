//! CLI inbound port

use schemars::JsonSchema;
use std::{path::PathBuf, str::FromStr};

use clap::{Parser, Subcommand};
use serde::Serialize;

use crate::{
    domain::{
        models::declaration_loading::DeclarationLoadOutcome,
        models::entity_ingestion::{
            DeclarationIngestionStage, EntityIngestionRequest, IngestionKey, MemberIngestionStage,
        },
        services::declaration_cleaning::DeclarationCleanerService,
        services::declaration_fetch::DeclarationFetcherService,
        services::declaration_loading::DeclarationLoaderService,
        services::declaration_resolution::DeclarationResolverService,
        services::entity_ingestion::{
            EntityFetcherService, EntityIngesterService, FetcherService, IngesterService,
        },
    },
    inbound::cli::config::{
        DataConfig, DeclarationFetcherConfig, DeclarationResolverConfig, FetcherConfig,
        LoaderConfig,
    },
    outbound::{ExposedDataPipeline, ExposedDatabase, ParliamentApiClient, WeldrsFunderScorer},
};

pub mod config;
pub mod response;
mod response_help;

pub use response::{CliSuccess, IntoResponse, Response};

/// CLI entrypoint for the `exposed` project
#[derive(Parser)]
pub struct ExposedCli {
    /// Commands for CLI
    #[command(subcommand)]
    pub command: Commands,
}

/// Run a server or prepare application data
#[derive(Subcommand)]
pub enum Commands {
    /// Start an instance of the `exposed` server
    Server,
    /// Options to run the `exposed` data pipeline
    Data(DataArgs),
}

/// CLI Data Args
#[derive(Parser)]
pub struct DataArgs {
    #[command(subcommand)]
    command: DataCommands,
}

/// CLI Subcommands
#[derive(Subcommand)]
#[allow(missing_docs)]
pub enum DataCommands {
    /// Show the latest ingestion key and directory path.
    #[command(after_long_help = response_help::single(latest))]
    Latest {
        /// YAML configuration with the ingestion `data_dir`.
        #[arg(long)]
        config: PathBuf,
    },
    /// Copy the latest raw capture into a new ingestion run.
    #[command(after_long_help = response_help::single(copy_latest_raw))]
    CopyLatestRaw {
        /// YAML configuration with the ingestion `data_dir`.
        #[arg(long)]
        config: PathBuf,
        /// Destination ingestion key. Defaults to a fresh UUID-v7.
        #[arg(long)]
        ingestion_key: Option<IngestionKey>,
    },
    #[command(after_long_help = response_help::declarations())]
    Declarations {
        /// Stage to run: fetch, clean, resolve, or load.
        #[arg(value_parser = ["fetch", "clean", "resolve", "load"])]
        stage: String,
        /// YAML configuration containing the declaration `data_dir`.
        #[arg(long)]
        config: PathBuf,
        /// Ingestion run to clean, resolve, or load.
        #[arg(long)]
        ingestion_key: Option<IngestionKey>,
    },
    #[command(after_long_help = response_help::single(members))]
    Members {
        stage: String,
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        ingestion_key: Option<IngestionKey>,
    },
}

/// Entry point for CLI commands
///
/// Command failures remain in the response until [`Response::write_to`] is called.
///
/// Failures include:
/// - Bad stage arg
/// - Bad config path
/// - Bad config shape
/// - Missing or non-Unicode `DATABASE_URL` for commands that use the database
/// - Service level failures
pub async fn run_cli(args: DataArgs) -> Response {
    match args.command {
        DataCommands::Latest { config } => latest(config).await.into_response(),
        DataCommands::CopyLatestRaw {
            config,
            ingestion_key,
        } => copy_latest_raw(config, ingestion_key).await.into_response(),
        DataCommands::Declarations {
            stage,
            config,
            ingestion_key,
        } => declarations(stage, config, ingestion_key).await,
        DataCommands::Members {
            stage,
            config,
            ingestion_key,
        } => members(stage, config, ingestion_key).await.into_response(),
    }
}

#[derive(Serialize, JsonSchema)]
struct IngestionResponse<T> {
    /// Ingestion result, or null when the store has no source runs.
    ingestion: Option<T>,
}

#[derive(Serialize, JsonSchema)]
struct IngestionLocation {
    ingestion_key: IngestionKey,
    /// Absolute ingestion run directory.
    path: PathBuf,
}

#[derive(Serialize, JsonSchema)]
struct CopiedIngestion {
    /// Run whose raw capture was copied.
    source_ingestion_key: IngestionKey,
    ingestion_key: IngestionKey,
    /// Absolute destination ingestion run directory.
    path: PathBuf,
}

#[derive(Serialize, JsonSchema)]
struct StageAcknowledgement<S> {
    ingestion_key: IngestionKey,
    stage: S,
    /// Absolute raw artifact: raw/declarations directory or raw/members.parquet file.
    path: PathBuf,
}

#[derive(Serialize, JsonSchema)]
struct DeclarationCleaningResponse {
    ingestion_key: IngestionKey,
    /// Absolute cleaned/declarations artifact directory.
    path: PathBuf,
    /// Raw member partitions, including empty partitions.
    member_partitions: usize,
    /// Declarations, including those with no funding.
    declarations: usize,
    /// Original funding occurrences.
    funding_entries: usize,
    /// Source-role funder observations across declaration and funding scopes.
    funders: usize,
}

#[derive(Serialize, JsonSchema)]
struct DeclarationResolutionResponse {
    ingestion_key: IngestionKey,
    /// Absolute resolved/declarations artifact directory.
    path: PathBuf,
    /// Source funder observations, including unresolved names.
    observations: usize,
    /// Original funding occurrences, counted once.
    payments: usize,
    /// Retained pair decisions between funder observations.
    pairs: usize,
}

#[derive(Serialize, JsonSchema)]
struct DeclarationLoadingResponse {
    ingestion_key: IngestionKey,
    /// Whether this call imported the run or confirmed an exact retry.
    outcome: DeclarationLoadOutcome,
    declarations: usize,
    /// Distinct resolved identities referenced by selected attributions.
    funders: usize,
    /// Original funding occurrences, including unavailable attributions.
    funding_entries: usize,
}

async fn latest(
    config: PathBuf,
) -> anyhow::Result<CliSuccess<IngestionResponse<IngestionLocation>>> {
    let config = LoaderConfig::try_from(&config)?;
    let ingestion = ExposedDataPipeline::latest_ingestion(&config.data_dir)
        .await?
        .map(|(path, ingestion_key)| IngestionLocation {
            ingestion_key,
            path,
        });
    Ok(CliSuccess::new(IngestionResponse { ingestion }))
}

async fn copy_latest_raw(
    config: PathBuf,
    ingestion_key: Option<IngestionKey>,
) -> anyhow::Result<CliSuccess<IngestionResponse<CopiedIngestion>>> {
    let config = LoaderConfig::try_from(&config)?;
    let ingestion_key = ingestion_key.unwrap_or_default();
    let ingestion = ExposedDataPipeline::copy_latest_raw(&config.data_dir, ingestion_key.clone())
        .await?
        .map(|(path, source_ingestion_key)| CopiedIngestion {
            source_ingestion_key,
            ingestion_key,
            path,
        });
    Ok(CliSuccess::new(IngestionResponse { ingestion }))
}

async fn declarations(
    stage: String,
    config: PathBuf,
    ingestion_key: Option<IngestionKey>,
) -> Response {
    let stage = match DeclarationIngestionStage::from_str(&stage) {
        Ok(stage) => stage,
        Err(error) => return Err::<Response, _>(error).into_response(),
    };
    match stage {
        DeclarationIngestionStage::Fetch => fetch_declarations(config, ingestion_key)
            .await
            .into_response(),
        DeclarationIngestionStage::Clean => clean_declarations(config, ingestion_key)
            .await
            .into_response(),
        DeclarationIngestionStage::Resolve => resolve_declarations(config, ingestion_key)
            .await
            .into_response(),
        DeclarationIngestionStage::Load => load_declarations(config, ingestion_key)
            .await
            .into_response(),
    }
}

async fn fetch_declarations(
    config: PathBuf,
    ingestion_key: Option<IngestionKey>,
) -> anyhow::Result<CliSuccess<StageAcknowledgement<DeclarationIngestionStage>>> {
    let config = DeclarationFetcherConfig::try_from(&config)?;
    let data_config = DataConfig::from_env()?;
    let api = ParliamentApiClient::new(config.batch_size)?;
    let db = ExposedDatabase::new(&data_config.connection_string).await?;
    let ingestion_key = ingestion_key.unwrap_or_default();
    let fs = ExposedDataPipeline::new_with_ingestion_key(&config.data_dir, ingestion_key.clone())?;
    DeclarationFetcherService::new(db, api, fs)
        .fetch_declarations()
        .await?;
    Ok(CliSuccess::new(StageAcknowledgement {
        path: std::path::absolute(config.data_dir)?
            .join(ingestion_key.to_string())
            .join("raw/declarations"),
        ingestion_key,
        stage: DeclarationIngestionStage::Fetch,
    }))
}

async fn resolve_declarations(
    config: PathBuf,
    ingestion_key: Option<IngestionKey>,
) -> anyhow::Result<CliSuccess<DeclarationResolutionResponse>> {
    let ingestion_key = ingestion_key
        .ok_or_else(|| anyhow::anyhow!("declarations resolve requires --ingestion-key"))?;
    let config = DeclarationResolverConfig::try_from(&config)?;
    let storage =
        ExposedDataPipeline::open_cleaned_declarations(&config.data_dir, ingestion_key.clone())?;
    let scorer = WeldrsFunderScorer::new()?;
    let summary = DeclarationResolverService::new(storage, scorer, config.candidate_budget)
        .resolve_declarations()
        .await?;
    Ok(CliSuccess::new(DeclarationResolutionResponse {
        path: std::path::absolute(config.data_dir)?
            .join(ingestion_key.to_string())
            .join("resolved/declarations"),
        ingestion_key,
        observations: summary.observations,
        payments: summary.payments,
        pairs: summary.pairs,
    }))
}

async fn load_declarations(
    config: PathBuf,
    ingestion_key: Option<IngestionKey>,
) -> anyhow::Result<CliSuccess<DeclarationLoadingResponse>> {
    let ingestion_key = ingestion_key
        .ok_or_else(|| anyhow::anyhow!("declarations load requires --ingestion-key"))?;
    let config = LoaderConfig::try_from(&config)?;
    let data_config = DataConfig::from_env()?;
    let storage =
        ExposedDataPipeline::open_resolved_declarations(&config.data_dir, ingestion_key.clone())?;
    let db = ExposedDatabase::new(&data_config.connection_string).await?;
    let summary = DeclarationLoaderService::new(storage, db)
        .load_declarations()
        .await?;
    Ok(CliSuccess::new(DeclarationLoadingResponse {
        ingestion_key,
        outcome: summary.outcome,
        declarations: summary.declarations,
        funders: summary.funders,
        funding_entries: summary.funding_entries,
    }))
}

async fn clean_declarations(
    config: PathBuf,
    ingestion_key: Option<IngestionKey>,
) -> anyhow::Result<CliSuccess<DeclarationCleaningResponse>> {
    let ingestion_key = ingestion_key
        .ok_or_else(|| anyhow::anyhow!("declarations clean requires --ingestion-key"))?;
    let config = LoaderConfig::try_from(&config)?;
    let fs =
        ExposedDataPipeline::open_existing_declarations(&config.data_dir, ingestion_key.clone())?;
    let summary = DeclarationCleanerService::new(fs)
        .clean_declarations()
        .await?;
    Ok(CliSuccess::new(DeclarationCleaningResponse {
        path: std::path::absolute(&config.data_dir)?
            .join(ingestion_key.to_string())
            .join("cleaned/declarations"),
        ingestion_key,
        member_partitions: summary.member_partitions,
        declarations: summary.declarations,
        funding_entries: summary.funding_entries,
        funders: summary.funders,
    }))
}

async fn members(
    stage: String,
    config: PathBuf,
    ingestion_key: Option<IngestionKey>,
) -> anyhow::Result<CliSuccess<StageAcknowledgement<MemberIngestionStage>>> {
    let stage = MemberIngestionStage::from_str(&stage)?;
    let ingestion_key = ingestion_key.unwrap_or_default();
    let data_dir = match stage {
        MemberIngestionStage::Fetch => {
            let config = FetcherConfig::try_from(&config)?;
            let api = ParliamentApiClient::new(config.batch_size)?;
            let fs = ExposedDataPipeline::new_with_ingestion_key(
                &config.data_dir,
                ingestion_key.clone(),
            )?;
            FetcherService::new(fs, api)
                .fetch_members(&EntityIngestionRequest {})
                .await?;
            config.data_dir
        }
        MemberIngestionStage::Load => {
            let config = LoaderConfig::try_from(&config)?;
            let data_config = DataConfig::from_env()?;
            let fs = ExposedDataPipeline::new_with_ingestion_key(
                &config.data_dir,
                ingestion_key.clone(),
            )?;
            let db = ExposedDatabase::new(&data_config.connection_string).await?;
            IngesterService::new(fs, db).load_members().await?;
            config.data_dir
        }
    };
    Ok(CliSuccess::new(StageAcknowledgement {
        path: std::path::absolute(data_dir)?
            .join(ingestion_key.to_string())
            .join("raw/members.parquet"),
        ingestion_key,
        stage,
    }))
}
