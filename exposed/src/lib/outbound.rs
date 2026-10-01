//! A collection of outbound adapters for the `exposed` project

mod file_system;
mod funder;
mod member_parquet;
mod parliament_api;
mod parliament_member;
mod postgres;

pub use file_system::ExposedDataPipeline;
pub use parliament_api::MembersApi;
pub use postgres::ExposedDatabase;
