//! Valid capture identities, complete observations and accepted Commons service.
use super::{entity_ingestion::EntityIngestionError, parliament_member::ParliamentMember};
use crate::domain::repositories::parliament_api::MemberHistory;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    str::FromStr,
};
use uuid::Uuid;

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

/// Valid observation context fixed when acquisition begins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureContext {
    capture_id: CaptureId,
    term_start: NaiveDate,
    observation_date: NaiveDate,
    started_at: DateTime<Utc>,
}
impl CaptureContext {
    /// Create the context used for both acquisition and offline interpretation.
    ///
    /// # Errors
    /// Rejects a term starting after the observation date.
    pub fn new(
        capture_id: CaptureId,
        term_start: NaiveDate,
        observation_date: NaiveDate,
        started_at: DateTime<Utc>,
    ) -> Result<Self, EntityIngestionError> {
        if term_start > observation_date {
            return Err(invalid("term start is after observation date"));
        }
        Ok(Self {
            capture_id,
            term_start,
            observation_date,
            started_at,
        })
    }
    /// Identity allocated for this capture.
    #[must_use]
    pub const fn capture_id(&self) -> CaptureId {
        self.capture_id
    }
    /// Beginning of the Parliament cohort.
    #[must_use]
    pub const fn term_start(&self) -> NaiveDate {
        self.term_start
    }
    /// Fixed calendar date in Europe/London.
    #[must_use]
    pub const fn observation_date(&self) -> NaiveDate {
        self.observation_date
    }
    /// UTC instant at the start of acquisition.
    #[must_use]
    pub const fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }
}

/// Complete source observations, checked together at construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberObservations {
    profiles: Vec<ParliamentMember>,
    current_commons: BTreeSet<i32>,
    histories: BTreeMap<i32, MemberHistory>,
}
impl MemberObservations {
    /// Accept complete candidate, current and history observations.
    ///
    /// # Errors
    /// Rejects duplicate, missing or unexpected identities in any dataset.
    pub fn new(
        profiles: Vec<ParliamentMember>,
        current_commons: Vec<i32>,
        histories: Vec<MemberHistory>,
    ) -> Result<Self, EntityIngestionError> {
        let candidates: BTreeSet<_> = profiles
            .iter()
            .map(ParliamentMember::parliament_member_id)
            .collect();
        if candidates.len() != profiles.len() {
            return Err(invalid("duplicate historical candidate identity"));
        }
        let current_count = current_commons.len();
        let current_ids: BTreeSet<_> = current_commons.into_iter().collect();
        if current_ids.len() != current_count || !current_ids.is_subset(&candidates) {
            return Err(invalid(
                "current Commons IDs are duplicated or missing from historical candidates",
            ));
        }
        let mut by_id = BTreeMap::new();
        for history in histories {
            let id = history.parliament_member_id();
            if by_id.insert(id, history).is_some() {
                return Err(invalid(format!("member {id}: duplicate history")));
            }
        }
        if by_id.keys().copied().collect::<BTreeSet<_>>() != candidates {
            return Err(invalid("histories must match requested member IDs exactly"));
        }
        Ok(Self {
            profiles,
            current_commons: current_ids,
            histories: by_id,
        })
    }
    /// Historical profiles in source order.
    #[must_use]
    pub fn profiles(&self) -> &[ParliamentMember] {
        &self.profiles
    }
    /// Distinct current Commons identities.
    #[must_use]
    pub const fn current_commons(&self) -> &BTreeSet<i32> {
        &self.current_commons
    }
    /// Exactly one history for each candidate, ordered by identity.
    #[must_use]
    pub const fn histories(&self) -> &BTreeMap<i32, MemberHistory> {
        &self.histories
    }
}

/// A complete capture with an explicit observation context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberCapture {
    context: CaptureContext,
    observations: MemberObservations,
}
impl MemberCapture {
    /// Combine valid context and complete observations.
    #[must_use]
    pub const fn new(context: CaptureContext, observations: MemberObservations) -> Self {
        Self {
            context,
            observations,
        }
    }
    /// Original acquisition context.
    #[must_use]
    pub const fn context(&self) -> &CaptureContext {
        &self.context
    }
    /// Complete raw observations.
    #[must_use]
    pub const fn observations(&self) -> &MemberObservations {
        &self.observations
    }
}

