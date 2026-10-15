use std::io::{self, Write};

use exposed::inbound::cli::{IntoResponse, Response};

fn render(value: impl IntoResponse) -> Vec<u8> {
    let mut output = Vec::new();
    value.into_response().write_to(&mut output).unwrap();
    output
}

#[test]
fn unit_is_silent_and_strings_are_verbatim() {
    assert!(render(()).is_empty());
    assert_eq!(render("hello"), b"hello");
    assert_eq!(render("first\nsecond\n".to_owned()), b"first\nsecond\n");
    assert_eq!(render(""), b"");
}

#[test]
fn custom_results_and_responses_use_their_conversion() {
    struct Count(usize);
    impl IntoResponse for Count {
        fn into_response(self) -> Response {
            format!("Count: {}\n", self.0).into_response()
        }
    }
    assert_eq!(render(Ok::<_, io::Error>(Count(3))), b"Count: 3\n");
    assert_eq!(render("text".into_response()), b"text");
    assert!(render(Ok::<_, io::Error>(())).is_empty());
}

#[test]
fn command_error_preserves_type_and_context_without_output() {
    let error = anyhow::Error::new(io::Error::new(io::ErrorKind::NotFound, "missing run"))
        .context("loading declarations");
    let result: anyhow::Result<String> = Err(error);
    let mut output = Vec::new();
    let error = result.into_response().write_to(&mut output).unwrap_err();
    assert!(output.is_empty());
    assert_eq!(error.to_string(), "loading declarations");
    assert_eq!(error.root_cause().to_string(), "missing run");
    assert_eq!(
        error.downcast_ref::<io::Error>().unwrap().kind(),
        io::ErrorKind::NotFound
    );
}

#[test]
fn writer_failure_is_returned() {
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed output"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let error = "text".into_response().write_to(Broken).unwrap_err();
    assert_eq!(
        error.downcast_ref::<io::Error>().unwrap().kind(),
        io::ErrorKind::BrokenPipe
    );
}

#[test]
fn typed_command_errors_survive_response_conversion() {
    let result: Result<(), io::Error> =
        Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied"));
    let mut output = Vec::new();
    let response = result.into_response().into_response();
    let error = response.write_to(&mut output).unwrap_err();
    assert!(output.is_empty());
    assert_eq!(
        error.downcast_ref::<io::Error>().unwrap().kind(),
        io::ErrorKind::PermissionDenied
    );
}
