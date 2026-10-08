//! Collection of repositories for the `exposed` project

/// Access data about funders
pub mod funder_repository;

/// Access data about MPs
pub mod parliament_member_repository;

/// Pubilc API for UK Parliament  
pub mod parliament_api;

pub mod declaration_cleaning;
/// Store for cleaning parliament API data;
pub mod entity_ingestion;

pub mod declaration_resolution;

/// Entity profiles and declared support.
pub mod entity_details;
