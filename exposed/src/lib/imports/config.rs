use chrono::NaiveDate;
use serde::Deserialize;
use std::{net::SocketAddr, path::PathBuf};

/// Optional runtime configuration for application-owned imports.
#[derive(Clone, Deserialize)]
pub struct ImportConfig {
    /// The single Parliament supported by this database.
    pub term_start: NaiveDate,
    /// Interpreter containing the API-only Python package, relative to the working directory.
    pub python: PathBuf,
    /// Python package directory, relative to the working directory.
    pub source_directory: PathBuf,
    /// Separate operator listener. Use loopback unless an access token is configured.
    pub admin_bind: SocketAddr,
    /// Environment variable holding a bearer token for operator commands.
    pub admin_token_env: Option<String>,
    /// Run the daily declaration check inside the application.
    #[serde(default = "enabled")]
    pub automatic_refresh: bool,
    /// Optional application notification interval. No interval means no notifications.
    pub notification_interval_seconds: Option<i64>,
    /// Official WhatsApp delivery configuration. Credentials stay in the environment.
    pub whatsapp: Option<WhatsAppConfig>,
}
fn enabled() -> bool {
    true
}

/// A template with one body text parameter must be approved for this sender.
#[derive(Clone, Deserialize)]
pub struct WhatsAppConfig {
    /// Supported Graph API version selected during sender setup, e.g. vXX.Y.
    pub api_version: String,
    /// Business Platform sender phone number identifier.
    pub phone_number_id: String,
    /// Recipient phone number in international digit format.
    pub recipient: String,
    /// Approved template name.
    pub template: String,
    /// Template language code.
    pub language: String,
    /// Environment variable holding the access token.
    pub token_env: String,
}
