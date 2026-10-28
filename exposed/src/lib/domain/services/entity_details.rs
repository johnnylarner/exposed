//! Entity profiles and summaries from a shared exact aggregate.

mod interface;
pub use interface::EntityDetailsService;

pub use crate::domain::models::entity_details::EntityDetailsError;
use crate::domain::models::reporting_period::{ReportingContext, ReportingPeriod};
use crate::domain::{
    models::{
        declared_totals::{PeriodAggregate, add_known},
        entity_details::{
            CalendarYear, CurrencyBreakdown, DeclaredGbpTotals, FunderDetails,
            FunderDetailsRequest, FunderFunding, MemberDeclaration, MemberDetails,
            MemberDetailsRequest, MemberHistory, MemberSummary, PartyTotal, RecipientAllocation,
        },
    },
    repositories::entity_details::EntityDetailsRepo,
};
use std::{cmp::Ordering, collections::BTreeMap};

const TOP_RECIPIENTS: usize = 10;

/// Detail service owns summary rules and presentation limits.
#[derive(Clone)]
pub struct Service<R> {
    repo: R,
}

impl<R> Service<R> {
    /// Creates a detail service.
    #[must_use]
    pub const fn new(repo: R) -> Self {
        Self { repo }
    }
}

impl<R: EntityDetailsRepo> EntityDetailsService for Service<R> {
    async fn member(
        &self,
        request: MemberDetailsRequest,
    ) -> Result<MemberDetails, EntityDetailsError> {
        self.repo
            .member(request.id())
            .await?
            .map(|history| {
                summarize_member(
                    history,
                    request.period(),
                    ReportingContext::at(chrono::Utc::now()),
                )
            })
            .ok_or(EntityDetailsError::NotFound)
    }
    async fn funder(
        &self,
        request: FunderDetailsRequest,
    ) -> Result<FunderDetails, EntityDetailsError> {
        self.repo
            .funder_funding(request.id())
            .await?
            .map(|funding| {
                summarize_funder(
                    funding,
                    request.period(),
                    ReportingContext::at(chrono::Utc::now()),
                )
            })
            .ok_or(EntityDetailsError::NotFound)
    }
}

struct DeclaredTotalsAggregate {
    historical: PeriodAggregate,
    current_term: PeriodAggregate,
    current_year: PeriodAggregate,
    calendar_year: CalendarYear,
    context: ReportingContext,
}

impl DeclaredTotalsAggregate {
    fn new(context: ReportingContext) -> Self {
        Self {
            historical: PeriodAggregate::default(),
            current_term: PeriodAggregate::default(),
            current_year: PeriodAggregate::default(),
            calendar_year: context.calendar_year(),
            context,
        }
    }

    fn observe(&mut self, declaration: &MemberDeclaration) {
        self.historical.observe(declaration);
        if self
            .context
            .window(ReportingPeriod::CurrentYear)
            .includes(declaration.registered_at)
        {
            self.current_year.observe(declaration);
        }
        if self
            .context
            .window(ReportingPeriod::CurrentTerm)
            .includes(declaration.registered_at)
        {
            self.current_term.observe(declaration);
        }
    }

    fn totals(&self) -> DeclaredGbpTotals {
        DeclaredGbpTotals {
            calendar_year: self.calendar_year,
            current_year: self.current_year.gbp_total(),
            current_term_start: self.context.current_term_start(),
            historical: self.historical.gbp_total(),
            current_term: self.current_term.gbp_total(),
        }
    }
}

fn complete_totals<'a>(
    declarations: impl Iterator<Item = &'a MemberDeclaration>,
    context: ReportingContext,
) -> DeclaredGbpTotals {
    let mut aggregate = DeclaredTotalsAggregate::new(context);
    for declaration in declarations {
        aggregate.observe(declaration);
    }
    aggregate.totals()
}

