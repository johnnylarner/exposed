//! CLI inbound port

use std::{path::PathBuf, str::FromStr};

use clap::{Args, Subcommand};

use crate::{
    domain::{
        models::entity_ingestion::{EntityIngestionRequest, MemberIngestionStage},
        services::entity_ingestion::{EntityFetcherService, Service},
    },
    inbound::cli::config::FetcherConfig,
    outbound::{ExposedDataPipeline, ParliamentApiClient},
};

pub mod config;

/// CLI Data Args
#[derive(Args)]
pub struct DataArgs {
    #[command(subcommand)]
    command: DataCommands,
}

/// CLI Subcommands
#[derive(Subcommand)]
#[allow(missing_docs)]
pub enum DataCommands {
    Members { stage: String, config: PathBuf },
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
        DataCommands::Members { stage, config } => {
            let stage = MemberIngestionStage::from_str(&stage)?;
            match stage {
                MemberIngestionStage::Fetch => {
                    let config = FetcherConfig::try_from(&config)?;
                    let api = ParliamentApiClient::new(config.batch_size)?;
                    let fs = ExposedDataPipeline::new(&config.key)?;
                    let service = Service::new(fs, api);

                    let req = EntityIngestionRequest {};
                    let ingestion_key = service.fetch_members(&req).await?.to_string();
                    print!("{ingestion_key}");
                }
            }
        }
    }
    Ok(())
}
