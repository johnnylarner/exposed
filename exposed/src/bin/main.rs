use clap::Parser;
use exposed::inbound::{
    cli::{Commands, ExposedCli, run_cli},
    http::server::run_server,
};

fn main() -> anyhow::Result<()> {
    let args = ExposedCli::parse();
    match args.command {
        Commands::Server => run_server(),
        Commands::Data(args) => {
            dotenvy::dotenv().ok();
            tokio::runtime::Runtime::new()?.block_on(run_cli(args))
        }
    }
}
