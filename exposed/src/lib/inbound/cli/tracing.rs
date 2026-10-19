use std::{io, process::ExitCode};

use anyhow::Context;

use super::{Response, runner::CommandOutput};

pub(super) fn trace_command(
    span: tracing::Span,
    execute: impl FnOnce() -> anyhow::Result<CommandOutput>,
    write: impl FnOnce(Response) -> io::Result<ExitCode>,
) -> anyhow::Result<ExitCode> {
    let _entered = span.entered();
    let result = match execute() {
        Ok(CommandOutput::Json(response)) => {
            if let Some(error) = response.failure() {
                tracing::error!(error = %format!("{error:#}"), "command failed");
            }
            write(response)
                .context("writing command response")
                .inspect(|code| {
                    if *code == ExitCode::SUCCESS {
                        tracing::info!("command completed");
                    }
                })
        }
        Ok(CommandOutput::HealthcheckPassed) => {
            tracing::info!("command completed");
            Ok(ExitCode::SUCCESS)
        }
        Ok(CommandOutput::ServerStopped) => Ok(ExitCode::SUCCESS),
        Err(error) => Err(error),
    };
    result.inspect_err(|error| {
        tracing::error!(error = %format!("{error:#}"), "operation failed");
    })
}

#[cfg(test)]
mod capture {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/common/tracing.rs"
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inbound::cli::{CliSuccess, IntoResponse};
    use schemars::JsonSchema;
    use serde::{Serialize, Serializer};
    use std::io::Write;

    struct BrokenWriter;

    impl Write for BrokenWriter {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed output"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn observe(
        output: anyhow::Result<CommandOutput>,
        write: impl FnOnce(Response) -> io::Result<ExitCode>,
    ) -> anyhow::Result<ExitCode> {
        trace_command(
            tracing::error_span!("operation", command = "test"),
            || output,
            write,
        )
    }

    #[test]
    fn command_and_output_failures_are_observed_once_each() {
        for (command_failed, expected) in [(false, 1), (true, 2)] {
            let capture = capture::Capture::default();
            tracing::subscriber::with_default(capture.subscriber(), || {
                let result: anyhow::Result<_> = if command_failed {
                    Err(anyhow::anyhow!("command cause").context("command context"))
                } else {
                    Ok(CliSuccess::new(1_u8))
                };
                let response = result.into_response().into_response();
                assert!(capture.events().is_empty());
                assert!(
                    observe(Ok(CommandOutput::Json(response)), |response| {
                        response.write_to(BrokenWriter, BrokenWriter)
                    })
                    .is_err()
                );
            });
            let errors = capture.errors();
            assert_eq!(errors.len(), expected);
            assert!(
                !capture
                    .events()
                    .iter()
                    .any(|event| event["fields"]["message"] == "command completed")
            );
            assert_eq!(
                errors.last().unwrap()["fields"]["error"],
                "writing command response: closed output"
            );
            if command_failed {
                assert_eq!(
                    errors[0]["fields"]["error"],
                    "command context: command cause"
                );
            }
        }
    }

    #[test]
    fn command_failure_is_not_logged_again_after_successful_write() {
        let capture = capture::Capture::default();
        tracing::subscriber::with_default(capture.subscriber(), || {
            let response = Err::<Response, _>(anyhow::anyhow!("command cause")).into_response();
            assert_eq!(
                observe(Ok(CommandOutput::Json(response)), |response| {
                    response.write_to(Vec::new(), Vec::new())
                })
                .unwrap(),
                ExitCode::FAILURE
            );
        });
        assert_eq!(capture.events().len(), 1);
        assert_eq!(capture.errors()[0]["fields"]["error"], "command cause");
    }

    #[test]
    fn serialization_failure_is_observed_at_boundary() {
        #[derive(JsonSchema)]
        struct Broken;
        impl Serialize for Broken {
            fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
                Err(serde::ser::Error::custom("cannot serialize payload"))
            }
        }
        for write_fails in [false, true] {
            let capture = capture::Capture::default();
            tracing::subscriber::with_default(capture.subscriber(), || {
                let response = CliSuccess::new(Broken).into_response().into_response();
                assert!(capture.events().is_empty());
                let result = observe(Ok(CommandOutput::Json(response)), |response| {
                    if write_fails {
                        response.write_to(Vec::new(), BrokenWriter)
                    } else {
                        response.write_to(Vec::new(), Vec::new())
                    }
                });
                if write_fails {
                    assert!(result.is_err());
                } else {
                    assert_eq!(result.unwrap(), ExitCode::FAILURE);
                }
            });
            assert_eq!(capture.events().len(), if write_fails { 2 } else { 1 });
            assert_eq!(
                capture.errors()[0]["fields"]["error"],
                "cannot serialize payload"
            );
            if write_fails {
                assert_eq!(
                    capture.errors()[1]["fields"]["error"],
                    "writing command response: closed output"
                );
            }
        }
    }

    #[test]
    fn completion_follows_successful_write() {
        let capture = capture::Capture::default();
        tracing::subscriber::with_default(capture.subscriber(), || {
            let response = CliSuccess::new(1_u8).into_response();
            assert_eq!(
                observe(Ok(CommandOutput::Json(response)), |response| {
                    assert!(capture.events().is_empty());
                    response.write_to(Vec::new(), Vec::new())
                })
                .unwrap(),
                ExitCode::SUCCESS
            );
        });
        assert_eq!(capture.events().len(), 1);
        assert_eq!(
            capture.events()[0]["fields"]["message"],
            "command completed"
        );
    }

    #[test]
    fn process_errors_and_silent_success_preserve_observation_policy() {
        let capture = capture::Capture::default();
        tracing::subscriber::with_default(capture.subscriber(), || {
            assert!(
                observe(Err(anyhow::anyhow!("process cause")), |_| panic!(
                    "no JSON output"
                ))
                .is_err()
            );
            assert_eq!(
                observe(Ok(CommandOutput::ServerStopped), |_| panic!(
                    "no JSON output"
                ))
                .unwrap(),
                ExitCode::SUCCESS
            );
            assert_eq!(capture.events().len(), 1);
            assert_eq!(
                observe(Ok(CommandOutput::HealthcheckPassed), |_| panic!(
                    "no JSON output"
                ))
                .unwrap(),
                ExitCode::SUCCESS
            );
        });
        assert_eq!(capture.errors().len(), 1);
        assert_eq!(capture.errors()[0]["fields"]["error"], "process cause");
        assert_eq!(capture.events().len(), 2);
    }
}
