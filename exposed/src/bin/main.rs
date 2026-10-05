use clap::{Parser, Subcommand};
use exposed::inbound::{
    cli::{DataArgs, run_cli},
    http::{config::ServerConfig, server::serve_exposed},
};
use tokio::net::TcpListener;

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

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    if matches!(args.command, Commands::Server) {
        dotenvy::dotenv().ok();
    }
    run(args)
}

#[tokio::main]
async fn run(args: Cli) -> anyhow::Result<()> {
    match args.command {
        Commands::Server => {
            let config = ServerConfig::from_env()?;
            let listener = match TcpListener::bind("0.0.0.0:6999").await {
                Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
                    TcpListener::bind("0.0.0.0:0").await?
                }
                result => result?,
            };
            println!(
                "Listening on http://localhost:{}",
                listener.local_addr()?.port()
            );
            serve_exposed(&config, listener).await?;
        }
        Commands::Data(args) => run_cli(args).await?,
    }

    Ok(())
}