fn summarize_member(
    mut history: MemberHistory,
    period: ReportingPeriod,
    context: ReportingContext,
) -> MemberDetails {
    let declared_totals = complete_totals(history.declarations.iter(), context);
    history
        .declarations
        .retain(|declaration| context.window(period).includes(declaration.registered_at));
    let mut selected = PeriodAggregate::default();
    let mut first_registered_at = None;
    let mut last_registered_at = None;
    let mut undated_declaration_count = 0;
    for declaration in &history.declarations {
        selected.observe(declaration);
        if let Some(date) = declaration.registered_at {
            first_registered_at = Some(
                first_registered_at
                    .map_or(date, |first: chrono::DateTime<chrono::Utc>| first.min(date)),
            );
            last_registered_at = Some(
                last_registered_at
                    .map_or(date, |last: chrono::DateTime<chrono::Utc>| last.max(date)),
            );
        } else {
            undated_declaration_count += 1;
        }
    }
    let summary = MemberSummary {
        declared_totals,
        entry_count: selected.entry_count,
        unknown_currency_count: selected.unknown_currency_count,
        currencies: selected.finish(),
        first_registered_at,
        last_registered_at,
        undated_declaration_count,
    };
    MemberDetails {
        period,
        member: history.member,
        declarations: history.declarations,
        summary,
    }
}

fn compare_allocations(left: &RecipientAllocation, right: &RecipientAllocation) -> Ordering {
    right
        .known_total
        .cmp(&left.known_total)
        .then_with(|| left.member.id.value().cmp(&right.member.id.value()))
        .then_with(|| left.member.name.cmp(&right.member.name))
}

fn summarize_funder(
    mut funding: FunderFunding,
    period: ReportingPeriod,
    context: ReportingContext,
) -> FunderDetails {
    let declared_totals = complete_totals(
        funding
            .declarations
            .iter()
            .map(|source| &source.declaration),
        context,
    );
    funding.declarations.retain(|source| {
        context
            .window(period)
            .includes(source.declaration.registered_at)
    });
    let mut first_registered_at = None;
    let mut entry_count = 0;
    let mut unknown_currency_count = 0;
    let mut recipients: BTreeMap<(String, u32), RecipientAllocation> = BTreeMap::new();
    for source in &funding.declarations {
        if let Some(date) = source.declaration.registered_at {
            first_registered_at = Some(
                first_registered_at
                    .map_or(date, |first: chrono::DateTime<chrono::Utc>| first.min(date)),
            );
        }
        for entry in &source.declaration.entries {
            entry_count += 1;
            let Some(currency) = &entry.currency else {
                unknown_currency_count += 1;
                continue;
            };
            let allocation = recipients
                .entry((currency.clone(), source.member.id.value()))
                .or_insert_with(|| RecipientAllocation {
                    member: source.member.clone(),
                    currency: Some(currency.clone()),
                    known_total: None,
                    entry_count: 0,
                    unknown_amount_count: 0,
                });
            add_known(&mut allocation.known_total, entry.amount.as_ref());
            allocation.entry_count += 1;
            allocation.unknown_amount_count += i64::from(entry.amount.is_none());
        }
    }
    let currencies = currency_breakdowns(recipients);
    FunderDetails {
        period,
        declared_totals,
        first_registered_at,
        declarations: funding.declarations,
        profile: funding.profile,
        currencies,
        entry_count,
        unknown_currency_count,
        recipient_limit: TOP_RECIPIENTS,
    }
}

