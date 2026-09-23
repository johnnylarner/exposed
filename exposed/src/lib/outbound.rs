//! A collection of outbound adapters for the `exposed` project

mod funder;
mod parliament_member;
mod postgres;

pub use postgres::ExposedDatabase;
