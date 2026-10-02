use std::path::PathBuf;

use clap::{Parser, Subcommand};
use exposed::inbound::{
    cli::{DataArgs, run_cli},
    http::{config::ServerConfig, server::serve_exposed},
};

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Server { config } => {
            let config = ServerConfig::try_from(&config)?;
            serve_exposed(&config).await?;
        }
        Commands::Data(args) => run_cli(args).await?,
    }

    Ok(())
}
