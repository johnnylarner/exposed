//! A collection of outbound adapters for the `exposed` project

mod entity_search_pipeline;
mod file_system;
mod funder;
mod parliament_member;
mod postgres;

pub use postgres::ExposedDatabase;
