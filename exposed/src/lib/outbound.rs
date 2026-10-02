//! A collection of outbound adapters for the `exposed` project

mod entity_search_pipeline;
mod file_system;
mod funder;
mod parliament_api;
mod parliament_member;
mod postgres;

pub use file_system::ExposedDataPipeline;
pub use parliament_api::ParliamentApiClient;
pub use postgres::ExposedDatabase;
