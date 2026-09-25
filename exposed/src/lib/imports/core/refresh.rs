use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::Serialize;

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct Changes {
    pub inserted: usize,
    pub updated: usize,
    pub unchanged: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct RefreshState {
    pub last_completed: Option<DateTime<Utc>>,
    pub last_attempt: Option<DateTime<Utc>>,
    pub next_attempt: Option<DateTime<Utc>>,
    pub outcome: Option<String>,
    pub rejected: usize,
    pub failures: u32,
    pub new_members: Vec<i32>,
}

impl RefreshState {
    pub fn due(&self, now: DateTime<Utc>) -> bool {
        self.next_attempt.is_none_or(|next| now >= next)
    }
    pub fn completed(
        &mut self,
        started: DateTime<Utc>,
        ended: DateTime<Utc>,
        rejected: usize,
        new_members: Vec<i32>,
    ) {
        self.last_attempt = Some(started);
        self.last_completed = Some(ended);
        // Start-to-start avoids adding the duration of a full traversal to every interval.
        self.next_attempt = Some((started + Duration::days(1)).max(ended));
        self.outcome = Some(
            if rejected == 0 {
                "succeeded"
            } else {
                "warnings"
            }
            .into(),
        );
        self.rejected = rejected;
        self.failures = 0;
        self.new_members = new_members;
    }
    pub fn failed(&mut self, started: DateTime<Utc>, ended: DateTime<Utc>) {
        self.last_attempt = Some(started);
        self.failures = self.failures.saturating_add(1);
        let seconds = match self.failures {
            1 => 300,
            2 => 900,
            _ => 3600,
        };
        self.next_attempt = Some(ended + Duration::seconds(seconds));
        self.outcome = Some("failed".into());
    }
}

use super::{
    ImportError, Result,
    declarations::{Accepted, Declaration, Evidence},
    members::{Member, MemberProfile},
    ports::{DeclarationSource, ImportStore, MemberSource},
};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Serialize)]
pub(crate) struct MemberSummary {
    pub status: &'static str,
    pub term_start: NaiveDate,
    pub as_of: NaiveDate,
    pub members: usize,
    pub current_commons: usize,
    pub former_commons: usize,
    pub service_periods: usize,
    pub excluded_candidates: usize,
    #[serde(flatten)]
    pub changes: Changes,
}

fn unique(
    profiles: Vec<MemberProfile>,
    seen: &mut BTreeMap<i32, MemberProfile>,
) -> Result<Vec<MemberProfile>> {
    let mut new = Vec::new();
    for profile in profiles {
        profile.validate()?;
        if let Some(previous) = seen.get(&profile.id) {
            if previous != &profile {
                return Err(ImportError::Invalid(format!(
                    "Conflicting duplicate member {}",
                    profile.id
                )));
            }
            eprintln!(
                "Duplicate member {}; identical profile collapsed",
                profile.id
            );
        } else {
            seen.insert(profile.id, profile.clone());
            new.push(profile);
        }
    }
    Ok(new)
}

pub(crate) async fn current_members(
    source: &impl MemberSource,
    term: NaiveDate,
    as_of: NaiveDate,
) -> Result<BTreeMap<i32, MemberProfile>> {
    let mut seen = BTreeMap::new();
    let mut offset = 0;
    loop {
        let page = source.profiles(term, as_of, true, offset).await?;
        unique(page.entries, &mut seen)?;
        match page.next {
            Some(next) => offset = next,
            None => break,
        }
    }
    Ok(seen)
}

pub(crate) async fn refresh_members(
    term: NaiveDate,
    as_of: NaiveDate,
    source: &impl MemberSource,
    store: &impl ImportStore,
) -> Result<MemberSummary> {
    if term > as_of {
        return Err(ImportError::Invalid(
            "Term start cannot be after the import date".into(),
        ));
    }
    let current = current_members(source, term, as_of).await?;
    let mut seen = BTreeMap::new();
    let mut result = MemberSummary {
        status: "succeeded",
        term_start: term,
        as_of,
        members: 0,
        current_commons: 0,
        former_commons: 0,
        service_periods: 0,
        excluded_candidates: 0,
        changes: Changes::default(),
    };
    let mut offset = 0;
    loop {
        let page = source.profiles(term, as_of, false, offset).await?;
        let profiles = unique(page.entries, &mut seen)?;
        if !profiles.is_empty() {
            let ids: Vec<_> = profiles.iter().map(|p| p.id).collect();
            let mut histories = HashMap::new();
            for history in source.histories(&ids).await? {
                if histories.insert(history.id, history).is_some() {
                    return Err(ImportError::Invalid("Duplicate member history".into()));
                }
            }
            if histories.len() != ids.len() || ids.iter().any(|id| !histories.contains_key(id)) {
                return Err(ImportError::Invalid(
                    "History response does not contain all requested member IDs".into(),
                ));
            }
            let mut members = Vec::new();
            for profile in profiles {
                let history = &histories[&profile.id];
                let is_current = current.contains_key(&profile.id);
                let member = Member::from_history(profile, history, term, as_of, is_current)?;
                if member.periods().is_empty() {
                    result.excluded_candidates += 1;
                } else {
                    members.push(member);
                }
            }
            let changes = store.member_batch(term, &members).await?;
            result.changes.inserted += changes.inserted;
            result.changes.updated += changes.updated;
            result.changes.unchanged += changes.unchanged;
            for member in members {
                result.members += 1;
                result.current_commons += usize::from(member.is_current());
                result.former_commons += usize::from(!member.is_current());
                result.service_periods += member.periods().len();
            }
            eprintln!("Committed member batch ({} processed total)", seen.len());
        }
        match page.next {
            Some(next) => offset = next,
            None => break,
        }
    }
    if current.keys().any(|id| !seen.contains_key(id)) {
        return Err(ImportError::Invalid(
            "Current search contains members missing from historical search".into(),
        ));
    }
    if result.members == 0 {
        return Err(ImportError::Invalid(
            "No Commons service was found for the configured term".into(),
        ));
    }
    Ok(result)
}

