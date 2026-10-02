//! HTTP inbound adapter

/// Config
pub mod config;
/// HTTP errors
pub mod error;
/// Handler code
mod handlers;
/// HTTP routes
pub mod routes;
/// Server initalization
pub mod server;
/// Axum app state
pub mod state;
/// HTTP success
pub mod success;
