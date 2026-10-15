use clap::Parser;
use exposed::inbound::{
    cli::{Commands, ExposedCli, IntoResponse, Response, run_cli, run_migrate, run_util},
    http::server::run_server,
};

fn main() -> anyhow::Result<std::process::ExitCode> {
    let args = ExposedCli::parse();
    match args.command {
        Commands::Server => run_server().map(|()| std::process::ExitCode::SUCCESS),
        Commands::Migrate(args) => {
            dotenvy::dotenv().ok();
            let response = match tokio::runtime::Runtime::new() {
                Ok(runtime) => runtime.block_on(run_migrate(args)),
                Err(error) => Err::<Response, _>(error).into_response(),
            };
            Ok(response.write_to(std::io::stdout().lock(), std::io::stderr().lock())?)
        }
        Commands::Data(args) => {
            if let Some(response) = args.schema_response() {
                return Ok(response.write_to(std::io::stdout().lock(), std::io::stderr().lock())?);
            }
            dotenvy::dotenv().ok();
            let response = match tokio::runtime::Runtime::new() {
                Ok(runtime) => runtime.block_on(run_cli(args)),
                Err(error) => Err::<Response, _>(error).into_response(),
            };
            Ok(response.write_to(std::io::stdout().lock(), std::io::stderr().lock())?)
        }
        Commands::Util(args) => {
            if let Some(response) = args.schema_response() {
                return Ok(response.write_to(std::io::stdout().lock(), std::io::stderr().lock())?);
            }
            dotenvy::dotenv().ok();
            let response = match tokio::runtime::Runtime::new() {
                Ok(runtime) => runtime.block_on(run_util(args)),
                Err(error) => Err::<Response, _>(error).into_response(),
            };
            Ok(response.write_to(std::io::stdout().lock(), std::io::stderr().lock())?)
        }
    }
}
