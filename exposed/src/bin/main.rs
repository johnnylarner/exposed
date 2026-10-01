use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    exposed::inbound::cli::run().await
}
