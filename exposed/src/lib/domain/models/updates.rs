//! Nonfinancial activity within one UTC calendar month.
use super::{
    funder::{FunderId, FunderKind},
    parliament_member::MemberId,
};
use chrono::{DateTime, Datelike, Months, Utc};
use std::num::NonZeroUsize;

/// Half-open UTC calendar month.
#[derive(Clone, Debug)]
pub struct RegistrationMonth {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}
impl RegistrationMonth {
    /// Selects the complete calendar month containing the supplied instant.
    ///
    /// # Panics
    /// Panics if the instant is in Chrono's final representable month.
    #[must_use]
    pub fn containing(now: DateTime<Utc>) -> Self {
        let start = now
            .date_naive()
            .with_day(1)
            .expect("day one exists")
            .and_hms_opt(0, 0, 0)
            .expect("midnight exists")
            .and_utc();
        let end = start
            .checked_add_months(Months::new(1))
            .expect("following calendar month exists");
        Self { start, end }
    }
    /// Inclusive lower boundary.
    #[must_use]
    pub const fn start(&self) -> DateTime<Utc> {
        self.start
    }
    /// Exclusive upper boundary.
    #[must_use]
    pub const fn end(&self) -> DateTime<Utc> {
        self.end
    }
    /// Whether a registration belongs to this month.
    #[must_use]
    pub fn includes(&self, date: DateTime<Utc>) -> bool {
        date >= self.start && date < self.end
    }
}
/// Recipient identity.
#[derive(Clone, Debug)]
pub struct UpdatedMember {
    /// Parliament identity.
    pub id: MemberId,
    /// Stored name.
    pub name: String,
}
/// Canonical company identity.
#[derive(Clone, Debug)]
pub struct UpdatedCompany {
    /// Canonical alias-group identity.
    pub id: FunderId,
    /// Preferred display name.
    pub name: String,
}
/// An identity with at least one declaration in the month.
#[derive(Debug)]
pub struct EntityUpdate<I> {
    /// Updated identity.
    pub identity: I,
    /// Distinct source declarations.
    pub declaration_count: NonZeroUsize,
    /// Latest registration in the month.
    pub latest_registration: DateTime<Utc>,
}
/// Complete monthly activity snapshot.
#[derive(Debug)]
pub struct MonthlyUpdates {
    /// Calendar boundaries.
    pub month: RegistrationMonth,
    /// All updated MPs.
    pub members: Vec<EntityUpdate<UpdatedMember>>,
    /// All updated canonical nonindividual funders.
    pub companies: Vec<EntityUpdate<UpdatedCompany>>,
}
/// Lightweight stored declaration fact.
pub struct UpdateDeclaration {
    /// Source identity, independent of funding occurrences.
    pub source_id: i32,
    /// Registration instant.
    pub registered_at: DateTime<Utc>,
    /// Recipient.
    pub member: UpdatedMember,
    /// Canonical funders, possibly repeated across occurrences.
    pub funders: Vec<UpdateFunder>,
}
/// Canonical funder metadata at the persistence boundary.
pub struct UpdateFunder {
    /// Canonical identity.
    pub id: FunderId,
    /// Preferred display name.
    pub name: String,
    /// Canonical root classification.
    pub kind: FunderKind,
}
