//! JSON command results and their conversion to CLI output.

use schemars::JsonSchema;
use serde::Serialize;
use std::{
    io::{self, Write},
    process::ExitCode,
};

/// Generic JSON success response.
///
/// Payloads must expose their JSON schema as well as their serialization.
///
/// ```compile_fail
/// use exposed::inbound::cli::CliSuccess;
/// #[derive(serde::Serialize)]
/// struct Undocumented { count: usize }
/// let _ = CliSuccess::new(Undocumented { count: 1 });
/// ```
pub struct CliSuccess<S: Serialize + JsonSchema> {
    response: S,
}

impl<S: Serialize + JsonSchema> CliSuccess<S> {
    /// Creates a success response with a typed payload.
    pub const fn new(response: S) -> Self {
        Self { response }
    }
}

/// A serialized success or the original command failure.
pub struct Response(anyhow::Result<Vec<u8>>);

#[derive(Serialize, JsonSchema)]
pub(super) struct ErrorResponse {
    error: ErrorMessage,
}

#[derive(Serialize, JsonSchema)]
struct ErrorMessage {
    message: String,
}

impl Response {
    pub(super) fn schema(document: &serde_json::Value) -> Self {
        Self(serde_json::to_vec(document).map_err(Into::into))
    }

    /// Writes one JSON document to stdout on success or stderr on failure.
    ///
    /// # Errors
    /// Returns an error if the selected output writer fails.
    pub fn write_to(self, mut stdout: impl Write, mut stderr: impl Write) -> io::Result<ExitCode> {
        let (mut bytes, writer, code): (_, &mut dyn Write, _) = match self.0 {
            Ok(bytes) => (bytes, &mut stdout, ExitCode::SUCCESS),
            Err(error) => {
                tracing::error!(error = %format!("{error:#}"), "command failed");
                let payload = ErrorResponse {
                    error: ErrorMessage {
                        message: format!("{error:#}"),
                    },
                };
                let bytes = serde_json::to_vec(&payload).map_err(io::Error::other)?;
                (bytes, &mut stderr, ExitCode::FAILURE)
            }
        };
        bytes.push(b'\n');
        writer.write_all(&bytes)?;
        if code == ExitCode::SUCCESS {
            tracing::info!("command completed");
        }
        Ok(code)
    }
}

/// Converts a typed command result to a CLI response.
///
/// Wrap success payloads in [`CliSuccess`] before converting them.
///
/// ```compile_fail
/// use exposed::inbound::cli::IntoResponse;
/// "arbitrary text".into_response();
/// ```
pub trait IntoResponse {
    /// Converts this value to a response for the CLI boundary.
    fn into_response(self) -> Response;
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

impl<S: Serialize + JsonSchema> IntoResponse for CliSuccess<S> {
    fn into_response(self) -> Response {
        Response(serde_json::to_vec(&self.response).map_err(Into::into))
    }
}

impl<T, E> IntoResponse for Result<T, E>
where
    T: IntoResponse,
    E: Into<anyhow::Error>,
{
    fn into_response(self) -> Response {
        match self {
            Ok(value) => value.into_response(),
            Err(error) => Response(Err(error.into())),
        }
    }
}
