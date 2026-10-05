//! A collection of outbound adapters for the `exposed` project

mod declaration_ingestion;
mod entity_ingestion;
mod file_system;
mod funder;
mod ingestion_catalog;
mod parliament_api;
mod parliament_member;
mod postgres;

pub use file_system::ExposedDataPipeline;
pub use ingestion_catalog::ExposedIngestionCatalog;
pub use parliament_api::ParliamentApiClient;
pub use postgres::ExposedDatabase;
