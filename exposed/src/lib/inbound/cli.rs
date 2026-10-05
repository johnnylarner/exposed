//! CLI inbound port

use std::{path::PathBuf, str::FromStr};

use clap::{Parser, Subcommand};

use crate::{
    domain::{
        models::entity_ingestion::{
            DeclarationIngestionStage, EntityIngestionRequest, IngestionKey, MemberIngestionStage,
        },
        services::declaration_fetch::DeclarationFetcherService,
        services::entity_ingestion::{
            EntityFetcherService, EntityIngesterService, FetcherService, IngesterService,
        },
    },
    inbound::cli::config::{DataConfig, DeclarationFetcherConfig, FetcherConfig, LoaderConfig},
    outbound::{ExposedDataPipeline, ExposedDatabase, ParliamentApiClient},
};

pub mod config;

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
        config: PathBuf,
    },
    Declarations {
        stage: String,
        config: PathBuf,
        #[arg(long)]
        ingestion_key: Option<IngestionKey>,
    },
    Members {
        stage: String,
        config: PathBuf,
        #[arg(long)]
        ingestion_key: Option<IngestionKey>,
    },
}

/// Entry point for CLI commands
///
/// # Errors
/// - Bad stage arg
/// - Bad config path
/// - Bad config shape
/// - Missing or non-Unicode `DATABASE_URL` for commands that use the database
/// - Service level failures
pub async fn run_cli(args: DataArgs) -> anyhow::Result<()> {
    match args.command {
        DataCommands::Latest { config } => {
            let config = LoaderConfig::try_from(&config)?;
            match ExposedDataPipeline::latest_ingestion(&config.data_dir).await? {
                Some((path, key)) => {
                    println!("Ingestion key: {key}");
                    println!("Path: {}", path.display());
                }
                None => println!("No ingestion runs found."),
            }
        }
        DataCommands::Declarations {
            stage,
            config,
            ingestion_key,
        } => {
            let DeclarationIngestionStage::Fetch = DeclarationIngestionStage::from_str(&stage)?;
            let config = DeclarationFetcherConfig::try_from(&config)?;
            let data_config = DataConfig::from_env()?;
            let api = ParliamentApiClient::new(config.batch_size)?;
            let db = ExposedDatabase::new(&data_config.connection_string).await;
            let fs = ExposedDataPipeline::new_with_ingestion_key(
                &config.data_dir,
                ingestion_key.unwrap_or_default(),
            )?;
            let service = DeclarationFetcherService::new(db, api, fs);
            service.fetch_declarations().await?;
        }
        DataCommands::Members {
            stage,
            config,
            ingestion_key,
        } => {
            let stage = MemberIngestionStage::from_str(&stage)?;
            let ingestion_key = ingestion_key.unwrap_or_default();
            match stage {
                MemberIngestionStage::Fetch => {
                    let config = FetcherConfig::try_from(&config)?;
                    let api = ParliamentApiClient::new(config.batch_size)?;
                    let fs = ExposedDataPipeline::new_with_ingestion_key(
                        &config.data_dir,
                        ingestion_key,
                    )?;
                    let service = FetcherService::new(fs, api);

                    let req = EntityIngestionRequest {};
                    service.fetch_members(&req).await?;
                }
                MemberIngestionStage::Load => {
                    let config = LoaderConfig::try_from(&config)?;
                    let data_config = DataConfig::from_env()?;
                    let fs = ExposedDataPipeline::new_with_ingestion_key(
                        &config.data_dir,
                        ingestion_key,
                    )?;
                    let db = ExposedDatabase::new(&data_config.connection_string).await;
                    let service = IngesterService::new(fs, db);

                    service.load_members().await?;
                }
            }
        }
    }
    Ok(())
}
