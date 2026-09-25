use super::date;
use crate::imports::{
    adapters::postgres::PostgresStore,
    core::{
        ImportError, Result,
        coordinator::{Clock, deliver_pending, queue_refresh_notification},
        ports::{ImportStore, NotificationQueue, Notifications},
        refresh::RefreshState,
    },
};
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;

struct Time(DateTime<Utc>);
impl Clock for Time {
    fn now(&self) -> DateTime<Utc> {
        self.0
    }
}
struct Delivery(bool);
impl Notifications for Delivery {
    async fn deliver(&self, _: &str) -> Result<()> {
        if self.0 {
            Ok(())
        } else {
            Err(ImportError::Source("Provider unavailable".into()))
        }
    }
}

#[sqlx::test(migrations = "../db/migrations")]
async fn notifications_use_app_frequency_and_retry_without_changing_import_outcome(pool: PgPool) {
    let store = PostgresStore::new(pool.clone());
    let now = Utc::now() + Duration::seconds(1);
    let mut state = RefreshState::default();
    state.completed(now, now, 0, vec![]);
    let term = date("2024-07-04");
    store.record_refresh(term, &state).await.unwrap();
    queue_refresh_notification(term, &store, &Time(now), None)
        .await
        .unwrap();
    assert!(store.pending(now).await.unwrap().is_none());
    queue_refresh_notification(term, &store, &Time(now), Some(3600))
        .await
        .unwrap();
    queue_refresh_notification(term, &store, &Time(now), Some(3600))
        .await
        .unwrap();
    deliver_pending(&store, &Delivery(false), &Time(now))
        .await
        .unwrap();
    assert!(store.pending(now).await.unwrap().is_none());
    assert_eq!(
        store.refresh_state(term).await.unwrap().last_completed,
        Some(now)
    );
    deliver_pending(&store, &Delivery(true), &Time(now + Duration::minutes(1)))
        .await
        .unwrap();
    let delivered: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM exposed.import_notifications WHERE delivered_at IS NOT NULL",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(delivered, 1);
    assert!(
        store
            .pending(now + Duration::hours(1))
            .await
            .unwrap()
            .is_none()
    );
}
