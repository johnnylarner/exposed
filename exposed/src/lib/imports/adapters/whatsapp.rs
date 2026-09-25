use crate::imports::{
    config::WhatsAppConfig,
    core::{ImportError, Result, ports::Notifications},
};
use anyhow::Context;
use serde_json::json;
use std::time::Duration;

pub(crate) struct WhatsApp {
    client: reqwest::Client,
    config: WhatsAppConfig,
    token: String,
}
impl WhatsApp {
    pub fn new(config: WhatsAppConfig) -> anyhow::Result<Self> {
        let token = std::env::var(&config.token_env)
            .context("WhatsApp token environment variable is missing")?;
        anyhow::ensure!(!token.is_empty(), "WhatsApp token is empty");
        anyhow::ensure!(
            config.api_version.starts_with('v')
                && config.api_version[1..]
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == '.'),
            "Invalid Graph API version"
        );
        anyhow::ensure!(
            !config.phone_number_id.is_empty()
                && config.phone_number_id.chars().all(|c| c.is_ascii_digit()),
            "Invalid WhatsApp sender ID"
        );
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()?,
            config,
            token,
        })
    }
}
impl Notifications for WhatsApp {
    async fn deliver(&self, message: &str) -> Result<()> {
        let url = format!(
            "https://graph.facebook.com/{}/{}/messages",
            self.config.api_version, self.config.phone_number_id
        );
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.token)
            .json(&json!({
                "messaging_product":"whatsapp", "to":self.config.recipient, "type":"template",
                "template": {"name":self.config.template,"language":{"code":self.config.language},
                    "components":[{"type":"body","parameters":[{"type":"text","text":message}]}]}
            }))
            .send()
            .await
            .map_err(|e| ImportError::source_failure("WhatsApp transport failed", e))?;
        let status = response.status();
        response.error_for_status().map_err(|e| {
            ImportError::source_failure(format!("WhatsApp returned HTTP {}", status.as_u16()), e)
        })?;
        Ok(())
    }
}
