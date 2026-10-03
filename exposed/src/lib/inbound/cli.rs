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
    inbound::cli::config::{DeclarationFetcherConfig, FetcherConfig, LoaderConfig},
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
/// - Service level failures
pub async fn run_cli(args: DataArgs) -> anyhow::Result<()> {
    match args.command {
        DataCommands::Declarations {
            stage,
            config,
            ingestion_key,
        } => {
            let DeclarationIngestionStage::Fetch = DeclarationIngestionStage::from_str(&stage)?;
            let config = DeclarationFetcherConfig::try_from(&config)?;
            let api = ParliamentApiClient::new(config.batch_size)?;
            let pool = sqlx::PgPool::connect(&config.connection_string).await?;
            let db = ExposedDatabase::from(pool);
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
                    let fs = ExposedDataPipeline::new_with_ingestion_key(
                        &config.data_dir,
                        ingestion_key,
                    )?;
                    let db = ExposedDatabase::new(&config.connection_string).await;
                    let service = IngesterService::new(fs, db);

                    service.load_members().await?;
                }
            }
        }
    }
    Ok(())
}
