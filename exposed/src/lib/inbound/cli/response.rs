//! Command results and their conversion to CLI output.

use std::io::Write;

/// A command's output text or its original failure.
///
/// Render the response once at the CLI boundary. Text is written verbatim.
///
/// ```
/// use exposed::inbound::cli::IntoResponse;
///
/// let mut output = Vec::new();
/// "Finished\n".into_response().write_to(&mut output)?;
/// assert_eq!(output, b"Finished\n");
/// # Ok::<(), anyhow::Error>(())
/// ```
pub struct Response(anyhow::Result<String>);

impl Response {
    /// Write successful output without adding a newline.
    ///
    /// # Errors
    /// Returns the original command error or an error from the writer.
    pub fn write_to(self, mut writer: impl Write) -> anyhow::Result<()> {
        writer.write_all(self.0?.as_bytes())?;
        Ok(())
    }
}

/// Convert a command's return value to CLI output.
///
/// Unit values are silent. Strings are written verbatim. Results preserve errors
/// and convert successful values through their own implementation.
///
/// Command-specific output types can choose their text representation.
///
/// ```
/// use exposed::inbound::cli::{IntoResponse, Response};
///
/// struct Imported(usize);
///
/// impl IntoResponse for Imported {
///     fn into_response(self) -> Response {
///         format!("Imported {} records\n", self.0).into_response()
///     }
/// }
///
/// let result: anyhow::Result<Imported> = Ok(Imported(3));
/// let mut output = Vec::new();
/// result.into_response().write_to(&mut output)?;
/// assert_eq!(output, b"Imported 3 records\n");
/// # Ok::<(), anyhow::Error>(())
/// ```
pub trait IntoResponse {
    /// Convert this value to a response for the CLI boundary.
    fn into_response(self) -> Response;
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

impl IntoResponse for () {
    fn into_response(self) -> Response {
        String::new().into_response()
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Response {
        Response(Ok(self))
    }
}

impl IntoResponse for &str {
    fn into_response(self) -> Response {
        self.to_owned().into_response()
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
