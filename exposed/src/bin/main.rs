mod tracing;

use anyhow::Context;
use clap::Parser;
use exposed::inbound::{
    cli::{Commands, ExposedCli, IntoResponse, Response, run_cli, run_healthcheck, run_migrate},
    http::server::run_server,
};
use std::process::ExitCode;

fn main() -> anyhow::Result<ExitCode> {
    let args = ExposedCli::parse();
    if let Commands::Data(data) = &args.command
        && let Some(response) = data.schema_response()
    {
        return write_response(response);
    }
    match &args.command {
        Commands::Server | Commands::Migrate(_) | Commands::Data(_) => {
            dotenvy::dotenv().ok();
        }
        Commands::Healthcheck => {}
    }
    if let Err(error) = tracing::init(&args.command) {
        return match args.command {
            Commands::Data(_) | Commands::Migrate(_) => {
                write_response(Err::<Response, _>(error).into_response())
            }
            Commands::Server | Commands::Healthcheck => Err(error),
        };
    }
    let span = match &args.command {
        Commands::Server => ::tracing::error_span!("operation", command = "server"),
        Commands::Healthcheck => ::tracing::error_span!("operation", command = "healthcheck"),
        Commands::Migrate(_) => ::tracing::error_span!("operation", command = "migrate"),
        Commands::Data(data) => data.trace_span(),
    };
    let _entered = span.enter();
    finish(dispatch(args.command))
}

fn finish(result: anyhow::Result<ExitCode>) -> anyhow::Result<ExitCode> {
    match result {
        Ok(code) => Ok(code),
        Err(error) => {
            ::tracing::error!(error = %format!("{error:#}"), "operation failed");
            Err(error)
        }
    }
}

fn write_response(response: Response) -> anyhow::Result<ExitCode> {
    response
        .write_to(std::io::stdout().lock(), std::io::stderr().lock())
        .context("writing command response")
}

fn dispatch(command: Commands) -> anyhow::Result<ExitCode> {
    match command {
        Commands::Server => run_server().map(|()| ExitCode::SUCCESS),
        Commands::Healthcheck => {
            tokio::runtime::Runtime::new()?.block_on(run_healthcheck())?;
            ::tracing::info!("command completed");
            Ok(ExitCode::SUCCESS)
        }
        Commands::Migrate(args) => {
            let response = match tokio::runtime::Runtime::new() {
                Ok(runtime) => runtime.block_on(run_migrate(args)),
                Err(error) => Err::<Response, _>(error).into_response(),
            };
            write_response(response)
        }
        Commands::Data(args) => {
            let response = match tokio::runtime::Runtime::new() {
                Ok(runtime) => runtime.block_on(run_cli(args)),
                Err(error) => Err::<Response, _>(error).into_response(),
            };
            write_response(response)
        }
    }
}

#[cfg(test)]
#[path = "../../tests/common/tracing.rs"]
mod capture;

#[cfg(test)]
mod tests {
    use super::capture;

    use exposed::inbound::cli::{CliSuccess, IntoResponse};
    use std::io::{self, Write};

    struct BrokenWriter;

    impl Write for BrokenWriter {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed output"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn process_boundary_logs_each_distinct_failure_once() {
        for (command_failed, expected) in [(false, 1), (true, 2)] {
            let capture = capture::Capture::default();
            ::tracing::subscriber::with_default(capture.subscriber(), || {
                let result: anyhow::Result<_> = if command_failed {
                    Err(anyhow::anyhow!("command cause"))
                } else {
                    Ok(CliSuccess::new(1_u8))
                };
                let result = result
                    .into_response()
                    .write_to(BrokenWriter, BrokenWriter)
                    .map_err(anyhow::Error::from);
                assert!(super::finish(result).is_err());
            });
            let errors = capture.errors();
            assert_eq!(errors.len(), expected);
            assert!(
                !capture
                    .events()
                    .iter()
                    .any(|event| event["fields"]["message"] == "command completed")
            );
            assert_eq!(errors.last().unwrap()["fields"]["error"], "closed output");
            if command_failed {
                assert_eq!(errors[0]["fields"]["error"], "command cause");
            }
        }
        let capture = capture::Capture::default();
        ::tracing::subscriber::with_default(capture.subscriber(), || {
            assert!(super::finish(Ok(std::process::ExitCode::FAILURE)).is_ok());
        });
        assert!(capture.errors().is_empty());
    }
}
