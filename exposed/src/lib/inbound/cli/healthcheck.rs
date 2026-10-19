use std::time::Duration;

use anyhow::{Context, ensure};
use reqwest::{Client, StatusCode, Url, redirect::Policy};

/// Checks whether the local HTTP server responds with its liveness status.
///
/// # Errors
/// Returns an error if the probe fails, times out, or receives a status other than 204.
pub(super) async fn run_healthcheck() -> anyhow::Result<()> {
    probe(Url::parse("http://127.0.0.1:6999/healthz")?).await
}

async fn probe(endpoint: Url) -> anyhow::Result<()> {
    let client = Client::builder()
        .no_proxy()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(2))
        .build()
        .context("building healthcheck client")?;
    let response = client
        .get(endpoint)
        .send()
        .await
        .context("requesting server liveness")?;
    ensure!(
        response.status() == StatusCode::NO_CONTENT,
        "expected healthcheck HTTP 204, received {}",
        response.status()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::{Router, http::StatusCode, response::Redirect, routing::get};
    use tokio::net::TcpListener;

    use super::probe;

    #[tokio::test]
    async fn accepts_only_no_content_without_following_redirects() -> anyhow::Result<()> {
        let router = Router::new()
            .route("/healthy", get(|| async { StatusCode::NO_CONTENT }))
            .route("/ok", get(|| async { StatusCode::OK }))
            .route(
                "/unavailable",
                get(|| async { StatusCode::SERVICE_UNAVAILABLE }),
            )
            .route(
                "/redirect",
                get(|| async { Redirect::temporary("/healthy") }),
            );
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let server = tokio::spawn(async move { axum::serve(listener, router).await });
        assert!(
            probe(format!("http://{address}/healthy").parse()?)
                .await
                .is_ok()
        );
        for (path, status) in [("ok", "200"), ("unavailable", "503"), ("redirect", "307")] {
            let error = probe(format!("http://{address}/{path}").parse()?)
                .await
                .unwrap_err();
            assert!(error.to_string().contains(status), "{error:#}");
        }
        server.abort();
        Ok(())
    }

    #[tokio::test]
    async fn fails_when_the_server_does_not_respond() -> anyhow::Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let endpoint = format!("http://{}/healthz", listener.local_addr()?).parse()?;
        let error = probe(endpoint).await.unwrap_err();
        assert!(error.downcast_ref::<reqwest::Error>().unwrap().is_timeout());
        Ok(())
    }

    #[tokio::test]
    async fn fails_when_the_connection_is_refused() -> anyhow::Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let endpoint = format!("http://{}/healthz", listener.local_addr()?).parse()?;
        drop(listener);
        let error = probe(endpoint).await.unwrap_err();
        assert!(error.downcast_ref::<reqwest::Error>().unwrap().is_connect());
        Ok(())
    }
}
