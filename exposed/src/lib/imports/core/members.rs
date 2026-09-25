use chrono::NaiveDate;

use super::{ImportError, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HouseMembership {
    pub house: i16,
    pub start: NaiveDate,
    pub end: Option<NaiveDate>,
}

#[derive(Clone, Debug)]
pub(crate) struct MemberHistory {
    pub id: i32,
    pub memberships: Vec<HouseMembership>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ServicePeriod {
    pub source_start: NaiveDate,
    pub served_from: NaiveDate,
    pub end: Option<NaiveDate>,
}

impl MemberHistory {
    pub fn service(
        &self,
        term: NaiveDate,
        as_of: NaiveDate,
        current: bool,
    ) -> Result<Vec<ServicePeriod>> {
        let mut periods: Vec<_> = self
            .memberships
            .iter()
            .filter(|m| m.house == 1 && m.start <= as_of && m.end.is_none_or(|end| end > term))
            .map(|m| ServicePeriod {
                source_start: m.start,
                served_from: m.start.max(term),
                end: m.end,
            })
            .collect();
        periods.sort_by_key(|p| p.source_start);
        periods.dedup();
        if periods
            .iter()
            .any(|p| p.end.is_some_and(|end| end < p.served_from))
        {
            return Err(ImportError::Invalid(format!(
                "Member {}: service ends before it starts",
                self.id
            )));
        }
        for pair in periods.windows(2) {
            if pair[0].source_start == pair[1].source_start
                || pair[0].end.is_none_or(|end| pair[1].served_from < end)
            {
                return Err(ImportError::Invalid(format!(
                    "Member {}: conflicting or overlapping service periods",
                    self.id
                )));
            }
        }
        if periods.iter().any(|p| p.end.is_none_or(|end| end > as_of)) != current {
            return Err(ImportError::Invalid(format!(
                "Member {}: history disagrees with current search",
                self.id
            )));
        }
        Ok(periods)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MemberProfile {
    pub id: i32,
    pub name: String,
    pub party_id: Option<i32>,
    pub party_name: Option<String>,
    pub house: i16,
    pub membership_from: Option<String>,
}

impl MemberProfile {
    pub fn validate(&self) -> Result<()> {
        if self.id <= 0
            || self.name.trim().is_empty()
            || ![1, 2].contains(&self.house)
            || self.party_id.is_some_and(|id| id <= 0)
        {
            return Err(ImportError::Invalid("Invalid member profile".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Member {
    profile: MemberProfile,
    current: bool,
    periods: Vec<ServicePeriod>,
}

impl Member {
    pub fn profile(&self) -> &MemberProfile {
        &self.profile
    }
    pub fn is_current(&self) -> bool {
        self.current
    }
    pub fn periods(&self) -> &[ServicePeriod] {
        &self.periods
    }
    pub fn from_history(
        profile: MemberProfile,
        history: &MemberHistory,
        term: NaiveDate,
        as_of: NaiveDate,
        current: bool,
    ) -> Result<Self> {
        profile.validate()?;
        if profile.id != history.id || (current && profile.house != 1) {
            return Err(ImportError::Invalid(
                "Inconsistent member identity or latest House".into(),
            ));
        }
        if history.memberships.is_empty()
            || history
                .memberships
                .iter()
                .any(|m| ![1, 2].contains(&m.house))
        {
            return Err(ImportError::Invalid("Invalid membership history".into()));
        }
        let periods = history.service(term, as_of, current)?;
        Ok(Self {
            profile,
            current,
            periods,
        })
    }
}

pub(crate) fn validate_configured_term(
    term: NaiveDate,
    stored: impl IntoIterator<Item = NaiveDate>,
) -> Result<()> {
    if stored.into_iter().any(|date| date != term) {
        return Err(ImportError::Invalid(
            "This database contains another term; rollover requires an explicit migration".into(),
        ));
    }
    Ok(())
}
