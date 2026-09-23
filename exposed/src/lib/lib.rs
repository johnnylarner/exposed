#![deny(
    clippy::all,
    clippy::restriction,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    missing_docs,
    warnings
)]

//! The exposed project serves declaration data from MPs in the UK parliament.

/// Contains the domain models for the project.
pub mod domain;

/// Contains the inbound adapters for the project
pub mod inbound;

/// Contains the outbound adapters for the project
pub mod outbound;

/// Contains the configuration parameters for this project
pub mod config;
