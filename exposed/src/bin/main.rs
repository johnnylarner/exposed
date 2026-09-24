use std::path::PathBuf;

use clap::Parser;
use exposed::{config::Config, inbound::http::server::serve_exposed};

#[derive(Parser)]
struct Cli {
    config: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let config = Config::try_from(&args.config)?;

    let _ = serve_exposed(&config).await?;
    Ok(())
}
