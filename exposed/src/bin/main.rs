use std::path::PathBuf;

use clap::Parser;
use exposed::{application, config::Config};

#[derive(Parser)]
struct Cli {
    config: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let config = Config::try_from(&args.config)?;

    tokio::select! {
        result = application::serve(&config) => result,
        signal = tokio::signal::ctrl_c() => { signal?; Ok(()) }
    }
}
