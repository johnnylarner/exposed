//! CLI inbound port

use schemars::JsonSchema;
use std::path::PathBuf;

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
mod config_help;
mod healthcheck;
mod migrate;
pub mod response;
mod response_help;
mod stages;

pub use healthcheck::run_healthcheck;
pub use migrate::{MigrateArgs, run_migrate};
pub use response::{CliSuccess, IntoResponse, Response};

/// CLI entrypoint for the `exposed` project
#[derive(Parser)]
#[command(version)]
pub struct ExposedCli {
    /// Commands for CLI
    #[command(subcommand)]
    pub command: Commands,
}

/// Run a server, prepare application data, or apply database migrations
#[derive(Subcommand)]
pub enum Commands {
    /// Start an instance of the `exposed` server
    Server,
    /// Check whether the local HTTP server responds.
    Healthcheck,
    /// Options to run the `exposed` data pipeline
    Data(DataArgs),
    /// Apply database migrations explicitly.
    Migrate(MigrateArgs),
}

/// CLI Data Args
#[derive(Parser)]
pub struct DataArgs {
    #[command(subcommand)]
    command: DataCommands,
}

/// CLI utility arguments.
#[derive(Parser)]
pub struct UtilArgs {
    #[command(subcommand)]
    command: UtilCommands,
}

#[derive(Subcommand)]
enum UtilCommands {
    /// Generate a fresh UUID-v7.
    #[command(
        name = "uuid-v7",
        after_long_help = "Example:\n  exposed data util uuid-v7\n\nUse --schema to inspect success and failure JSON schemas without generating a UUID."
    )]
    UuidV7 {
        /// Print response schemas without running the command.
        #[arg(long)]
        schema: bool,
    },
    /// Show the latest ingestion key and directory path.
    #[command(
        after_long_help = "Example:\n  exposed data util latest --config config.yaml\n\nSelects the most recently modified UUID run directory. Equal timestamps use the greatest UUID. An existing empty store returns ingestion: null. A missing or unreadable store fails with its path.\n\nUse --schema to inspect success and failure JSON schemas without reading configuration or storage."
    )]
    Latest(StoreArgs),
    /// Copy the latest raw capture into a new ingestion run.
    #[command(
        after_long_help = "Example:\n  exposed data util copy-latest-raw --config config.yaml\n\nCopies the latest run's raw data into a fresh UUID-v7 run. Use --ingestion-key to select the destination UUID. An existing empty store returns ingestion: null. A missing or unreadable store fails with its path. The selected run must contain raw data.\n\nUse --schema to inspect success and failure JSON schemas without reading configuration or storage."
    )]
    CopyLatestRaw {
        #[command(flatten)]
        store: StoreArgs,
        /// Destination ingestion key. Defaults to a fresh UUID-v7.
        #[arg(long)]
        ingestion_key: Option<IngestionKey>,
    },
}

#[derive(clap::Args)]
struct StoreArgs {
    /// YAML configuration with the ingestion `data_dir`.
    #[arg(long, required_unless_present = "schema", long_help = config_help::store())]
    config: Option<PathBuf>,
    /// Print response schemas without reading configuration or storage.
    #[arg(long)]
    schema: bool,
}

impl StoreArgs {
    fn into_config(self) -> PathBuf {
        self.config
            .expect("execution requires config at the argument boundary")
    }
}

/// Data pipeline commands.
#[derive(Subcommand)]
pub enum DataCommands {
    /// Utilities for inspecting and preparing ingestion runs.
    Util(UtilArgs),
    /// Capture, clean, resolve, and load declarations, in that order.
    Declarations(stages::StageArgs<DeclarationIngestionStage>),
    /// Capture and load members, in that order.
    Members(stages::StageArgs<MemberIngestionStage>),
}

impl DataArgs {
    /// Returns response schemas without reading configuration or running a stage.
    #[must_use]
    pub fn schema_response(&self) -> Option<Response> {
        match &self.command {
            DataCommands::Util(args) => args.schema_response(),
            DataCommands::Declarations(args) => matches!(args.input, stages::StageInput::Schema)
                .then(|| Response::schema(&response_help::declaration(&args.stage))),
            DataCommands::Members(args) => matches!(args.input, stages::StageInput::Schema)
                .then(|| Response::schema(&response_help::member(&args.stage))),
        }
    }
}

impl UtilArgs {
    /// Returns response schemas without reading configuration or running a utility.
    #[must_use]
    pub fn schema_response(&self) -> Option<Response> {
        match &self.command {
            UtilCommands::UuidV7 { schema } => {
                schema.then(|| Response::schema(&response_help::inspection(uuid_v7)))
            }
            UtilCommands::Latest(store) => store
                .schema
                .then(|| Response::schema(&response_help::inspection(latest))),
            UtilCommands::CopyLatestRaw { store, .. } => store
                .schema
                .then(|| Response::schema(&response_help::inspection(copy_latest_raw))),
        }
    }
}

/// Entry point for CLI commands.
///
/// Command failures remain in the response until [`Response::write_to`] is called.
///
/// Failures include:
/// - Bad config path
/// - Bad config shape
/// - Missing or non-Unicode `DATABASE_URL` for commands that use the database
/// - Service level failures
pub async fn run_cli(args: DataArgs) -> Response {
    if let Some(response) = args.schema_response() {
        return response;
    }
    match args.command {
        DataCommands::Util(args) => run_util(args).await,
        DataCommands::Declarations(args) => {
            let (config, ingestion_key) = args.input.into_execution();
            match args.stage {
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
        DataCommands::Members(args) => {
            let (config, ingestion_key) = args.input.into_execution();
            members(args.stage, config, ingestion_key)
                .await
                .into_response()
        }
    }
}

async fn run_util(args: UtilArgs) -> Response {
    match args.command {
        UtilCommands::UuidV7 { .. } => uuid_v7().await.into_response(),
        UtilCommands::Latest(store) => latest(store.into_config()).await.into_response(),
        UtilCommands::CopyLatestRaw {
            store,
            ingestion_key,
        } => copy_latest_raw(store.into_config(), ingestion_key)
            .await
            .into_response(),
    }
}

#[derive(Serialize, JsonSchema)]
struct UuidResponse {
    uuid: uuid::Uuid,
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

async fn uuid_v7() -> anyhow::Result<CliSuccess<UuidResponse>> {
    Ok(CliSuccess::new(UuidResponse {
        uuid: uuid::Uuid::now_v7(),
    }))
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

async fn fetch_declarations(
    config: PathBuf,
    ingestion_key: IngestionKey,
) -> anyhow::Result<CliSuccess<StageAcknowledgement<DeclarationIngestionStage>>> {
    let config = DeclarationFetcherConfig::try_from(&config)?;
    let data_config = DataConfig::from_env()?;
    let api = ParliamentApiClient::new(config.batch_size)?;
    let db = ExposedDatabase::new(&data_config.connection_string).await?;
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
    ingestion_key: IngestionKey,
) -> anyhow::Result<CliSuccess<DeclarationResolutionResponse>> {
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
    ingestion_key: IngestionKey,
) -> anyhow::Result<CliSuccess<DeclarationLoadingResponse>> {
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
    ingestion_key: IngestionKey,
) -> anyhow::Result<CliSuccess<DeclarationCleaningResponse>> {
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
    stage: MemberIngestionStage,
    config: PathBuf,
    ingestion_key: IngestionKey,
) -> anyhow::Result<CliSuccess<StageAcknowledgement<MemberIngestionStage>>> {
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
            let fs = ExposedDataPipeline::open_existing_members(
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
