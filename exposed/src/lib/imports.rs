//! Import use cases, ports, and delivery adapters owned by the Rust application.

mod adapters;
mod core;

mod admin;
/// Configuration of imports and optional WhatsApp delivery.
pub mod config;
mod runtime;
#[cfg(test)]
mod tests;
pub use runtime::serve;
