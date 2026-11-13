//! Exact rankings use the same aggregate as member summaries.
//! Companies includes every canonical non-individual kind, including unclassified funders.
use crate::domain::models::reporting_period::{ReportingContext, ReportingPeriod};
use crate::domain::{
    models::{
        declared_totals::PeriodAggregate,
        rankings::{Ranking, RankingDeclaration, Rankings},
    },
    repositories::rankings::{RankingsError, RankingsRepo},
};
use std::collections::BTreeMap;
/// Complete rankings for a selected registration period.
pub trait RankingsService: Clone + Send + Sync + 'static {
    /// Returns all four boards from one snapshot.
    fn rankings(
        &self,
        period: ReportingPeriod,
    ) -> impl Future<Output = Result<Rankings, RankingsError>> + Send;
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
fn ordered<K: Ord, I>(groups: BTreeMap<K, (I, PeriodAggregate)>) -> Vec<Ranking<I>> {
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
    rows.into_iter().map(|(_, row)| row).collect()
}
impl<R: RankingsRepo> RankingsService for Service<R> {
    async fn rankings(&self, period: ReportingPeriod) -> Result<Rankings, RankingsError> {
        let context = ReportingContext::at(chrono::Utc::now());
        let sources = self
            .repo
            .ranking_declarations(context.window(period))
            .await?;
        Ok(summarize(sources, period, context))
    }
}
pub(crate) fn summarize(
    sources: Vec<RankingDeclaration>,
    period: ReportingPeriod,
    context: ReportingContext,
) -> Rankings {
    let mut members = BTreeMap::new();
    let mut individuals = BTreeMap::new();
    let mut companies = BTreeMap::new();
    let mut parties = BTreeMap::new();
    for source in sources {
        let declaration = source.declaration;
        if !context.window(period).includes(declaration.registered_at) {
            continue;
        }
        members
            .entry(source.member.id.value())
            .or_insert_with(|| (source.member, PeriodAggregate::default()))
            .1
            .observe(&declaration);
        let party = parties
            .entry(source.party.id)
            .or_insert_with(|| (source.party.clone(), PeriodAggregate::default()));
        if let Some(name) = source.party.name
            && party.0.name.as_ref().is_none_or(|current| &name < current)
        {
            party.0.name = Some(name);
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
            let board = if funder.kind.is_individual() {
                &mut individuals
            } else {
                &mut companies
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
        period,
        current_term_start: context.current_term_start(),
        members: ordered(members),
        individuals: ordered(individuals),
        companies: ordered(companies),
        parties: ordered(parties),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::entity_details::{DeclarationCategory, DeclaredGbpPeriodTotal};
    use crate::domain::models::funder::FunderKind;
    #[test]
    fn all_categories_have_matching_member_party_and_funder_totals() {
        use crate::domain::models::{
            entity_details::{DeclaredFunding, FunderReference, MemberDeclaration},
            funder::FunderId,
            parliament_member::MemberId,
            rankings::{RankedFunder, RankedMember, RankedParty},
        };
        for category_id in (1..=12).chain([987]) {
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
                let context = ReportingContext::at(chrono::Utc::now());
                let rankings = summarize(
                    vec![RankingDeclaration {
                        member: RankedMember {
                            id: MemberId::new(1).unwrap(),
                            name: "Member".into(),
                            party_name: Some("Party".into()),
                            membership_from: Some("Constituency".into()),
                        },
                        party: RankedParty {
                            id: Some(1),
                            name: Some("Party".into()),
                        },
                        declaration: MemberDeclaration {
                            source_id: 1,
                            category: DeclarationCategory::from_source_id(category_id),
                            category_name: "Support".into(),
                            registered_at: Some(context.current_term_start()),
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
                    ReportingPeriod::CurrentTerm,
                    context,
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
                let member = &rankings.members[0].total;
                let party = &rankings.parties[0].total;
                let funder = &rankings
                    .individuals
                    .iter()
                    .chain(&rankings.companies)
                    .next()
                    .unwrap()
                    .total;
                for total in [member, party, funder] {
                    assert_eq!(
                        total.amount,
                        Some("1".parse().unwrap()),
                        "category {category_id}, {kind:?}"
                    );
                    assert_eq!(total.declaration_count, 1);
                    assert!(!total.incomplete);
                }
            }
        }
    }
    #[test]
    fn exact_sort_preserves_complete_board_with_precise_ties_zero_negative_and_null() {
        let mut groups = BTreeMap::new();
        for id in 1..=25 {
            let mut aggregate = PeriodAggregate::default();
            let declaration = crate::domain::models::entity_details::MemberDeclaration {
                source_id: id,
                category: DeclarationCategory::Other(987),
                category_name: "Other".into(),
                registered_at: Some(ReportingContext::at(chrono::Utc::now()).current_term_start()),
                entries: vec![crate::domain::models::entity_details::DeclaredFunding {
                    funder: None,
                    payment_type: None,
                    amount: match id {
                        25 => Some("9007199254740993.123456790".parse().unwrap()),
                        24 => None,
                        23 => Some("-1".parse().unwrap()),
                        22 => Some("0".parse().unwrap()),
                        _ => Some("9007199254740993.123456789".parse().unwrap()),
                    },
                    currency: Some("GBP".into()),
                }],
            };
            aggregate.observe(&declaration);
            groups.insert(id, (id, aggregate));
        }
        let rows = ordered(groups);
        assert_eq!(
            rows.iter().map(|row| row.identity).collect::<Vec<_>>(),
            std::iter::once(25)
                .chain(1..=21)
                .chain([22, 23, 24])
                .collect::<Vec<_>>()
        );
        let total: &DeclaredGbpPeriodTotal = &rows[0].total;
        assert_eq!(
            total.amount,
            Some("9007199254740993.123456790".parse().unwrap())
        );
    }
    #[test]
    fn party_names_use_available_evidence_in_both_declaration_orders() {
        use crate::domain::models::{
            entity_details::{DeclaredFunding, MemberDeclaration},
            parliament_member::MemberId,
            rankings::{RankedMember, RankedParty},
        };
        for names in [
            [Some("Available party"), None],
            [None, Some("Available party")],
        ] {
            let context = ReportingContext::at(chrono::Utc::now());
            let sources = names
                .into_iter()
                .enumerate()
                .map(|(index, name)| RankingDeclaration {
                    member: RankedMember {
                        id: MemberId::new(u32::try_from(index + 1).unwrap()).unwrap(),
                        name: format!("Member {index}"),
                        party_name: name.map(str::to_string),
                        membership_from: None,
                    },
                    party: RankedParty {
                        id: Some(7),
                        name: name.map(str::to_string),
                    },
                    declaration: MemberDeclaration {
                        source_id: i32::try_from(index + 1).unwrap(),
                        category: DeclarationCategory::from_source_id(1),
                        category_name: "Employment".into(),
                        registered_at: Some(context.current_term_start()),
                        entries: vec![DeclaredFunding {
                            funder: None,
                            amount: Some("1.23456789".parse().unwrap()),
                            currency: Some("GBP".into()),
                            payment_type: None,
                        }],
                    },
                    funders: BTreeMap::new(),
                })
                .collect();
            let rankings = summarize(sources, ReportingPeriod::CurrentTerm, context);
            assert_eq!(rankings.parties.len(), 1);
            assert_eq!(
                rankings.parties[0].identity.name.as_deref(),
                Some("Available party")
            );
            assert_eq!(
                rankings.parties[0].total.amount,
                Some("2.46913578".parse().unwrap())
            );
            assert_eq!(rankings.parties[0].total.declaration_count, 2);
            assert_eq!(rankings.members.len(), 2);
        }
    }
}
