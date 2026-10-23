//! Exact current-term rankings use the same aggregate as member summaries.
//! Companies includes every canonical non-individual kind, including unclassified funders.
use crate::domain::{
    models::{
        declared_totals::{CurrentParliamentStart, PeriodAggregate},
        entity_details::DeclarationCategory,
        funder::FunderKind,
        rankings::{Ranking, RankingDeclaration, Rankings},
    },
    repositories::rankings::{RankingsError, RankingsRepo},
};
use std::collections::BTreeMap;
/// Fixed current-term top-ten operation.
pub trait RankingsService: Clone + Send + Sync + 'static {
    /// Returns all four boards from one snapshot.
    fn rankings(&self) -> impl Future<Output = Result<Rankings, RankingsError>> + Send;
}
/// Rankings application service.
#[derive(Clone)]
pub struct Service<R> {
    repo: R,
}
impl<R> Service<R> {
    /// Creates a rankings service.
    pub const fn new(repo: R) -> Self {
        Self { repo }
    }
}
fn top<K: Ord, I>(groups: BTreeMap<K, (I, PeriodAggregate)>) -> Vec<Ranking<I>> {
    let mut rows: Vec<_> = groups
        .into_iter()
        .map(|(key, (identity, aggregate))| {
            (
                key,
                Ranking {
                    identity,
                    total: aggregate.gbp_total(),
                },
            )
        })
        .collect();
    rows.sort_by(|left, right| {
        right
            .1
            .total
            .amount
            .cmp(&left.1.total.amount)
            .then_with(|| left.0.cmp(&right.0))
    });
    rows.into_iter().take(10).map(|(_, row)| row).collect()
}
impl<R: RankingsRepo> RankingsService for Service<R> {
    async fn rankings(&self) -> Result<Rankings, RankingsError> {
        let boundary = CurrentParliamentStart::new();
        let sources = self.repo.ranking_declarations(boundary.value()).await?;
        Ok(summarize(sources, boundary))
    }
}
fn summarize(sources: Vec<RankingDeclaration>, boundary: CurrentParliamentStart) -> Rankings {
    let mut members = BTreeMap::new();
    let mut individuals = BTreeMap::new();
    let mut companies = BTreeMap::new();
    let mut parties = BTreeMap::new();
    for source in sources {
        let declaration = source.declaration;
        if !boundary.includes(&declaration) {
            continue;
        }
        members
            .entry(source.member.id.value())
            .or_insert_with(|| (source.member, PeriodAggregate::default()))
            .1
            .observe(&declaration);
        if declaration.category != DeclarationCategory::DonationSupport {
            continue;
        }
        let party = parties
            .entry(source.party.id)
            .or_insert_with(|| (source.party.clone(), PeriodAggregate::default()));
        if source.party.name < party.0.name {
            party.0.name = source.party.name;
        }
        party.1.observe(&declaration);
        let mut funders = BTreeMap::new();
        for entry in &declaration.entries {
            if let Some(funder) = entry
                .funder
                .as_ref()
                .and_then(|reference| source.funders.get(&reference.id.value()))
            {
                funders
                    .entry(funder.id.value())
                    .or_insert_with(|| (funder.clone(), Vec::new()))
                    .1
                    .push(entry.clone());
            }
        }
        for (key, (funder, entries)) in funders {
            let board = match funder.kind {
                FunderKind::Individual => &mut individuals,
                FunderKind::Company
                | FunderKind::Trust
                | FunderKind::TradeUnion
                | FunderKind::Other
                | FunderKind::NotSpecified
                | FunderKind::BuildingSociety
                | FunderKind::LimitedLiabilityPartnership
                | FunderKind::UnincorporatedAssociation
                | FunderKind::FriendlySociety
                | FunderKind::RegisteredParty => &mut companies,
            };
            let mut projected = declaration.clone();
            projected.entries = entries;
            board
                .entry(key)
                .or_insert_with(|| (funder, PeriodAggregate::default()))
                .1
                .observe(&projected);
        }
    }
    Rankings {
        current_term_start: boundary.value(),
        members: top(members),
        individuals: top(individuals),
        companies: top(companies),
        parties: top(parties),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::entity_details::DeclaredGbpPeriodTotal;
    #[test]
    fn companies_include_every_non_individual_kind() {
        use crate::domain::models::{
            entity_details::{DeclaredFunding, FunderReference, MemberDeclaration},
            funder::FunderId,
            parliament_member::MemberId,
            rankings::{RankedFunder, RankedMember, RankedParty},
        };
        for (kind, individual) in [
            (FunderKind::Individual, true),
            (FunderKind::Company, false),
            (FunderKind::Trust, false),
            (FunderKind::TradeUnion, false),
            (FunderKind::Other, false),
            (FunderKind::NotSpecified, false),
            (FunderKind::BuildingSociety, false),
            (FunderKind::LimitedLiabilityPartnership, false),
            (FunderKind::UnincorporatedAssociation, false),
            (FunderKind::FriendlySociety, false),
            (FunderKind::RegisteredParty, false),
        ] {
            let id: FunderId = "10000000-0000-0000-0000-000000000001".parse().unwrap();
            let boundary = CurrentParliamentStart::new();
            let rankings = summarize(
                vec![RankingDeclaration {
                    member: RankedMember {
                        id: MemberId::new(1).unwrap(),
                        name: "Member".into(),
                        party_name: "Party".into(),
                        membership_from: "Constituency".into(),
                    },
                    party: RankedParty {
                        id: 1,
                        name: "Party".into(),
                    },
                    declaration: MemberDeclaration {
                        source_id: 1,
                        category: DeclarationCategory::DonationSupport,
                        category_name: "Support".into(),
                        registered_at: Some(boundary.value()),
                        entries: vec![DeclaredFunding {
                            funder: Some(FunderReference {
                                id,
                                name: "Funder".into(),
                            }),
                            amount: Some("1".parse().unwrap()),
                            currency: Some("GBP".into()),
                            payment_type: None,
                        }],
                    },
                    funders: BTreeMap::from([(
                        id.value(),
                        RankedFunder {
                            id,
                            name: "Funder".into(),
                            kind: kind.clone(),
                        },
                    )]),
                }],
                boundary,
            );
            assert_eq!(
                rankings.individuals.len(),
                usize::from(individual),
                "{kind:?}"
            );
            assert_eq!(
                rankings.companies.len(),
                usize::from(!individual),
                "{kind:?}"
            );
        }
    }
    #[test]
    fn exact_sort_is_descending_null_last_stable_and_limited_to_ten() {
        let mut groups = BTreeMap::new();
        for id in 1..=15 {
            let mut aggregate = PeriodAggregate::default();
            let declaration = crate::domain::models::entity_details::MemberDeclaration {
                source_id: id,
                category: DeclarationCategory::Other(987),
                category_name: "Other".into(),
                registered_at: Some(CurrentParliamentStart::new().value()),
                entries: vec![crate::domain::models::entity_details::DeclaredFunding {
                    funder: None,
                    payment_type: None,
                    amount: match id {
                        15 => Some("9007199254740993.123456790".parse().unwrap()),
                        14 => None,
                        _ => Some("9007199254740993.123456789".parse().unwrap()),
                    },
                    currency: Some("GBP".into()),
                }],
            };
            aggregate.observe(&declaration);
            groups.insert(id, (id, aggregate));
        }
        let rows = top(groups);
        assert_eq!(
            rows.iter().map(|row| row.identity).collect::<Vec<_>>(),
            std::iter::once(15).chain(1..=9).collect::<Vec<_>>()
        );
        let total: &DeclaredGbpPeriodTotal = &rows[0].total;
        assert_eq!(
            total.amount,
            Some("9007199254740993.123456790".parse().unwrap())
        );
    }
}
