//! CLI inbound port

use std::{fmt::Write, path::PathBuf, str::FromStr};

use clap::{Parser, Subcommand};

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

pub use response::{IntoResponse, Response};

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
    Latest {
        /// YAML configuration with the ingestion `data_dir`.
        #[arg(long)]
        config: PathBuf,
    },
    /// Copy the latest raw capture into a new ingestion run.
    CopyLatestRaw {
        /// YAML configuration with the ingestion `data_dir`.
        #[arg(long)]
        config: PathBuf,
        /// Destination ingestion key. Defaults to a fresh UUID-v7.
        #[arg(long)]
        ingestion_key: Option<IngestionKey>,
    },
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
        } => declarations(stage, config, ingestion_key)
            .await
            .into_response(),
        DataCommands::Members {
            stage,
            config,
            ingestion_key,
        } => members(stage, config, ingestion_key).await.into_response(),
    }
}

async fn latest(config: PathBuf) -> anyhow::Result<String> {
    let config = LoaderConfig::try_from(&config)?;
    match ExposedDataPipeline::latest_ingestion(&config.data_dir).await? {
        Some((path, key)) => Ok(format!("Ingestion key: {key}\nPath: {}\n", path.display())),
        None => Ok("No ingestion runs found.\n".to_owned()),
    }
}

async fn copy_latest_raw(
    config: PathBuf,
    ingestion_key: Option<IngestionKey>,
) -> anyhow::Result<String> {
    let config = LoaderConfig::try_from(&config)?;
    let key = ingestion_key.unwrap_or_default();
    match ExposedDataPipeline::copy_latest_raw(&config.data_dir, key.clone()).await? {
        Some((path, source_key)) => Ok(format!(
            "Source ingestion key: {source_key}\nIngestion key: {key}\nPath: {}\n",
            path.display()
        )),
        None => Ok("No ingestion runs found.\n".to_owned()),
    }
}

async fn declarations(
    stage: String,
    config: PathBuf,
    ingestion_key: Option<IngestionKey>,
) -> anyhow::Result<String> {
    match DeclarationIngestionStage::from_str(&stage)? {
        DeclarationIngestionStage::Fetch => {
            fetch_declarations(config, ingestion_key).await?;
            Ok(String::new())
        }
        DeclarationIngestionStage::Resolve => resolve_declarations(config, ingestion_key).await,
        DeclarationIngestionStage::Load => load_declarations(config, ingestion_key).await,
        DeclarationIngestionStage::Clean => clean_declarations(config, ingestion_key).await,
    }
}

async fn fetch_declarations(
    config: PathBuf,
    ingestion_key: Option<IngestionKey>,
) -> anyhow::Result<()> {
    let config = DeclarationFetcherConfig::try_from(&config)?;
    let data_config = DataConfig::from_env()?;
    let api = ParliamentApiClient::new(config.batch_size)?;
    let db = ExposedDatabase::new(&data_config.connection_string).await;
    let fs = ExposedDataPipeline::new_with_ingestion_key(
        &config.data_dir,
        ingestion_key.unwrap_or_default(),
    )?;
    DeclarationFetcherService::new(db, api, fs)
        .fetch_declarations()
        .await?;

    Ok(())
}

async fn resolve_declarations(
    config: PathBuf,
    ingestion_key: Option<IngestionKey>,
) -> anyhow::Result<String> {
    let mut output = String::new();

    let ingestion_key = ingestion_key
        .ok_or_else(|| anyhow::anyhow!("declarations resolve requires --ingestion-key"))?;
    let config = DeclarationResolverConfig::try_from(&config)?;
    let storage =
        ExposedDataPipeline::open_cleaned_declarations(&config.data_dir, ingestion_key.clone())?;
    let scorer = WeldrsFunderScorer::new()?;
    let summary = DeclarationResolverService::new(storage, scorer, config.candidate_budget)
        .resolve_declarations()
        .await?;
    writeln!(output, "Ingestion key: {ingestion_key}")?;
    writeln!(
        output,
        "Resolved declarations: {}",
        std::path::absolute(config.data_dir)?
            .join(ingestion_key.to_string())
            .join("resolved/declarations")
            .display()
    )?;
    writeln!(output, "Observations: {}", summary.observations)?;
    writeln!(output, "Funding occurrences: {}", summary.payments)?;
    writeln!(output, "Pair decisions: {}", summary.pairs)?;

    Ok(output)
}

