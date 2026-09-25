use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{
    Result,
    ports::{DeclarationSource, ImportStore, MemberSource, NotificationQueue, Notifications},
    refresh::{refresh_declarations, refresh_members},
};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ImportKind {
    Members,
    Declarations,
    Initialize,
}

pub(crate) trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

pub(crate) async fn run_import(
    kind: ImportKind,
    term: NaiveDate,
    as_of: NaiveDate,
    source: &(impl MemberSource + DeclarationSource),
    store: &(impl ImportStore + NotificationQueue),
    clock: &impl Clock,
    notification_interval: Option<i64>,
    scheduled: bool,
) -> Result<Value> {
    let _lease = store.acquire().await?;
    let mut state = store.refresh_state(term).await?;
    let started = clock.now();
    if scheduled && !state.due(started) {
        return Ok(json!({"status":"not-due"}));
    }
    let members = match kind {
        ImportKind::Members | ImportKind::Initialize => {
            Some(refresh_members(term, as_of, source, store).await?)
        }
        ImportKind::Declarations => None,
    };
    if matches!(kind, ImportKind::Members) {
        return Ok(json!(members));
    }
    state.last_attempt = Some(started);
    state.next_attempt = Some(started); // Restart after an interrupted run is immediately due.
    store.record_refresh(term, &state).await?;
    let result = refresh_declarations(term, as_of, source, store).await;
    let ended = clock.now();
    match &result {
        Ok(summary) => state.completed(
            started,
            ended,
            summary.rejected,
            summary.new_members.clone(),
        ),
        Err(_) => state.failed(started, ended),
    }
    store.record_refresh(term, &state).await?;
    if let Err(error) = queue_refresh_notification(term, store, clock, notification_interval).await
    {
        eprintln!("Could not queue import notification: {error}");
    }
    let declarations = result?;
    Ok(if matches!(kind, ImportKind::Initialize) {
        json!({"status":"succeeded", "members":members, "declarations":declarations})
    } else {
        json!(declarations)
    })
}

pub(crate) async fn deliver_pending(
    queue: &impl NotificationQueue,
    notifications: &impl Notifications,
    clock: &impl Clock,
) -> Result<()> {
    if let Some((id, message, attempts)) = queue.pending(clock.now()).await? {
        match notifications.deliver(&message).await {
            Ok(()) => queue.delivered(id).await?,
            Err(error) => {
                eprintln!("Notification delivery failed: {error}");
                let seconds = 60_i64
                    .saturating_mul(2_i64.saturating_pow(attempts.clamp(0, 6) as u32))
                    .min(3600);
                queue
                    .retry_notification(id, clock.now() + Duration::seconds(seconds))
                    .await?;
            }
        }
    }
    Ok(())
}

// Retrying this operation is safe after queue storage failures: the interval key is unique.
pub(crate) async fn queue_refresh_notification(
    term: NaiveDate,
    store: &(impl ImportStore + NotificationQueue),
    clock: &impl Clock,
    interval: Option<i64>,
) -> Result<()> {
    let Some(interval) = interval.filter(|seconds| *seconds > 0) else {
        return Ok(());
    };
    let state = store.refresh_state(term).await?;
    let Some(outcome) = &state.outcome else {
        return Ok(());
    };
    let bucket = clock.now().timestamp().div_euclid(interval);
    let key = format!("refresh:{term}:{bucket}");
    let message = format!(
        "Exposed refresh: {outcome}. Last complete source check: {}. Rejected declarations: {}. New MPs awaiting manual refresh: {:?}.",
        state
            .last_completed
            .map_or_else(|| "never".into(), |d| d.to_rfc3339()),
        state.rejected,
        state.new_members
    );
    store.enqueue(&key, &message).await
}
