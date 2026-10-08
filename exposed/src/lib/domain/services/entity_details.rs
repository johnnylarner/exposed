//! Entity profiles and summaries from a shared exact aggregate.

mod interface;
pub use interface::EntityDetailsService;

use crate::domain::{
    models::{
        entity_details::{
            CurrencyBreakdown, FunderDetails, FunderFunding, MemberDetails, PartyTotal,
            RecipientAllocation,
        },
        funder::FunderId,
        parliament_member::MemberId,
    },
    repositories::entity_details::{EntityDetailsRepo, EntityDetailsRepoError},
};
use std::{cmp::Ordering, collections::BTreeMap};
use thiserror::Error;

const RECENT_DECLARATIONS: usize = 20;
const TOP_RECIPIENTS: usize = 10;

/// Entity detail lookup failure.
#[derive(Debug, Error)]
pub enum EntityDetailsError {
    /// No stored identity matches the request.
    #[error("entity not found")]
    NotFound,
    /// Private database diagnostics.
    #[error(transparent)]
    Unexpected(#[from] EntityDetailsRepoError),
}

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
    async fn member(&self, id: MemberId) -> Result<MemberDetails, EntityDetailsError> {
        self.repo
            .member(id, RECENT_DECLARATIONS)
            .await?
            .ok_or(EntityDetailsError::NotFound)
    }
    async fn funder(&self, id: FunderId) -> Result<FunderDetails, EntityDetailsError> {
        self.repo
            .funder_funding(id)
            .await?
            .map(summarize_funder)
            .ok_or(EntityDetailsError::NotFound)
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
    for allocation in funding.allocations {
        entry_count += allocation.entry_count;
        if let Some(currency) = &allocation.currency {
            currencies
                .entry(currency.clone())
                .or_default()
                .push(allocation);
        } else {
            unknown_currency_count += allocation.entry_count;
        }
    }
    let currencies = currencies
        .into_iter()
        .map(|(currency, mut allocations)| {
            let mut parties: BTreeMap<(i32, String), PartyTotal> = BTreeMap::new();
            for allocation in &allocations {
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
                currency,
                parties,
                top_recipients: allocations,
            }
        })
        .collect();
    FunderDetails {
        profile: funding.profile,
        currencies,
        entry_count,
        unknown_currency_count,
        recipient_limit: TOP_RECIPIENTS,
    }
}
