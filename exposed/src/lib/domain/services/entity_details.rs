//! Entity profiles and summaries from a shared exact aggregate.

mod interface;
pub use interface::EntityDetailsService;

pub use crate::domain::models::entity_details::EntityDetailsError;
use crate::domain::{
    models::entity_details::{
        CalendarYear, CategoryTotal, CurrencyBreakdown, DeclarationCategory,
        DeclaredGbpPeriodTotal, DeclaredGbpTotals, FunderDetails, FunderDetailsRequest,
        FunderFunding, MemberCurrencyTotal, MemberDeclaration, MemberDetails, MemberDetailsRequest,
        MemberHistory, MemberSummary, PartyTotal, RecipientAllocation,
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
            .map(|history| summarize_member(history, CalendarYear::containing(chrono::Utc::now())))
            .ok_or(EntityDetailsError::NotFound)
    }
    async fn funder(
        &self,
        request: FunderDetailsRequest,
    ) -> Result<FunderDetails, EntityDetailsError> {
        self.repo
            .funder_funding(request.id())
            .await?
            .map(summarize_funder)
            .ok_or(EntityDetailsError::NotFound)
    }
}

fn add_known(total: &mut Option<bigdecimal::BigDecimal>, amount: Option<&bigdecimal::BigDecimal>) {
    if let Some(amount) = amount {
        match total {
            Some(total) => *total += amount,
            None => *total = Some(amount.clone()),
        }
    }
}

#[derive(Default)]
struct PeriodAggregate {
    currencies: BTreeMap<String, MemberCurrencyTotal>,
    categories: BTreeMap<(String, DeclarationCategory, String), CategoryTotal>,
    declaration_count: usize,
    entry_count: usize,
    unknown_currency_count: usize,
}

impl PeriodAggregate {
    fn observe(&mut self, declaration: &MemberDeclaration) {
        self.declaration_count += 1;
        for entry in &declaration.entries {
            self.entry_count += 1;
            let Some(currency) = &entry.currency else {
                self.unknown_currency_count += 1;
                continue;
            };
            let group =
                self.currencies
                    .entry(currency.clone())
                    .or_insert_with(|| MemberCurrencyTotal {
                        currency: currency.clone(),
                        amount: None,
                        entry_count: 0,
                        unknown_amount_count: 0,
                        categories: Vec::new(),
                    });
            add_known(&mut group.amount, entry.amount.as_ref());
            group.entry_count += 1;
            group.unknown_amount_count += usize::from(entry.amount.is_none());
            let category = self
                .categories
                .entry((
                    currency.clone(),
                    declaration.category,
                    declaration.category_name.clone(),
                ))
                .or_insert_with(|| CategoryTotal {
                    category: declaration.category,
                    category_name: declaration.category_name.clone(),
                    amount: None,
                    entry_count: 0,
                    unknown_amount_count: 0,
                });
            add_known(&mut category.amount, entry.amount.as_ref());
            category.entry_count += 1;
            category.unknown_amount_count += usize::from(entry.amount.is_none());
        }
    }

    fn gbp_total(&self) -> DeclaredGbpPeriodTotal {
        let gbp = self.currencies.get("GBP");
        DeclaredGbpPeriodTotal {
            amount: gbp.and_then(|group| group.amount.clone()),
            declaration_count: self.declaration_count,
            incomplete: self.unknown_currency_count > 0
                || gbp.is_some_and(|group| group.unknown_amount_count > 0),
        }
    }

    fn finish(mut self) -> Vec<MemberCurrencyTotal> {
        for ((currency, _, _), category) in self.categories {
            if let Some(group) = self.currencies.get_mut(&currency) {
                group.categories.push(category);
            }
        }
        self.currencies.into_values().collect()
    }
}

struct CurrentParliamentStart(chrono::DateTime<chrono::Utc>);
impl CurrentParliamentStart {
    fn new() -> Self {
        use chrono::TimeZone;
        Self(
            chrono::Utc
                .with_ymd_and_hms(2024, 7, 9, 0, 0, 0)
                .single()
                .expect("valid parliament start"),
        )
    }
    fn includes(&self, declaration: &MemberDeclaration) -> bool {
        declaration.registered_at.is_some_and(|date| date >= self.0)
    }
}

fn summarize_member(history: MemberHistory, calendar_year: CalendarYear) -> MemberDetails {
    let boundary = CurrentParliamentStart::new();
    let mut historical = PeriodAggregate::default();
    let mut current_term = PeriodAggregate::default();
    let mut current_year = PeriodAggregate::default();
    let mut first_registered_at = None;
    let mut last_registered_at = None;
    let mut undated_declaration_count = 0;
    for declaration in &history.declarations {
        historical.observe(declaration);
        if calendar_year.includes(declaration.registered_at) {
            current_year.observe(declaration);
        }
        if boundary.includes(declaration) {
            current_term.observe(declaration);
        }
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
        declared_totals: DeclaredGbpTotals {
            calendar_year,
            current_year: current_year.gbp_total(),
            current_term_start: boundary.0,
            historical: historical.gbp_total(),
            current_term: current_term.gbp_total(),
        },
        entry_count: historical.entry_count,
        unknown_currency_count: historical.unknown_currency_count,
        currencies: historical.finish(),
        first_registered_at,
        last_registered_at,
        undated_declaration_count,
    };
    MemberDetails {
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

fn summarize_funder(funding: FunderFunding) -> FunderDetails {
    let mut entry_count = 0;
    let mut unknown_currency_count = 0;
    let mut currencies: BTreeMap<String, Vec<RecipientAllocation>> = BTreeMap::new();
    let mut recipients: BTreeMap<(String, u32), RecipientAllocation> = BTreeMap::new();
    for source in &funding.declarations {
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
    for ((currency, _), allocation) in recipients {
        currencies.entry(currency).or_default().push(allocation);
    }
    let currencies = currencies
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
        .collect();
    FunderDetails {
        declarations: funding.declarations,
        profile: funding.profile,
        currencies,
        entry_count,
        unknown_currency_count,
        recipient_limit: TOP_RECIPIENTS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::entity_details::{DeclaredFunding, MemberProfile};

    #[test]
    fn calendar_year_totals_use_utc_boundaries_and_preserve_coverage() {
        let declaration =
            |date: Option<&str>, amount: Option<&str>, currency: Option<&str>| MemberDeclaration {
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
            };
        let history = MemberHistory {
            member: MemberProfile {
                id: "1".parse().unwrap(),
                name: "Example".into(),
                party_id: 1,
                party_name: "Party".into(),
                membership_from: "Example".into(),
                is_current_commons: true,
            },
            declarations: vec![
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
            ],
        };
        let year = CalendarYear::containing("2025-06-01T00:00:00Z".parse().unwrap());
        let details = summarize_member(history.clone(), year);
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
            summarize_member(missing, year)
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
            summarize_member(unknown, year)
                .summary
                .declared_totals
                .current_year
                .incomplete
        );
        assert!(!year.includes(None));
        assert!(!year.includes(Some("2025-12-31T23:00:00-02:00".parse().unwrap())));
    }
}
