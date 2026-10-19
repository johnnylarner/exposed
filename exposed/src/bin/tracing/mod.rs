use std::{fs::OpenOptions, sync::Mutex};

use anyhow::Context;
use exposed::inbound::cli::Commands;
use tracing_subscriber::{EnvFilter, fmt::writer::BoxMakeWriter};

pub(super) fn init(command: &Commands) -> anyhow::Result<()> {
    let writer = match std::env::var_os("EXPOSED_LOG_FILE") {
        Some(path) => BoxMakeWriter::new(Mutex::new(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .context("opening EXPOSED_LOG_FILE")?,
        )),
        None => match command {
            Commands::Server => BoxMakeWriter::new(std::io::stderr),
            Commands::Healthcheck | Commands::Data(_) | Commands::Migrate(_) => return Ok(()),
        },
    };
    let filter =
        EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("off,exposed=info"))?;
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(filter)
        .with_writer(writer)
        .try_init()
        .map_err(|error| anyhow::anyhow!("initializing tracing: {error}"))
}
