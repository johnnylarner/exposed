use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use exposed::{config::Config, inbound::http::server::serve_exposed};

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
    Fetch { config: PathBuf },
    Clean { config: PathBuf },
    Resolve { config: PathBuf },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Server { config } => {
            let config = Config::try_from(&config)?;
            let _ = serve_exposed(&config).await?;
        }
        _ => panic!("only server commands supported"),
    }

    Ok(())
}