/// A distinct Commons interval accepted for a particular Parliament.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServicePeriod {
    source_start_date: NaiveDate,
    source_end_date: Option<NaiveDate>,
    served_from: NaiveDate,
}
impl ServicePeriod {
    /// Original start used to reconcile interval identity.
    #[must_use]
    pub const fn source_start_date(&self) -> NaiveDate {
        self.source_start_date
    }
    /// Original date service ceased, also used as served-until.
    #[must_use]
    pub const fn source_end_date(&self) -> Option<NaiveDate> {
        self.source_end_date
    }
    /// Later of source start and Parliament start.
    #[must_use]
    pub const fn served_from(&self) -> NaiveDate {
        self.served_from
    }
}

/// One member's nonempty, consistent Commons service for a Parliament.
#[derive(Clone, Debug)]
pub struct CommonsService {
    member: ParliamentMember,
    is_current_commons: bool,
    periods: Vec<ServicePeriod>,
}
impl CommonsService {
    /// Member with these Commons service periods.
    #[must_use]
    pub const fn member(&self) -> &ParliamentMember {
        &self.member
    }
    /// Status observed in the current Commons search, consistent with this service.
    #[must_use]
    pub const fn is_current_commons(&self) -> bool {
        self.is_current_commons
    }
    /// Distinct and non-overlapping periods of service.
    #[must_use]
    pub fn periods(&self) -> &[ServicePeriod] {
        &self.periods
    }
}

/// Entire accepted cohort, constructed before storage receives it.
#[derive(Clone, Debug)]
pub struct MemberRefresh {
    term_start: NaiveDate,
    members: Vec<CommonsService>,
}
impl MemberRefresh {
    /// Construct the eligible cohort using the saved observation context.
    ///
    /// # Errors
    /// Rejects conflicting service, inconsistent current status or an empty cohort.
    pub fn from_capture(capture: &MemberCapture) -> Result<Self, EntityIngestionError> {
        let term_start = capture.context().term_start();
        let as_of = capture.context().observation_date();
        let observations = capture.observations();
        let mut members = Vec::new();
        for profile in observations.profiles() {
            let id = profile.parliament_member_id();
            let current = observations.current_commons().contains(&id);
            if current && profile.latest_house() != 1 {
                return Err(invalid(format!(
                    "member {id}: current Commons status disagrees with latest_house"
                )));
            }
            let periods =
                service_periods(&observations.histories()[&id], term_start, as_of, current)?;
            if periods.is_empty() {
                continue;
            }
            members.push(CommonsService {
                member: profile.clone(),
                is_current_commons: current,
                periods,
            });
        }
        if members.is_empty() {
            return Err(invalid(
                "no Commons service was found for the configured term",
            ));
        }
        Ok(Self {
            term_start,
            members,
        })
    }
    /// Parliament shared by every accepted member's service.
    #[must_use]
    pub const fn term_start(&self) -> NaiveDate {
        self.term_start
    }
    /// Accepted members and their complete Commons service.
    #[must_use]
    pub fn members(&self) -> &[CommonsService] {
        &self.members
    }
}

fn service_periods(
    history: &MemberHistory,
    term_start: NaiveDate,
    as_of: NaiveDate,
    current: bool,
) -> Result<Vec<ServicePeriod>, EntityIngestionError> {
    let id = history.parliament_member_id();
    let memberships: BTreeSet<_> = history
        .house_memberships()
        .iter()
        .filter(|m| {
            m.house() == 1
                && m.start_date() <= as_of
                && m.end_date().is_none_or(|end| end > term_start)
        })
        .collect();
    let periods: Vec<_> = memberships
        .into_iter()
        .map(|m| ServicePeriod {
            source_start_date: m.start_date(),
            source_end_date: m.end_date(),
            served_from: m.start_date().max(term_start),
        })
        .collect();
    for pair in periods.windows(2) {
        if pair[0].source_start_date() == pair[1].source_start_date() {
            return Err(invalid(format!("member {id}: conflicting service starts")));
        }
        if pair[0]
            .source_end_date()
            .is_none_or(|end| pair[1].served_from() < end)
        {
            return Err(invalid(format!("member {id}: overlapping service periods")));
        }
    }
    let active = periods
        .iter()
        .any(|p| p.source_end_date().is_none_or(|end| end > as_of));
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
