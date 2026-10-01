//! Source observations needed to reproduce a configured Commons cohort.
use crate::domain::models::member_ingestion::{MemberHistory, MemberProfile};
use chrono::NaiveDate;
use thiserror::Error;

/// Parliament source capability, used only by Fetch.
pub trait ParliamentApi: Clone + Send + Sync + 'static {
    /// Current Commons profiles, retaining repeats for comparison within this stream.
    fn current_commons(
        &self,
    ) -> impl Future<Output = Result<Vec<MemberProfile>, ParliamentApiError>> + Send;
    /// Historical candidates without filtering on their latest House or current status.
    fn commons_candidates(
        &self,
        term_start: NaiveDate,
        observation_date: NaiveDate,
    ) -> impl Future<Output = Result<Vec<MemberProfile>, ParliamentApiError>> + Send;
    /// Histories for the requested IDs, retaining returned identities and duplicates.
    fn member_histories(
        &self,
        member_ids: &[i32],
    ) -> impl Future<Output = Result<Vec<MemberHistory>, ParliamentApiError>> + Send;
}

/// Source failure with request and field context.
#[derive(Error, Debug)]
#[error("{0}")]
pub struct ParliamentApiError(pub String);
