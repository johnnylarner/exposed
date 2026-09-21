//! Entity search refers to the landing page of the `exposed` application. Its purpose it to make it
//! easy for journalists to find [`parliament_members`] or [`funders`].
//!

mod error;
mod interface;

use crate::domain::repositories::funder_repository::FunderRepo;
use crate::domain::repositories::parliament_member_repository::ParliamentMemberRepo;
pub use crate::domain::services::entity_search::error::EntitySearchError;
pub use crate::domain::services::entity_search::interface::EntitySearchService as Interface;

/// Allows users to search the databse using free text.
#[derive(Clone)]
pub struct EntitySearchService<P, F> {
    mp_repo: P,
    funder_repo: F,
}

impl<P, F> EntitySearchService<P, F> {
    /// Creates a new instance
    pub fn new(mp_repo: P, funder_repo: F) -> Self {
        Self {
            mp_repo,
            funder_repo,
        }
    }
}

impl<P, F> Interface<P, F> for EntitySearchService<P, F>
where
    P: ParliamentMemberRepo,
    F: FunderRepo,
{
    async fn search_entities(&self) -> Result<(), EntitySearchError> {
        let members = self.mp_repo.get_members_by_text_search().await?;
        let _ = self.funder_repo.get_funders_by_text_search().await?;

        Ok(members)
    }
}