fn currency_breakdowns(
    recipients: BTreeMap<(String, u32), RecipientAllocation>,
) -> Vec<CurrencyBreakdown> {
    let mut currencies: BTreeMap<String, Vec<RecipientAllocation>> = BTreeMap::new();
    for ((currency, _), allocation) in recipients {
        currencies.entry(currency).or_default().push(allocation);
    }
    currencies
        .into_iter()
        .map(|(currency, mut allocations)| {
            let mut amount = None;
            let mut parties: BTreeMap<(i32, String), PartyTotal> = BTreeMap::new();
            for allocation in &allocations {
                add_known(&mut amount, allocation.known_total.as_ref());
                let party = parties
                    .entry((
                        allocation.member.party_id,
                        allocation.member.party_name.clone(),
                    ))
                    .or_insert_with(|| PartyTotal {
                        party_id: allocation.member.party_id,
                        party_name: allocation.member.party_name.clone(),
                        amount: None,
                        entry_count: 0,
                        unknown_amount_count: 0,
                    });
                if let Some(amount) = &allocation.known_total {
                    if let Some(total) = &mut party.amount {
                        *total += amount;
                    } else {
                        party.amount = Some(amount.clone());
                    }
                }
                party.entry_count += allocation.entry_count;
                party.unknown_amount_count += allocation.unknown_amount_count;
            }
            let mut parties: Vec<_> = parties.into_values().collect();
            parties.sort_by(|left, right| {
                right
                    .amount
                    .cmp(&left.amount)
                    .then_with(|| left.party_name.cmp(&right.party_name))
                    .then_with(|| left.party_id.cmp(&right.party_id))
            });
            allocations.sort_by(compare_allocations);
            allocations.truncate(TOP_RECIPIENTS);
            CurrencyBreakdown {
                amount,
                currency,
                parties,
                top_recipients: allocations,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::entity_details::{
        DeclarationCategory, DeclaredFunding, MemberDeclaration, MemberProfile,
    };

    fn declaration(
        date: Option<&str>,
        amount: Option<&str>,
        currency: Option<&str>,
    ) -> MemberDeclaration {
        MemberDeclaration {
            source_id: 1,
            category: DeclarationCategory::Other(987),
            category_name: "Unknown category".into(),
            registered_at: date.map(|date| date.parse().unwrap()),
            entries: vec![DeclaredFunding {
                funder: None,
                payment_type: None,
                amount: amount.map(|amount| amount.parse().unwrap()),
                currency: currency.map(str::to_owned),
            }],
        }
    }

    fn history(declarations: Vec<MemberDeclaration>) -> MemberHistory {
        MemberHistory {
            member: MemberProfile {
                id: "1".parse().unwrap(),
                name: "Example".into(),
                party_id: 1,
                party_name: "Party".into(),
                membership_from: "Example".into(),
                is_current_commons: true,
            },
            declarations,
        }
    }

    fn funder_history(history: MemberHistory) -> FunderFunding {
        use crate::domain::models::{
            entity_details::{FunderDeclaration, FunderProfile},
            funder::{Funder, FunderKind},
        };
        let id = "10000000-0000-0000-0000-000000000001".parse().unwrap();
        FunderFunding {
            profile: FunderProfile {
                funder: Funder::new(id, "Example".into(), FunderKind::Company),
                company_number: None,
                aliases: Vec::new(),
                member_ids: vec![id],
                alias_links: Vec::new(),
                family_links: Vec::new(),
            },
            declarations: history
                .declarations
                .into_iter()
                .map(|declaration| FunderDeclaration {
                    member: history.member.clone(),
                    declaration,
                })
                .collect(),
        }
    }

    fn assert_periods_equal(left: &DeclaredGbpTotals, right: &DeclaredGbpTotals) {
        assert_eq!(left.calendar_year.value(), right.calendar_year.value());
        assert_eq!(left.current_term_start, right.current_term_start);
        for (left, right) in [
            (&left.historical, &right.historical),
            (&left.current_term, &right.current_term),
            (&left.current_year, &right.current_year),
        ] {
            assert_eq!(left.amount, right.amount);
            assert_eq!(left.declaration_count, right.declaration_count);
            assert_eq!(left.incomplete, right.incomplete);
        }
    }

    fn summarize_both(history: MemberHistory, year: CalendarYear) -> MemberDetails {
        let context =
            ReportingContext::at(format!("{}-06-01T00:00:00Z", year.value()).parse().unwrap());
        let funder = summarize_funder(
            funder_history(history.clone()),
            ReportingPeriod::Historical,
            context,
        );
        let member = summarize_member(history, ReportingPeriod::Historical, context);
        assert_periods_equal(&member.summary.declared_totals, &funder.declared_totals);
        assert_eq!(
            member.summary.first_registered_at,
            funder.first_registered_at
        );
        member
    }

    #[test]
    fn funder_and_member_share_term_boundaries_and_matching_occurrence_totals() {
        let year = CalendarYear::containing("2025-06-01T00:00:00Z".parse().unwrap());
        let mut duplicate =
            declaration(Some("2024-07-09T01:00:00+01:00"), Some("4.25"), Some("GBP"));
        duplicate.entries.push(duplicate.entries[0].clone());
        let details = summarize_both(
            history(vec![
                declaration(
                    Some("2024-07-08T23:59:59.999999Z"),
                    Some("-20"),
                    Some("GBP"),
                ),
                duplicate,
                declaration(None, Some("3"), Some("GBP")),
            ]),
            year,
        );
        assert_eq!(
            details.summary.declared_totals.current_term.amount,
            Some("8.5".parse().unwrap())
        );
        assert_eq!(
            details
                .summary
                .declared_totals
                .current_term
                .declaration_count,
            1
        );
        assert_eq!(
            details.summary.declared_totals.historical.amount,
            Some("-8.5".parse().unwrap())
        );
        assert_eq!(
            details.summary.declared_totals.historical.declaration_count,
            3
        );
        assert_eq!(details.summary.entry_count, 4);
    }

    #[test]
    fn both_entities_classify_offset_registrations_by_utc_year() {
        let year = CalendarYear::containing("2025-06-01T00:00:00Z".parse().unwrap());
        let details = summarize_both(
            history(vec![
                declaration(Some("2024-12-31T23:00:00-02:00"), Some("1.25"), Some("GBP")),
                declaration(Some("2025-01-01T00:30:00+01:00"), Some("10"), Some("GBP")),
                declaration(Some("2025-12-31T23:00:00-02:00"), Some("100"), Some("GBP")),
                declaration(Some("2026-01-01T00:30:00+01:00"), Some("2.75"), Some("GBP")),
            ]),
            year,
        );
        assert_eq!(
            details.summary.declared_totals.current_year.amount,
            Some("4".parse().unwrap())
        );
        assert_eq!(
            details
                .summary
                .declared_totals
                .current_year
                .declaration_count,
            2
        );
    }

    #[test]
    fn calendar_year_totals_use_utc_boundaries_and_preserve_coverage() {
        let history = history(vec![
            declaration(
                Some("2024-12-31T23:59:59.999999Z"),
                Some("100"),
                Some("GBP"),
            ),
            declaration(
                Some("2025-01-01T00:00:00Z"),
                Some("9007199254740993.123456789"),
                Some("GBP"),
            ),
            declaration(
                Some("2025-12-31T23:59:59.999999Z"),
                Some("-9007199254740993.023456789"),
                Some("GBP"),
            ),
            declaration(Some("2025-02-01T00:00:00Z"), None, Some("EUR")),
            declaration(Some("2026-01-01T00:00:00Z"), None, Some("GBP")),
            declaration(None, Some("50"), None),
        ]);
        let year = CalendarYear::containing("2025-06-01T00:00:00Z".parse().unwrap());
        let details = summarize_both(history.clone(), year);
        let total = details.summary.declared_totals.current_year;
        assert_eq!(total.amount, Some("0.1".parse().unwrap()));
        assert_eq!(total.declaration_count, 3);
        assert!(!total.incomplete);
        assert_eq!(
            details.summary.declared_totals.historical.declaration_count,
            6
        );
        assert!(details.summary.declared_totals.historical.incomplete);
        let mut missing = history.clone();
        missing
            .declarations
            .push(declaration(Some("2025-07-01T00:00:00Z"), None, Some("GBP")));
        assert!(
            summarize_both(missing, year)
                .summary
                .declared_totals
                .current_year
                .incomplete
        );
        let mut unknown = history;
        unknown
            .declarations
            .push(declaration(Some("2025-07-01T00:00:00Z"), Some("8"), None));
        assert!(
            summarize_both(unknown, year)
                .summary
                .declared_totals
                .current_year
                .incomplete
        );
    }

    #[test]
    fn calendar_year_totals_distinguish_empty_zero_and_unavailable() {
        let year = CalendarYear::containing("2025-06-01T00:00:00Z".parse().unwrap());
        let summarize = |declarations| {
            summarize_both(history(declarations), year)
                .summary
                .declared_totals
                .current_year
        };
        let empty = summarize(Vec::new());
        assert_eq!(empty.amount, None);
        assert_eq!(empty.declaration_count, 0);
        assert!(!empty.incomplete);
        let zero = summarize(vec![declaration(
            Some("2025-01-01T00:00:00Z"),
            Some("0"),
            Some("GBP"),
        )]);
        assert_eq!(zero.amount, Some("0".parse().unwrap()));
        assert_eq!(zero.declaration_count, 1);
        assert!(!zero.incomplete);
        let foreign = summarize(vec![declaration(
            Some("2025-01-01T00:00:00Z"),
            None,
            Some("EUR"),
        )]);
        assert_eq!(foreign.amount, None);
        assert_eq!(foreign.declaration_count, 1);
        assert!(!foreign.incomplete);
        let missing = summarize(vec![declaration(
            Some("2025-01-01T00:00:00Z"),
            None,
            Some("GBP"),
        )]);
        assert_eq!(missing.amount, None);
        assert_eq!(missing.declaration_count, 1);
        assert!(missing.incomplete);
        let mut no_entries = declaration(Some("2025-01-01T00:00:00Z"), None, None);
        no_entries.entries.clear();
        let no_entries = summarize(vec![no_entries]);
        assert_eq!(no_entries.amount, None);
        assert_eq!(no_entries.declaration_count, 1);
        assert!(!no_entries.incomplete);
        assert_eq!(year.value(), 2025);
        assert_eq!(
            CalendarYear::containing("2024-12-31T23:00:00-02:00".parse().unwrap()).value(),
            2025
        );
        assert!(year.includes(Some("2024-12-31T23:00:00-02:00".parse().unwrap())));
        assert!(!year.includes(None));
        assert!(!year.includes(Some("2025-12-31T23:00:00-02:00".parse().unwrap())));
    }
    #[test]
    fn selected_projection_scopes_every_ancillary_field_without_changing_headlines() {
        let context = ReportingContext::at("2025-01-01T00:00:00Z".parse().unwrap());
        let mut duplicate =
            declaration(Some("2025-12-31T23:59:59.999999Z"), Some("0"), Some("GBP"));
        duplicate.entries.push(duplicate.entries[0].clone());
        let source = history(vec![
            declaration(None, Some("100"), None),
            declaration(Some("2024-07-08T23:59:59.999999Z"), None, Some("GBP")),
            declaration(Some("2024-07-09T01:00:00+01:00"), Some("-2"), Some("GBP")),
            declaration(Some("2025-01-01T00:30:00+01:00"), Some("10"), Some("EUR")),
            declaration(
                Some("2024-12-31T23:00:00-02:00"),
                Some("9007199254740993.123456789"),
                Some("GBP"),
            ),
            duplicate,
            declaration(Some("2026-01-01T00:00:00Z"), Some("3"), Some("GBP")),
        ]);
        let complete = summarize_member(source.clone(), ReportingPeriod::Historical, context);
        for (period, declarations, entries, unknown, undated) in [
            (ReportingPeriod::Historical, 7, 8, 1, 1),
            (ReportingPeriod::CurrentTerm, 5, 6, 0, 0),
            (ReportingPeriod::CurrentYear, 2, 3, 0, 0),
        ] {
            let member = summarize_member(source.clone(), period, context);
            let funder = summarize_funder(funder_history(source.clone()), period, context);
            assert_eq!(member.period, period);
            assert_eq!(funder.period, period);
            assert_eq!(member.declarations.len(), declarations);
            assert_eq!(funder.declarations.len(), declarations);
            assert_eq!(member.summary.entry_count, entries);
            assert_eq!(funder.entry_count, i64::try_from(entries).unwrap());
            assert_eq!(member.summary.unknown_currency_count, unknown);
            assert_eq!(
                funder.unknown_currency_count,
                i64::try_from(unknown).unwrap()
            );
            assert_eq!(member.summary.undated_declaration_count, undated);
            assert_periods_equal(
                &complete.summary.declared_totals,
                &member.summary.declared_totals,
            );
            assert_periods_equal(&complete.summary.declared_totals, &funder.declared_totals);
            let selected = context.window(period);
            assert!(
                member
                    .declarations
                    .iter()
                    .all(|d| selected.includes(d.registered_at))
            );
            let gbp = member
                .summary
                .currencies
                .iter()
                .find(|group| group.currency == "GBP")
                .unwrap();
            let funder_gbp = funder
                .currencies
                .iter()
                .find(|group| group.currency == "GBP")
                .unwrap();
            assert_eq!(gbp.amount, funder_gbp.amount);
            assert_eq!(gbp.categories[0].category, DeclarationCategory::Other(987));
            assert_eq!(
                funder_gbp.parties[0].entry_count,
                i64::try_from(gbp.entry_count).unwrap()
            );
            assert_eq!(
                funder_gbp.top_recipients[0].entry_count,
                i64::try_from(gbp.entry_count).unwrap()
            );
            assert_eq!(
                member.summary.first_registered_at,
                funder.first_registered_at
            );
            if period == ReportingPeriod::CurrentYear {
                assert_eq!(member.summary.currencies.len(), 1);
                assert_eq!(
                    gbp.amount,
                    Some("9007199254740993.123456789".parse().unwrap())
                );
                assert_eq!(
                    member.summary.last_registered_at,
                    Some("2025-12-31T23:59:59.999999Z".parse().unwrap())
                );
            }
        }
    }
    #[test]
    fn funder_filters_before_recipient_limit_and_party_aggregation() {
        let context = ReportingContext::at("2025-06-01T00:00:00Z".parse().unwrap());
        let mut source = funder_history(history(Vec::new()));
        for id in 1..=12 {
            let mut member = history(Vec::new()).member;
            member.id = id.to_string().parse().unwrap();
            member.party_id = id;
            member.party_name = format!("Party {id}");
            source
                .declarations
                .push(crate::domain::models::entity_details::FunderDeclaration {
                    member: member.clone(),
                    declaration: declaration(
                        Some("2024-08-01T00:00:00Z"),
                        Some("100"),
                        Some("GBP"),
                    ),
                });
            if id == 12 {
                source.declarations.push(
                    crate::domain::models::entity_details::FunderDeclaration {
                        member,
                        declaration: declaration(
                            Some("2025-12-31T23:59:59Z"),
                            Some("1"),
                            Some("GBP"),
                        ),
                    },
                );
            }
        }
        source.declarations.last_mut().unwrap().declaration.entries[0].amount =
            Some("-1000".parse().unwrap());
        source
            .declarations
            .iter_mut()
            .find(|d| d.member.id.value() == 12)
            .unwrap()
            .declaration
            .entries[0]
            .amount = Some("-2000".parse().unwrap());
        let term = summarize_funder(source.clone(), ReportingPeriod::CurrentTerm, context);
        assert_eq!(term.currencies[0].top_recipients.len(), 10);
        assert!(
            term.currencies[0]
                .top_recipients
                .iter()
                .all(|r| r.member.id.value() != 12)
        );
        let year = summarize_funder(source, ReportingPeriod::CurrentYear, context);
        assert_eq!(year.currencies[0].top_recipients.len(), 1);
        assert_eq!(year.currencies[0].top_recipients[0].member.id.value(), 12);
        assert_eq!(year.currencies[0].amount, Some("-1000".parse().unwrap()));
        assert_eq!(year.currencies[0].parties.len(), 1);
        assert_eq!(year.currencies[0].parties[0].party_id, 12);
        assert_periods_equal(&term.declared_totals, &year.declared_totals);
    }
}