#[derive(Debug, Serialize)]
pub(crate) struct DeclarationSummary {
    pub status: &'static str,
    pub term_start: NaiveDate,
    pub members: usize,
    pub declarations: usize,
    pub rejected: usize,
    pub new_members: Vec<i32>,
}

struct EvidenceSet<'a, S> {
    source: &'a S,
    records: HashMap<i32, Evidence>,
}

impl<S: DeclarationSource> EvidenceSet<'_, S> {
    fn remember(&mut self, evidence: Evidence) -> Result<()> {
        if let Some(id) = evidence.id() {
            if let Some(previous) = self.records.get(&id) {
                if previous.payload != evidence.payload {
                    return Err(ImportError::Invalid(format!(
                        "Conflicting duplicate declaration {id}"
                    )));
                }
                eprintln!("Duplicate declaration {id}; identical source evidence collapsed");
            } else {
                self.records.insert(id, evidence);
            }
        }
        Ok(())
    }

    fn resolve<'a>(
        &'a mut self,
        evidence: &'a Evidence,
        member: i32,
        mut ancestors: HashSet<i32>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Declaration>> + Send + 'a>> {
        Box::pin(async move {
            let draft = self.source.interpret(evidence)?;
            if Some(draft.id) != evidence.id() || draft.member_source_id != member {
                return Err(ImportError::Rejected(
                    "Source identity or member mismatch".into(),
                ));
            }
            if !ancestors.insert(draft.id) {
                return Err(ImportError::Rejected("Cyclic parent relationship".into()));
            }
            let parent = if let Some(parent_id) = draft.required_parent() {
                if !self.records.contains_key(&parent_id) {
                    for record in self.source.parents(member, parent_id).await? {
                        self.remember(record)?;
                    }
                }
                let record = self.records.get(&parent_id).cloned().ok_or_else(|| {
                    ImportError::Rejected("Required parent was not returned".into())
                })?;
                Some(self.resolve(&record, member, ancestors).await?)
            } else {
                None
            };
            draft.accept(parent.as_ref())
        })
    }
}

pub(crate) async fn refresh_declarations(
    term: NaiveDate,
    as_of: NaiveDate,
    source: &(impl DeclarationSource + MemberSource),
    store: &impl ImportStore,
) -> Result<DeclarationSummary> {
    let cohort = store.cohort(term).await?;
    if cohort.is_empty() {
        return Err(ImportError::Invalid(
            "No stored Commons cohort for the configured term; initialize members first".into(),
        ));
    }
    let current = current_members(source, term, as_of).await?;
    let new_members = current
        .keys()
        .filter(|id| !cohort.contains_key(id))
        .copied()
        .collect();
    let mut result = DeclarationSummary {
        status: "succeeded",
        term_start: term,
        members: cohort.len(),
        declarations: 0,
        rejected: 0,
        new_members,
    };
    let mut sources = EvidenceSet {
        source,
        records: HashMap::new(),
    };
    let mut processed = HashSet::new();
    for (source_id, member_id) in cohort {
        let mut accepted = Vec::new();
        let mut offset = 0;
        loop {
            let page = source.declarations(source_id, offset).await?;
            for evidence in &page.entries {
                sources.remember(evidence.clone())?;
            }
            for evidence in page.entries {
                if evidence.id().is_some_and(|id| !processed.insert(id)) {
                    continue;
                }
                match sources.resolve(&evidence, source_id, HashSet::new()).await {
                    Ok(declaration) => accepted.push(Accepted {
                        declaration,
                        fetched_at: evidence.fetched_at,
                    }),
                    Err(ImportError::Rejected(reason)) => {
                        result.rejected += 1;
                        eprintln!(
                            "Rejected declaration {:?} (member {}, {}): {}",
                            evidence.id(),
                            source_id,
                            evidence.context,
                            reason
                        );
                    }
                    Err(error) => return Err(error),
                }
            }
            match page.next {
                Some(next) => offset = next,
                None => break,
            }
        }
        // No write becomes visible until all pages and necessary parents have succeeded.
        store.publish(member_id, &accepted).await?;
        result.declarations += accepted.len();
        eprintln!("Committed declarations for member {source_id}");
    }
    Ok(result)
}
