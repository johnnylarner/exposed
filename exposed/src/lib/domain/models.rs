//! Collection of models used in the `exposed` project

pub mod declaration;
pub mod declaration_cleaning;
pub mod declaration_ingestion;
pub mod entity_ingestion;
pub mod entity_search;
pub mod funder;
pub mod funder_name;
pub mod funding_entry;
pub mod parliament_member;
pub mod search_similarity;

/// Checked database projection for a resolved declaration run.
pub mod declaration_loading;
pub mod declaration_resolution;

/// Entity profiles and declared support.
pub mod entity_details;
