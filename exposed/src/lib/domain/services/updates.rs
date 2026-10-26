//! Distinct monthly declaration activity.
use crate::domain::{
    models::updates::{EntityUpdate, MonthlyUpdates, RegistrationMonth, UpdatedCompany},
    repositories::updates::{UpdatesError, UpdatesRepo},
};
use chrono::{DateTime, Utc};
use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroUsize,
};
/// Complete current-month operation with a request-owned clock instant.
pub trait UpdatesService: Clone + Send + Sync + 'static {
    /// Returns every identity updated in the instant's UTC calendar month.
    fn updates(
        &self,
        now: DateTime<Utc>,
    ) -> impl Future<Output = Result<MonthlyUpdates, UpdatesError>> + Send;
}
/// Monthly activity application service.
#[derive(Clone)]
pub struct Service<R> {
    repo: R,
}
impl<R> Service<R> {
    /// Creates the service.
    pub const fn new(repo: R) -> Self {
        Self { repo }
    }
}
struct Activity<I> {
    identity: I,
    sources: BTreeSet<i32>,
    latest: DateTime<Utc>,
}
fn observe<K: Ord, I>(
    groups: &mut BTreeMap<K, Activity<I>>,
    key: K,
    identity: I,
    source: i32,
    date: DateTime<Utc>,
) {
    let activity = groups.entry(key).or_insert_with(|| Activity {
        identity,
        sources: BTreeSet::new(),
        latest: date,
    });
    activity.sources.insert(source);
    activity.latest = activity.latest.max(date);
}
fn rows<K: Ord, I>(
    groups: BTreeMap<K, Activity<I>>,
    name: impl Fn(&I) -> &str,
) -> Vec<EntityUpdate<I>> {
    let mut rows: Vec<_> = groups.into_iter().collect();
    rows.sort_by(|a, b| {
        b.1.latest
            .cmp(&a.1.latest)
            .then_with(|| name(&a.1.identity).cmp(name(&b.1.identity)))
            .then_with(|| a.0.cmp(&b.0))
    });
    rows.into_iter()
        .map(|(_, activity)| EntityUpdate {
            identity: activity.identity,
            declaration_count: NonZeroUsize::new(activity.sources.len())
                .expect("observed activity has a source"),
            latest_registration: activity.latest,
        })
        .collect()
}
impl<R: UpdatesRepo> UpdatesService for Service<R> {
    async fn updates(&self, now: DateTime<Utc>) -> Result<MonthlyUpdates, UpdatesError> {
        let month = RegistrationMonth::containing(now);
        let declarations = self.repo.declarations(&month).await?;
        let mut members = BTreeMap::new();
        let mut companies = BTreeMap::new();
        for declaration in declarations {
            if !month.includes(declaration.registered_at) {
                continue;
            }
            observe(
                &mut members,
                declaration.member.id.value(),
                declaration.member,
                declaration.source_id,
                declaration.registered_at,
            );
            for funder in declaration.funders {
                if !funder.kind.is_individual() {
                    observe(
                        &mut companies,
                        funder.id.value(),
                        UpdatedCompany {
                            id: funder.id,
                            name: funder.name,
                        },
                        declaration.source_id,
                        declaration.registered_at,
                    );
                }
            }
        }
        Ok(MonthlyUpdates {
            month,
            members: rows(members, |identity| &identity.name),
            companies: rows(companies, |identity| &identity.name),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::{
        funder::{FunderId, FunderKind},
        parliament_member::MemberId,
        updates::{UpdateDeclaration, UpdateFunder, UpdatedMember},
    };
    fn date(value: &str) -> DateTime<Utc> {
        value.parse().unwrap()
    }
    #[test]
    fn calendar_boundaries_cover_rollover_leap_year_and_utc_offsets() {
        for (instant, start, end) in [
            (
                "2024-02-29T12:00:00Z",
                "2024-02-01T00:00:00Z",
                "2024-03-01T00:00:00Z",
            ),
            (
                "2026-12-31T23:59:59Z",
                "2026-12-01T00:00:00Z",
                "2027-01-01T00:00:00Z",
            ),
            (
                "2026-11-01T00:30:00+01:00",
                "2026-10-01T00:00:00Z",
                "2026-11-01T00:00:00Z",
            ),
        ] {
            let month = RegistrationMonth::containing(date(instant));
            assert_eq!(month.start(), date(start));
            assert_eq!(month.end(), date(end));
            assert!(month.includes(date(start)));
            assert!(!month.includes(date(end)));
        }
    }
    #[derive(Clone)]
    struct Repo;
    impl UpdatesRepo for Repo {
        fn declarations(
            &self,
            month: &RegistrationMonth,
        ) -> impl Future<Output = Result<Vec<UpdateDeclaration>, UpdatesError>> {
            assert_eq!(month.start(), date("2026-10-01T00:00:00Z"));
            let mut facts = Vec::new();
            for id in 1..=12 {
                for source in [id, id, id + 100] {
                    facts.push(UpdateDeclaration {
                        source_id: source,
                        registered_at: date(if source > 100 {
                            "2026-10-31T23:59:59Z"
                        } else {
                            "2026-10-01T00:00:00Z"
                        }),
                        member: UpdatedMember {
                            id: MemberId::new(u32::try_from(id).unwrap()).unwrap(),
                            name: if id < 3 {
                                "Same".into()
                            } else {
                                format!("Member {id:02}")
                            },
                        },
                        funders: vec![
                            UpdateFunder {
                                id: FunderId::new(uuid::Uuid::from_u128(1)),
                                name: "Company".into(),
                                kind: FunderKind::Trust,
                            },
                            UpdateFunder {
                                id: FunderId::new(uuid::Uuid::from_u128(2)),
                                name: "Individual".into(),
                                kind: FunderKind::Individual,
                            },
                        ],
                    });
                }
            }
            facts.push(UpdateDeclaration {
                source_id: 999,
                registered_at: month.end(),
                member: UpdatedMember {
                    id: MemberId::new(99).unwrap(),
                    name: "Outside".into(),
                },
                funders: vec![],
            });
            std::future::ready(Ok(facts))
        }
    }
    #[tokio::test]
    async fn service_counts_distinct_sources_retains_all_identities_and_includes_future_month_dates()
     {
        let result = Service::new(Repo)
            .updates(date("2026-10-02T12:00:00Z"))
            .await
            .unwrap();
        assert_eq!(result.members.len(), 12);
        assert!(
            result
                .members
                .iter()
                .all(|row| row.declaration_count.get() == 2)
        );
        assert_eq!(result.members[0].identity.name, "Member 03");
        assert_eq!(result.members[10].identity.id.value(), 1);
        assert_eq!(result.members[11].identity.id.value(), 2);
        assert_eq!(result.companies.len(), 1);
        assert_eq!(result.companies[0].declaration_count.get(), 24);
        assert_eq!(
            result.companies[0].latest_registration,
            date("2026-10-31T23:59:59Z")
        );
    }
}
