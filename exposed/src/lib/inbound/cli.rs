//! Command-line inbound adapter, including configuration and service construction.

use crate::{
    config::{Config, MemberFetchConfig, MemberLoadConfig, read_data_config},
    domain::{
        models::{
            entity_ingestion::{
                EntityIngestionOutcome, EntityIngestionRequest, MemberIngestionStage,
            },
            member_ingestion::CaptureId,
        },
        services::entity_ingestion::{
            FetchService, LoadService, interface::EntitySearchIngestionService,
        },
    },
    inbound::http::server::serve_exposed,
    outbound::{ExposedDataPipeline, MemberDatabase, MembersApi},
};
use anyhow::{Context, ensure};
use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use std::{path::PathBuf, process::ExitCode};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    Server { config: PathBuf },
    Data(DataArgs),
}
#[derive(Args)]
struct DataArgs {
    #[command(subcommand)]
    command: DataCommands,
}
#[derive(Subcommand)]
enum DataCommands {
    Fetch {
        #[command(subcommand)]
        target: FetchTarget,
    },
    Load {
        #[command(subcommand)]
        target: LoadTarget,
    },
    Clean {
        config: PathBuf,
    },
    Resolve {
        config: PathBuf,
    },
}
#[derive(Subcommand)]
enum FetchTarget {
    Members { config: PathBuf },
}
#[derive(Subcommand)]
enum LoadTarget {
    Members {
        config: PathBuf,
        capture_id: CaptureId,
    },
}

/// Run the CLI using process arguments and return its exit status.
///
/// Owns command dispatch, service construction, output and interruption handling.
pub async fn run() -> ExitCode {
    let args = Cli::parse();
    // Dropping the operation on Ctrl-C drops an in-flight transaction. Capture
    // publication is synchronous once source acquisition has finished.
    tokio::select! {
        result = dispatch(args) => match result {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                let code = if error.is::<InvalidConfiguration>() { 2 } else { 1 };
                eprintln!("{error:#}");
                ExitCode::from(code)
            }
        },
        _ = tokio::signal::ctrl_c() => { eprintln!("Interrupted"); ExitCode::from(130) }
    }
}
async fn dispatch(args: Cli) -> anyhow::Result<()> {
    let outcome: EntityIngestionOutcome = match args.command {
        Commands::Server { config } => {
            return serve_exposed(&Config::try_from(&config)?).await;
        }
        Commands::Data(DataArgs {
            command:
                DataCommands::Fetch {
                    target: FetchTarget::Members { config },
                },
        }) => {
            let started_at = Utc::now();
            let observation_date = started_at
                .with_timezone(&chrono_tz::Europe::London)
                .date_naive();
            let config: MemberFetchConfig = read_data_config(&config)
                .context(InvalidConfiguration("invalid member Fetch configuration"))?;
            ensure!(
                !config.data_root.as_os_str().is_empty(),
                InvalidConfiguration("data_root must not be empty")
            );
            let source = MembersApi::new(&config.members_api_url)
                .context(InvalidConfiguration("invalid Members API configuration"))?;
            let storage = ExposedDataPipeline::new(config.data_root);
            FetchService::new(storage, source, started_at, observation_date)
                .run_ingestion(&EntityIngestionRequest::new_members_request(
                    MemberIngestionStage::Fetch {
                        term_start: config.term_start,
                    },
                ))
                .await?
        }
        Commands::Data(DataArgs {
            command:
                DataCommands::Load {
                    target: LoadTarget::Members { config, capture_id },
                },
        }) => {
            let config: MemberLoadConfig = read_data_config(&config)
                .context(InvalidConfiguration("invalid member Load configuration"))?;
            ensure!(
                !config.data_root.as_os_str().is_empty(),
                InvalidConfiguration("data_root must not be empty")
            );
            let storage = ExposedDataPipeline::new(config.data_root);
            let database = MemberDatabase::new(&config.connection_string).context(
                InvalidConfiguration("invalid member database configuration"),
            )?;
            LoadService::new(storage, database)
                .run_ingestion(&EntityIngestionRequest::new_members_request(
                    MemberIngestionStage::Load { capture_id },
                ))
                .await?
        }
        Commands::Data(_) => {
            anyhow::bail!("declaration cleaning and resolution are not implemented")
        }
    };
    let mut summary = serde_json::to_value(outcome)?;
    summary["status"] = "succeeded".into();
    println!("{}", serde_json::to_string(&summary)?);
    Ok(())
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct InvalidConfiguration(&'static str);
