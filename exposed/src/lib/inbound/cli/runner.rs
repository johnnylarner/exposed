use std::process::ExitCode;

use anyhow::Context;
use clap::Parser;

use super::{Commands, ExposedCli, IntoResponse, Response, run_cli};
use super::{healthcheck::run_healthcheck, migrate::run_migrate, telemetry::init_tracing};
use crate::inbound::http::server::run_server;

pub(super) enum CommandOutput {
    Json(Response),
    HealthcheckPassed,
    ServerStopped,
}

/// Runs the CLI adapter, including setup, command execution, and output.
///
/// # Errors
/// Returns server, healthcheck, setup, or output failures using process diagnostics.
pub fn run() -> anyhow::Result<ExitCode> {
    let args = ExposedCli::parse();
    if let Commands::Data(data) = &args.command
        && let Some(response) = data.schema_response()
    {
        return response
            .write_to(std::io::stdout().lock(), std::io::stderr().lock())
            .context("writing command response");
    }
    match &args.command {
        Commands::Server | Commands::Migrate(_) | Commands::Data(_) => {
            dotenvy::dotenv().ok();
        }
        Commands::Healthcheck => {}
    }
    if let Err(error) = init_tracing(&args.command) {
        return match args.command {
            Commands::Data(_) | Commands::Migrate(_) => Err::<Response, _>(error)
                .into_response()
                .write_to(std::io::stdout().lock(), std::io::stderr().lock())
                .context("writing command response"),
            Commands::Server | Commands::Healthcheck => Err(error),
        };
    }
    let span = match &args.command {
        Commands::Server => tracing::error_span!("operation", command = "server"),
        Commands::Healthcheck => tracing::error_span!("operation", command = "healthcheck"),
        Commands::Migrate(_) => tracing::error_span!("operation", command = "migrate"),
        Commands::Data(data) => data.trace_span(),
    };
    super::tracing::trace_command(
        span,
        || dispatch(args.command),
        |response| response.write_to(std::io::stdout().lock(), std::io::stderr().lock()),
    )
}

fn dispatch(command: Commands) -> anyhow::Result<CommandOutput> {
    match command {
        Commands::Server => run_server().map(|()| CommandOutput::ServerStopped),
        Commands::Healthcheck => {
            tokio::runtime::Runtime::new()?.block_on(run_healthcheck())?;
            Ok(CommandOutput::HealthcheckPassed)
        }
        Commands::Migrate(args) => {
            let response = match tokio::runtime::Runtime::new() {
                Ok(runtime) => runtime.block_on(run_migrate(args)),
                Err(error) => Err::<Response, _>(error).into_response(),
            };
            Ok(CommandOutput::Json(response))
        }
        Commands::Data(args) => {
            let response = match tokio::runtime::Runtime::new() {
                Ok(runtime) => runtime.block_on(run_cli(args)),
                Err(error) => Err::<Response, _>(error).into_response(),
            };
            Ok(CommandOutput::Json(response))
        }
    }
}
