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

    let services = async {
        if let Some(imports) = &config.imports {
            tokio::try_join!(
                serve_exposed(&config),
                exposed::imports::serve(imports, &config.connection_string)
            )?;
        } else {
            serve_exposed(&config).await?;
        }
        Ok(())
    };
    tokio::select! {
        result = services => result,
        signal = tokio::signal::ctrl_c() => { signal?; Ok(()) }
    }
}
