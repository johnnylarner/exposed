use std::{path::PathBuf, str::FromStr, sync::LazyLock};

use exposed::{
    config::Config, domain::models::entity_search::EntitySearchRequest,
    inbound::http::server::serve_exposed,
};
use reqwest::StatusCode;
use serde_json::Value;

const TEST_CONFIG: LazyLock<Config> = LazyLock::new(|| {
    let test_config = PathBuf::from_str("config/test.yaml").unwrap();
    let config = Config::try_from(&test_config).unwrap();
    config
});

pub struct Guard;

pub async fn search_entities(params: &EntitySearchRequest) -> anyhow::Result<Vec<Value>> {
    let client = reqwest::Client::new();
    let port = TEST_CONFIG.port;

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
            return Ok(entities);
        }
        StatusCode::INTERNAL_SERVER_ERROR => {
            return Err(anyhow::anyhow!("something unexpected happened"));
        }
        StatusCode::UNPROCESSABLE_ENTITY => return Err(anyhow::anyhow!("bad url")),
        _ => {
            return Err(anyhow::anyhow!(
                "something really unexpected happened, investigate"
            ));
        }
    }
}

pub async fn start_app() -> anyhow::Result<Guard> {
    let _handle = tokio::task::spawn(async move {
        let _ = serve_exposed(&TEST_CONFIG).await;
        println!("started");
    });

    Ok(Guard)
}
