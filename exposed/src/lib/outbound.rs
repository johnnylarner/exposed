//! A collection of outbound adapters for the `exposed` project

mod funder;
mod parliament_member;
mod postgres;

pub(crate) mod parliament;
pub(crate) mod whatsapp;

pub use postgres::ExposedDatabase;
