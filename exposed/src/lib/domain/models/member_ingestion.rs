//! Captured member observations and the rules for interpreting Commons service.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    str::FromStr,
};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::entity_ingestion::EntityIngestionError;

/// Identity of an immutable capture, ordered by `UUIDv7` creation time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "Uuid", into = "Uuid")]
pub struct CaptureId(Uuid);

impl CaptureId {
    /// Allocate an identity at the beginning of a fetch attempt.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for CaptureId {
    fn default() -> Self {
        Self::new()
    }
}
impl From<CaptureId> for Uuid {
    fn from(id: CaptureId) -> Self {
        id.0
    }
}
impl TryFrom<Uuid> for CaptureId {
    type Error = EntityIngestionError;
    fn try_from(id: Uuid) -> Result<Self, Self::Error> {
        if id.get_version_num() != 7 || id.get_variant() != uuid::Variant::RFC4122 {
            return Err(invalid("capture ID must be a UUIDv7"));
        }
        Ok(Self(id))
    }
}
impl FromStr for CaptureId {
    type Err = EntityIngestionError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(value)
            .map_err(|_| invalid("capture ID must be a UUIDv7"))?
            .try_into()
    }
}
impl fmt::Display for CaptureId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Context fixed when acquisition begins, retained for offline interpretation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureContext {
    /// `UUIDv7` allocated for this attempt.
    pub capture_id: CaptureId,
    /// Beginning of the configured Parliament.
    pub term_start: NaiveDate,
    /// Calendar date in Europe/London at the start of acquisition.
    pub observation_date: NaiveDate,
    /// UTC instant at the start of acquisition.
    pub started_at: DateTime<Utc>,
}

/// Historical candidate profile; optional source values remain absent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberProfile {
    /// Parliament's numeric identity.
    pub parliament_member_id: i32,
    /// Source display name, without normalization.
    pub name: String,
    /// Latest party identity when supplied.
    pub party_id: Option<i32>,
    /// Latest party name when supplied.
    pub party_name: Option<String>,
    /// Latest House, which may be Lords for a former MP.
    pub latest_house: i16,
    /// Latest membership location when supplied.
    pub latest_membership_from: Option<String>,
}

/// One source House membership, before any term clipping or deduplication.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct HouseMembership {
    /// Commons (1) or Lords (2).
    pub house: i16,
    /// Source calendar start date.
    pub start_date: NaiveDate,
    /// Date service ceased, if supplied.
    pub end_date: Option<NaiveDate>,
}

/// A returned history, retaining its identity for completeness checks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberHistory {
    /// Parliament's numeric identity.
    pub parliament_member_id: i32,
    /// All returned House membership periods, including identical repeats.
    pub house_memberships: Vec<HouseMembership>,
}

/// Complete source observations, with no database identities or derived service.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberObservations {
    /// Distinct historical candidate profiles.
    pub profiles: Vec<MemberProfile>,
    /// Distinct identities observed in the current Commons search.
    pub current_commons: Vec<i32>,
    /// Exactly one history for every candidate.
    pub histories: Vec<MemberHistory>,
}

/// A member capture ready to persist or interpret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberCapture {
    /// Original term and observation context.
    pub context: CaptureContext,
    /// Complete source observations.
    pub observations: MemberObservations,
}

/// Row counts for the three raw datasets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureCounts {
    /// Historical candidate profiles.
    pub profiles: usize,
    /// Current Commons identities.
    pub current_commons: usize,
    /// All source House membership periods.
    pub house_memberships: usize,
}

impl MemberObservations {
    /// Counts before service filtering and deduplication.
    #[must_use]
    pub fn counts(&self) -> CaptureCounts {
        CaptureCounts {
            profiles: self.profiles.len(),
            current_commons: self.current_commons.len(),
            house_memberships: self
                .histories
                .iter()
                .map(|h| h.house_memberships.len())
                .sum(),
        }
    }

    /// Check identities and completeness without applying service rules.
    ///
    /// # Errors
    /// Rejects duplicate, invalid, extra or missing source identities and empty histories.
    pub fn validate(&self) -> Result<(), EntityIngestionError> {
        let mut candidates = BTreeSet::new();
        for profile in &self.profiles {
            let id = profile.parliament_member_id;
            if id <= 0 || !candidates.insert(id) {
                return Err(invalid(format!(
                    "member {id}: invalid or duplicate profile identity"
                )));
            }
        }
        let mut current = BTreeSet::new();
        for id in &self.current_commons {
            if !current.insert(*id) || !candidates.contains(id) {
                return Err(invalid(format!(
                    "current Commons member {id}: duplicate or missing from historical candidates"
                )));
            }
        }
        let mut histories = BTreeSet::new();
        for history in &self.histories {
            let id = history.parliament_member_id;
            if !histories.insert(id)
                || !candidates.contains(&id)
                || history.house_memberships.is_empty()
            {
                return Err(invalid(format!(
                    "member {id}: duplicate, unexpected or empty history"
                )));
            }
        }
        if histories != candidates {
            return Err(invalid("histories do not contain all requested member IDs"));
        }
        Ok(())
    }
}

/// Validated member fields for PostgreSQL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberRecord {
    /// Parliament's numeric identity, used for reconciliation.
    pub parliament_member_id: i32,
    /// Source display name.
    pub name: String,
    /// Required latest party identity.
    pub party_id: i32,
    /// Required latest party name.
    pub party_name: String,
    /// Latest House from the historical candidate profile.
    pub latest_house: i16,
    /// Required latest membership location.
    pub latest_membership_from: String,
    /// Status from the captured current Commons search.
    pub is_current_commons: bool,
}

