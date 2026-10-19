use std::path::PathBuf;

use clap::{Parser, Subcommand};
use schemars::JsonSchema;
use serde::Serialize;
use sqlx::{Connection, PgConnection, migrate::Migrator};

use super::{CliSuccess, IntoResponse, Response, config::DataConfig, response_help};

/// Arguments for explicit database migrations.
#[derive(Parser)]
pub struct MigrateArgs {
    #[command(subcommand)]
    command: MigrateCommand,
}

#[derive(Subcommand)]
enum MigrateCommand {
    /// Apply pending up migrations.
    #[command(after_long_help = response_help::single(run))]
    Run {
        /// Migration directory, relative to the current working directory.
        #[arg(long, default_value = "db/migrations")]
        source: PathBuf,
    },
}

/// Runs the selected migration command through the CLI response contract.
pub(super) async fn run_migrate(args: MigrateArgs) -> Response {
    match args.command {
        MigrateCommand::Run { source } => run(source).await.into_response(),
    }
}

#[derive(Serialize, JsonSchema)]
struct MigrationResponse {
    /// Supplied migration directory.
    source: PathBuf,
}

async fn run(source: PathBuf) -> anyhow::Result<CliSuccess<MigrationResponse>> {
    let config = DataConfig::from_env()?;
    let mut migrator = Migrator::new(source.as_path()).await?;
    migrator.dangerous_set_table_name("public._sqlx_migrations");
    let mut connection = PgConnection::connect(&config.connection_string).await?;
    migrator.run(&mut connection).await?;
    Ok(CliSuccess::new(MigrationResponse { source }))
}
