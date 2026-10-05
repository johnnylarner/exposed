use std::{fs::File, io::Write, path::PathBuf};

use exposed::{
    domain::models::entity_search::EntitySearchRequest,
    inbound::{
        cli::config::{DeclarationFetcherConfig, FetcherConfig, LoaderConfig},
        http::{config::ServerConfig, server::serve_exposed},
    },
};
use reqwest::StatusCode;
use serde_json::Value;
use tempfile::{NamedTempFile, TempDir};

mod declarations;
pub use declarations::{DeclarationFunding, FundingEntry, read_declaration_funding};

pub struct Guard {
    port: u16,
}

pub async fn search_entities(
    app: &Guard,
    params: &EntitySearchRequest,
) -> anyhow::Result<Vec<Value>> {
    let client = reqwest::Client::new();

    let port = app.port;

    let (term, strictness, entries) = (params.term(), params.strictness(), params.max_entries());
    let url = format!(
        "http://127.0.0.1:{port}/search?term={term}&&strictness={strictness}&&max_entries={entries}"
    );
    let response = client.get(url).send().await?;

    match response.status() {
        StatusCode::OK => {
            let bytes = response.bytes().await?;
            let data: Value = serde_json::from_slice(&bytes)?;
            let entities = data
                .get("entities")
                .and_then(|d| d.as_array())
                .ok_or(anyhow::anyhow!("unparsable"))?
                .to_vec();
            Ok(entities)
        }
        StatusCode::INTERNAL_SERVER_ERROR => Err(anyhow::anyhow!("something unexpected happened")),
        StatusCode::UNPROCESSABLE_ENTITY => Err(anyhow::anyhow!("bad url")),
        _ => Err(anyhow::anyhow!(
            "something really unexpected happened, investigate"
        )),
    }
}

pub async fn start_app() -> anyhow::Result<Guard> {
    let config = ServerConfig::from_env()?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let _handle = tokio::task::spawn(async move {
        let _ = serve_exposed(&config, listener).await;
        println!("started");
    });

    Ok(Guard { port })
}

pub fn cli_fetcher_config(tmp: &TempDir) -> (NamedTempFile<File>, PathBuf) {
    let config = FetcherConfig {
        batch_size: 100,
        data_dir: tmp.path().to_path_buf(),
    };
    let contents = rust_yaml::to_string(&config).unwrap();
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(contents.as_bytes()).unwrap();
    let path = file.path().to_path_buf();
    (file, path)
}

pub fn cli_loader_config(tmp: &TempDir) -> (NamedTempFile<File>, PathBuf) {
    let config = LoaderConfig {
        data_dir: tmp.path().to_path_buf(),
    };
    let contents = rust_yaml::to_string(&config).unwrap();
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(contents.as_bytes()).unwrap();
    let path = file.path().to_path_buf();
    (file, path)
}

pub fn cli_declaration_fetcher_config(tmp: &TempDir) -> (NamedTempFile<File>, PathBuf) {
    let config = DeclarationFetcherConfig {
        data_dir: tmp.path().to_path_buf(),
        // Small pages exercise pagination against the live source.
        batch_size: 5,
    };
    let contents = rust_yaml::to_string(&config).unwrap();
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(contents.as_bytes()).unwrap();
    let path = file.path().to_path_buf();
    (file, path)
}
