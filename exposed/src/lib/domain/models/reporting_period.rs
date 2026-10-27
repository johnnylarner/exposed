//! Canonical registration periods and their resolved UTC windows.
use super::entity_details::CalendarYear;
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

/// Selected registration scope; absence defaults to the parliamentary term.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReportingPeriod {
    /// All stored records, including undated declarations.
    Historical,
    /// Registrations from the current parliament's opening, without an upper bound.
    #[default]
    CurrentTerm,
    /// The complete current UTC calendar year, including future registrations.
    CurrentYear,
}

/// One captured clock context shared by headlines and selected data.
#[derive(Clone, Copy, Debug)]
pub struct ReportingContext {
    calendar_year: CalendarYear,
}
impl ReportingContext {
    /// Captures the calendar year at one instant.
    #[must_use]
    pub fn at(now: DateTime<Utc>) -> Self {
        Self {
            calendar_year: CalendarYear::containing(now),
        }
    }
    /// Captured UTC calendar year.
    #[must_use]
    pub const fn calendar_year(self) -> CalendarYear {
        self.calendar_year
    }
    /// Inclusive opening of the current parliament.
    ///
    /// # Panics
    /// Cannot panic: the fixed parliament opening is a valid UTC instant.
    #[must_use]
    pub fn current_term_start(self) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 7, 9, 0, 0, 0)
            .single()
            .expect("valid parliament start")
    }
    /// Resolves policy once; callers cannot construct arbitrary windows.
    ///
    /// # Panics
    /// Panics if the captured UTC year cannot be represented by chrono.
    #[must_use]
    pub fn window(self, period: ReportingPeriod) -> RegistrationWindow {
        let (start_inclusive, end_exclusive) = match period {
            ReportingPeriod::Historical => (None, None),
            ReportingPeriod::CurrentTerm => (Some(self.current_term_start()), None),
            ReportingPeriod::CurrentYear => {
                let start = Utc
                    .with_ymd_and_hms(self.calendar_year.value(), 1, 1, 0, 0, 0)
                    .single()
                    .expect("valid year start");
                let end = start.checked_add_months(chrono::Months::new(12));
                (Some(start), end)
            }
        };
        RegistrationWindow {
            start_inclusive,
            end_exclusive,
        }
    }
}

/// Half-open UTC registration window, constructible only through period policy.
#[derive(Clone, Copy, Debug)]
pub struct RegistrationWindow {
    start_inclusive: Option<DateTime<Utc>>,
    end_exclusive: Option<DateTime<Utc>>,
}
impl RegistrationWindow {
    /// Tests a registration, including undated records only for historical.
    #[must_use]
    pub fn includes(self, registered_at: Option<DateTime<Utc>>) -> bool {
        registered_at.map_or_else(
            || self.start_inclusive.is_none() && self.end_exclusive.is_none(),
            |date| {
                self.start_inclusive.is_none_or(|start| date >= start)
                    && self.end_exclusive.is_none_or(|end| date < end)
            },
        )
    }
    /// Inclusive lower bound for persistence queries.
    #[must_use]
    pub const fn start(self) -> Option<DateTime<Utc>> {
        self.start_inclusive
    }
    /// Exclusive upper bound for persistence queries.
    #[must_use]
    pub const fn end(self) -> Option<DateTime<Utc>> {
        self.end_exclusive
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_windows_preserve_exact_boundaries_and_full_year() {
        let context = ReportingContext::at("2025-01-01T00:00:00Z".parse().unwrap());
        let historical = context.window(ReportingPeriod::Historical);
        let term = context.window(ReportingPeriod::CurrentTerm);
        let year = context.window(ReportingPeriod::CurrentYear);
        assert!(historical.includes(None));
        assert!(!term.includes(None));
        assert!(!year.includes(None));
        for (date, in_term, in_year) in [
            ("2024-07-08T23:59:59.999999999Z", false, false),
            ("2024-07-09T01:00:00+01:00", true, false),
            ("2024-12-31T23:59:59.999999999Z", true, false),
            ("2024-12-31T23:00:00-01:00", true, true),
            ("2025-12-31T23:59:59.999999999Z", true, true),
            ("2026-01-01T00:00:00Z", true, false),
        ] {
            let date = Some(date.parse().unwrap());
            assert!(historical.includes(date));
            assert_eq!(term.includes(date), in_term);
            assert_eq!(year.includes(date), in_year);
        }
        for (wire, expected) in [
            ("historical", ReportingPeriod::Historical),
            ("current-term", ReportingPeriod::CurrentTerm),
            ("current-year", ReportingPeriod::CurrentYear),
        ] {
            assert_eq!(
                serde_json::from_str::<ReportingPeriod>(&format!("\"{wire}\"")).unwrap(),
                expected
            );
            assert_eq!(
                serde_json::to_string(&expected).unwrap(),
                format!("\"{wire}\"")
            );
        }
        assert!(serde_json::from_str::<ReportingPeriod>("\"\"").is_err());
    }
}