/// A distinct Commons interval in the configured term.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServicePeriod {
    /// Source start date, the stable identity of an interval.
    pub source_start_date: NaiveDate,
    /// Source end date, also used as served-until.
    pub source_end_date: Option<NaiveDate>,
    /// Later of source start and term start.
    pub served_from: NaiveDate,
}

/// One accepted profile and its complete replacement set of term service.
#[derive(Clone, Debug)]
pub struct MemberImport {
    /// Validated profile to reconcile.
    pub member: MemberRecord,
    /// Distinct and non-overlapping Commons periods.
    pub periods: Vec<ServicePeriod>,
}

/// Entire refresh validated before storage receives it.
#[derive(Clone, Debug)]
pub struct MemberRefresh {
    /// Captured Parliament start.
    pub term_start: NaiveDate,
    /// Eligible members; absent members must remain untouched.
    pub members: Vec<MemberImport>,
    /// Candidates with no qualifying Commons service.
    pub excluded_candidates: usize,
}

/// Committed profile outcomes and cohort/service counts.
#[derive(Clone, Debug, Default, Serialize)]
pub struct MemberImportSummary {
    /// Number of eligible members processed.
    pub members: usize,
    /// Current Commons members.
    pub current_commons: usize,
    /// Former Commons members.
    pub former_commons: usize,
    /// Newly inserted profiles.
    pub inserted: usize,
    /// Changed profiles.
    pub updated: usize,
    /// Identical profiles.
    pub unchanged: usize,
    /// Distinct term service periods reconciled.
    pub service_periods: usize,
    /// Candidates excluded for lack of term service.
    pub excluded_candidates: usize,
}

impl MemberCapture {
    /// Interpret all observations before making any database writes.
    ///
    /// # Errors
    /// Rejects incomplete captures, invalid fields and inconsistent Commons service.
    pub fn prepare_refresh(&self) -> Result<MemberRefresh, EntityIngestionError> {
        self.observations.validate()?;
        let term_start = self.context.term_start;
        let as_of = self.context.observation_date;
        if term_start > as_of {
            return Err(invalid("term start is after observation date"));
        }
        let current: BTreeSet<_> = self.observations.current_commons.iter().copied().collect();
        let histories: BTreeMap<_, _> = self
            .observations
            .histories
            .iter()
            .map(|h| (h.parliament_member_id, h))
            .collect();
        let mut refresh = MemberRefresh {
            term_start,
            members: Vec::new(),
            excluded_candidates: 0,
        };
        for profile in &self.observations.profiles {
            let id = profile.parliament_member_id;
            let is_current = current.contains(&id);
            let periods = service_periods(histories[&id], term_start, as_of, is_current)?;
            if periods.is_empty() {
                refresh.excluded_candidates += 1;
                continue;
            }
            let member = profile.for_database(is_current)?;
            refresh.members.push(MemberImport { member, periods });
        }
        if refresh.members.is_empty() {
            return Err(invalid(
                "no Commons service was found for the configured term",
            ));
        }
        Ok(refresh)
    }
}

impl MemberProfile {
    fn for_database(&self, is_current_commons: bool) -> Result<MemberRecord, EntityIngestionError> {
        let id = self.parliament_member_id;
        let required =
            |field: &str| invalid(format!("member {id}: {field} is required for PostgreSQL"));
        if self.name.trim().is_empty() {
            return Err(required("name"));
        }
        if !matches!(self.latest_house, 1 | 2) || (is_current_commons && self.latest_house != 1) {
            return Err(invalid(format!("member {id}: inconsistent latest House")));
        }
        Ok(MemberRecord {
            parliament_member_id: id,
            name: self.name.clone(),
            party_id: self
                .party_id
                .filter(|id| *id > 0)
                .ok_or_else(|| required("party_id"))?,
            party_name: self
                .party_name
                .clone()
                .ok_or_else(|| required("party_name"))?,
            latest_house: self.latest_house,
            latest_membership_from: self
                .latest_membership_from
                .clone()
                .ok_or_else(|| required("latest_membership_from"))?,
            is_current_commons,
        })
    }
}

fn service_periods(
    history: &MemberHistory,
    term_start: NaiveDate,
    as_of: NaiveDate,
    current: bool,
) -> Result<Vec<ServicePeriod>, EntityIngestionError> {
    let id = history.parliament_member_id;
    let mut memberships = BTreeSet::new();
    for membership in &history.house_memberships {
        if !matches!(membership.house, 1 | 2)
            || membership
                .end_date
                .is_some_and(|end| end < membership.start_date)
        {
            return Err(invalid(format!(
                "member {id}: invalid House or membership date ordering"
            )));
        }
        if membership.house == 1
            && membership.start_date <= as_of
            && membership.end_date.is_none_or(|end| end > term_start)
        {
            memberships.insert(membership);
        }
    }
    let periods: Vec<_> = memberships
        .into_iter()
        .map(|m| ServicePeriod {
            source_start_date: m.start_date,
            source_end_date: m.end_date,
            served_from: m.start_date.max(term_start),
        })
        .collect();
    for pair in periods.windows(2) {
        if pair[0].source_start_date == pair[1].source_start_date {
            return Err(invalid(format!("member {id}: conflicting service starts")));
        }
        if pair[0]
            .source_end_date
            .is_none_or(|end| pair[1].served_from < end)
        {
            return Err(invalid(format!("member {id}: overlapping service periods")));
        }
    }
    let active = periods
        .iter()
        .any(|p| p.source_end_date.is_none_or(|end| end > as_of));
    if active != current {
        return Err(invalid(format!(
            "member {id}: history disagrees with current Commons search"
        )));
    }
    Ok(periods)
}

pub(crate) fn invalid(message: impl Into<String>) -> EntityIngestionError {
    EntityIngestionError::DataError(message.into())
}
