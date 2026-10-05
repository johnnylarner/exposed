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
    Server,
    Data(DataArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Server => {
            let config = ServerConfig::from_env()?;
            serve_exposed(&config).await?;
        }
        Commands::Data(args) => run_cli(args).await?,
    }

    Ok(())
}
