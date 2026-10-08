//! A collection of outbound adapters for the `exposed` project

mod declaration_cleaning;
mod declaration_ingestion;
mod declaration_source;
mod entity_ingestion;
mod file_system;
mod funder;
mod parliament_api;
mod parliament_member;
mod postgres;

pub use file_system::ExposedDataPipeline;
pub use parliament_api::ParliamentApiClient;
pub use postgres::ExposedDatabase;

mod declaration_resolution;
pub use declaration_resolution::SplinkScorer;

mod declaration_loading;

mod entity_details;
