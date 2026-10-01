//! Source observations needed to reproduce a configured Commons cohort.
use crate::domain::models::parliament_member::ParliamentMember;
use chrono::NaiveDate;
use thiserror::Error;

/// Parliament source capability, used only by Fetch.
pub trait ParliamentApi: Clone + Send + Sync + 'static {
    /// Current Commons observations, retaining repeats for comparison within this stream.
    fn current_commons(
        &self,
    ) -> impl Future<Output = Result<Vec<ParliamentMember>, ParliamentApiError>> + Send;
    /// Historical candidates without filtering on their latest House or current status.
    fn commons_candidates(
        &self,
        term_start: NaiveDate,
        observation_date: NaiveDate,
    ) -> impl Future<Output = Result<Vec<ParliamentMember>, ParliamentApiError>> + Send;
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

/// One source House membership, before term clipping or deduplication.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct HouseMembership {
    house: i16,
    start_date: NaiveDate,
    end_date: Option<NaiveDate>,
}
impl HouseMembership {
    /// Accept a source period without deriving Commons service from it.
    ///
    /// # Errors
    /// Rejects an unknown House or an end date before the start.
    pub fn new(
        house: i16,
        start_date: NaiveDate,
        end_date: Option<NaiveDate>,
    ) -> Result<Self, ParliamentApiError> {
        if !matches!(house, 1 | 2) || end_date.is_some_and(|end| end < start_date) {
            return Err(ParliamentApiError(
                "invalid House or membership date ordering".into(),
            ));
        }
        Ok(Self {
            house,
            start_date,
            end_date,
        })
    }
    /// Commons (1) or Lords (2).
    #[must_use]
    pub const fn house(&self) -> i16 {
        self.house
    }
    /// Source calendar start date.
    #[must_use]
    pub const fn start_date(&self) -> NaiveDate {
        self.start_date
    }
    /// Date service ceased, if supplied.
    #[must_use]
    pub const fn end_date(&self) -> Option<NaiveDate> {
        self.end_date
    }
}

/// A returned history, retaining its source identity and every period.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberHistory {
    parliament_member_id: i32,
    house_memberships: Vec<HouseMembership>,
}
impl MemberHistory {
    /// Accept one identified, nonempty source history.
    ///
    /// # Errors
    /// Rejects an invalid member identity or an empty history.
    pub fn new(
        parliament_member_id: i32,
        house_memberships: Vec<HouseMembership>,
    ) -> Result<Self, ParliamentApiError> {
        if parliament_member_id <= 0 || house_memberships.is_empty() {
            return Err(ParliamentApiError(format!(
                "member {parliament_member_id}: invalid identity or empty history"
            )));
        }
        Ok(Self {
            parliament_member_id,
            house_memberships,
        })
    }
    /// Parliament's numeric identity.
    #[must_use]
    pub const fn parliament_member_id(&self) -> i32 {
        self.parliament_member_id
    }
    /// All source periods, including identical repeats.
    #[must_use]
    pub fn house_memberships(&self) -> &[HouseMembership] {
        &self.house_memberships
    }
}
