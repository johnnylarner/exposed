use std::{fs::OpenOptions, sync::Mutex};

use super::Commands;
use anyhow::Context;
use tracing_subscriber::{EnvFilter, fmt::writer::BoxMakeWriter};

/// Initializes JSON tracing for the command's configured log destination.
///
/// `EXPOSED_LOG_FILE` selects an append-only file sink for every command. Without
/// it, the server logs to stderr and other commands leave tracing uninitialized.
/// `RUST_LOG` controls filtering, with `off,exposed=info` as the fallback.
///
/// # Errors
///
/// Returns an error if the log file cannot be opened, the fallback filter cannot
/// be parsed, or a global tracing subscriber is already installed.
pub fn init_tracing(command: &Commands) -> anyhow::Result<()> {
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
