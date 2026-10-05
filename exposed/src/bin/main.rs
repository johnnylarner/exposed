use clap::{Parser, Subcommand};
use exposed::inbound::{
    cli::{DataArgs, run_cli},
    http::server::run_server,
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

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    match args.command {
        Commands::Server => run_server(),
        Commands::Data(args) => tokio::runtime::Runtime::new()?.block_on(run_cli(args)),
    }
}
