//! Contains entity ingestion models

// use std::str::FromStr;
//
use strum::EnumString;
use thiserror::Error;

use crate::domain::repositories::{
    entity_ingestion_pipline::EntitySearchPipelineError, parliament_api::ParliamentApiError,
};

/// Data required to search entities
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityIngestionRequest {
    // target: EntityIngestionTarget,
}
//
//
// impl EntityIngestionRequest {
//     pub fn new_members_request(stage: &str) -> Result<Self, EntityIngestionError> {
//         let stage = MemberIngestionStage::from_str(stage)
//             .map_err(|_| EntityIngestionError::InvalidStage(stage.to_string()))?;
//         Ok(Self {
//             target: EntityIngestionTarget::Members(stage),
//         })
//     }
//     pub fn new_declaration_request(stage: &str) -> Result<Self, EntityIngestionError> {
//         let stage = DeclarationIngestionStage::from_str(stage)
//             .map_err(|_| EntityIngestionError::InvalidStage(stage.to_string()))?;
//
//         Ok(Self {
//             target: EntityIngestionTarget::Declaration(stage),
//         })
//     }
//
//     pub fn target(&self) -> &EntityIngestionTarget {
//         &self.target
//     }
// }
//
// /// Ingestion targets
// #[derive(Clone, Debug, PartialEq, Eq)]
// pub enum EntityIngestionTarget {
//     /// MPs
//     Members(MemberIngestionStage),
//     /// Declarations, funders, funding entries
//     Declaration(DeclarationIngestionStage),
// }
//
/// From what stage to run in the Member pipeline
#[derive(Clone, Debug, PartialEq, Eq, EnumString)]
#[allow(missing_docs)]
#[strum(ascii_case_insensitive)]
pub enum MemberIngestionStage {
    Fetch,
    // Load,
}
//
// /// From what stage to run the declaration pipeline
// #[derive(Clone, Debug, PartialEq, Eq, EnumString)]
// #[allow(missing_docs)]
// #[strum(ascii_case_insensitive)]
// pub enum DeclarationIngestionStage {
//     Fetch,
//     Clean,
//     Resolve,
//     Load,
// }
//
/// Errors related to entity search
#[derive(Error, Debug)]
pub enum EntityIngestionError {
    /// Errors related to the parliament API
    #[error("error when retrieving data from the parliament API: {0}")]
    ApiError(#[from] ParliamentApiError),

    /// Generic error related to entity parsing, cleaning and resolution
    #[error("error in ingestion pipeline: {0}")]
    DataError(String),

    /// Invalid strictness value
    #[error("at least one entry must be expected to return")]
    IoError(#[from] EntitySearchPipelineError),

    /// Invalid stage
    #[error("invalid stage provided: {0}")]
    InvalidStage(String),
}