async fn load_declarations(
    config: PathBuf,
    ingestion_key: Option<IngestionKey>,
) -> anyhow::Result<String> {
    let mut output = String::new();

    let ingestion_key = ingestion_key
        .ok_or_else(|| anyhow::anyhow!("declarations load requires --ingestion-key"))?;
    let config = LoaderConfig::try_from(&config)?;
    let data_config = DataConfig::from_env()?;
    let storage =
        ExposedDataPipeline::open_resolved_declarations(&config.data_dir, ingestion_key.clone())?;
    let db = ExposedDatabase::new(&data_config.connection_string).await;
    let summary = DeclarationLoaderService::new(storage, db)
        .load_declarations()
        .await?;
    writeln!(output, "Ingestion key: {ingestion_key}")?;
    match summary.outcome {
        DeclarationLoadOutcome::Imported => {
            writeln!(output, "Loaded declarations: {}", summary.declarations)?;
        }
        DeclarationLoadOutcome::AlreadyLoaded => {
            writeln!(
                output,
                "Run already loaded. Declarations: {}",
                summary.declarations
            )?;
        }
    }
    writeln!(output, "Resolved identities: {}", summary.funders)?;
    writeln!(output, "Funding occurrences: {}", summary.funding_entries)?;

    Ok(output)
}

async fn clean_declarations(
    config: PathBuf,
    ingestion_key: Option<IngestionKey>,
) -> anyhow::Result<String> {
    let mut output = String::new();

    let ingestion_key = ingestion_key
        .ok_or_else(|| anyhow::anyhow!("declarations clean requires --ingestion-key"))?;
    let config = LoaderConfig::try_from(&config)?;
    let fs =
        ExposedDataPipeline::open_existing_declarations(&config.data_dir, ingestion_key.clone())?;
    let summary = DeclarationCleanerService::new(fs)
        .clean_declarations()
        .await?;
    writeln!(output, "Ingestion key: {ingestion_key}")?;
    writeln!(
        output,
        "Cleaned declarations: {}",
        std::path::absolute(&config.data_dir)?
            .join(ingestion_key.to_string())
            .join("cleaned/declarations")
            .display()
    )?;
    writeln!(output, "Member partitions: {}", summary.member_partitions)?;
    writeln!(output, "Declarations: {}", summary.declarations)?;
    writeln!(output, "Funding entries: {}", summary.funding_entries)?;
    writeln!(output, "Funder observations: {}", summary.funders)?;

    Ok(output)
}

async fn members(
    stage: String,
    config: PathBuf,
    ingestion_key: Option<IngestionKey>,
) -> anyhow::Result<()> {
    let stage = MemberIngestionStage::from_str(&stage)?;
    let ingestion_key = ingestion_key.unwrap_or_default();
    match stage {
        MemberIngestionStage::Fetch => {
            let config = FetcherConfig::try_from(&config)?;
            let api = ParliamentApiClient::new(config.batch_size)?;
            let fs = ExposedDataPipeline::new_with_ingestion_key(&config.data_dir, ingestion_key)?;
            let service = FetcherService::new(fs, api);

            let req = EntityIngestionRequest {};
            service.fetch_members(&req).await?;
        }
        MemberIngestionStage::Load => {
            let config = LoaderConfig::try_from(&config)?;
            let data_config = DataConfig::from_env()?;
            let fs = ExposedDataPipeline::new_with_ingestion_key(&config.data_dir, ingestion_key)?;
            let db = ExposedDatabase::new(&data_config.connection_string).await;
            let service = IngesterService::new(fs, db);

            service.load_members().await?;
        }
    }
    Ok(())
}

