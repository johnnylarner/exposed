use super::refresh::{Changes, RefreshState};
use super::{
    Result,
    declarations::{Accepted, Draft, Evidence},
    members::{Member, MemberHistory, MemberProfile},
};
use chrono::{DateTime, NaiveDate, Utc};
use std::{collections::BTreeMap, future::Future};
use uuid::Uuid;

pub(crate) struct Page<T> {
    pub entries: Vec<T>,
    pub next: Option<usize>,
}

pub(crate) trait MemberSource: Send + Sync {
    fn profiles(
        &self,
        term: NaiveDate,
        as_of: NaiveDate,
        current: bool,
        offset: usize,
    ) -> impl Future<Output = Result<Page<MemberProfile>>> + Send;
    fn histories(&self, ids: &[i32]) -> impl Future<Output = Result<Vec<MemberHistory>>> + Send;
}

pub(crate) trait DeclarationSource: Send + Sync {
    fn declarations(
        &self,
        member: i32,
        offset: usize,
    ) -> impl Future<Output = Result<Page<Evidence>>> + Send;
    fn parents(
        &self,
        member: i32,
        parent: i32,
    ) -> impl Future<Output = Result<Vec<Evidence>>> + Send;
    fn interpret(&self, evidence: &Evidence) -> Result<Draft>;
}

/// Each operation publishes atomically; a failure never changes a partial batch/MP.
/// IDs survive upserts; omitted records remain; dropping a lease releases the run lock.
pub(crate) trait ImportStore: Send + Sync {
    type Lease: Send;
    fn acquire(&self) -> impl Future<Output = Result<Self::Lease>> + Send;
    fn member_batch(
        &self,
        term: NaiveDate,
        members: &[Member],
    ) -> impl Future<Output = Result<Changes>> + Send;
    fn cohort(&self, term: NaiveDate) -> impl Future<Output = Result<BTreeMap<i32, Uuid>>> + Send;
    fn publish(
        &self,
        member: Uuid,
        declarations: &[Accepted],
    ) -> impl Future<Output = Result<()>> + Send;
    fn refresh_state(&self, term: NaiveDate) -> impl Future<Output = Result<RefreshState>> + Send;
    fn record_refresh(
        &self,
        term: NaiveDate,
        state: &RefreshState,
    ) -> impl Future<Output = Result<()>> + Send;
}

pub(crate) trait Notifications: Send + Sync {
    fn deliver(&self, message: &str) -> impl Future<Output = Result<()>> + Send;
}

/// Durable delivery queue, independent of import transactions and refresh outcomes.
pub(crate) trait NotificationQueue: Send + Sync {
    fn enqueue(&self, key: &str, message: &str) -> impl Future<Output = Result<()>> + Send;
    fn pending(
        &self,
        now: DateTime<Utc>,
    ) -> impl Future<Output = Result<Option<(Uuid, String, i32)>>> + Send;
    fn delivered(&self, id: Uuid) -> impl Future<Output = Result<()>> + Send;
    fn retry_notification(
        &self,
        id: Uuid,
        next: DateTime<Utc>,
    ) -> impl Future<Output = Result<()>> + Send;
}
