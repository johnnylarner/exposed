//! Composition and lifecycle for source workers, operator API, and timers.
use anyhow::Context;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgPoolOptions;
use std::{sync::Arc, time::Duration};

use super::{
    adapters::{
        parliament::{CliTransport, Parliament},
        postgres::PostgresStore,
        whatsapp::WhatsApp,
    },
    admin::{self, AdminState},
    config::ImportConfig,
    core::{
        ImportError,
        coordinator::{Clock, ImportKind, deliver_pending, queue_refresh_notification, run_import},
        ports::ImportStore,
    },
};

pub(crate) struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Run the import operator listener and the in-process daily refresh task.
///
/// # Errors
/// Returns a configuration, database connection, or listener error at startup.
/// Migrations remain an explicit operator action.
pub async fn serve(config: &ImportConfig, connection_string: &str) -> anyhow::Result<()> {
    let token = config
        .admin_token_env
        .as_ref()
        .map(|name| std::env::var(name))
        .transpose()
        .context("Operator token environment variable is missing")?;
    anyhow::ensure!(
        token.as_ref().is_none_or(|t| !t.is_empty()),
        "Operator token is empty"
    );
    anyhow::ensure!(
        config.admin_bind.ip().is_loopback() || token.is_some(),
        "A non-loopback operator listener requires an access token"
    );
    anyhow::ensure!(
        config.notification_interval_seconds.is_none_or(|n| n > 0),
        "Notification interval must be positive"
    );
    anyhow::ensure!(
        config.notification_interval_seconds.is_none() || config.whatsapp.is_some(),
        "A notification interval requires a WhatsApp adapter"
    );
    let whatsapp = config.whatsapp.clone().map(WhatsApp::new).transpose()?;
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .acquire_timeout(Duration::from_secs(15))
        .connect(connection_string)
        .await?;
    let store = PostgresStore::new(pool);
    store.refresh_state(config.term_start).await?;
    let config = Arc::new(config.clone());
    let state = AdminState::new(store.clone(), config.clone(), token);
    let listener = tokio::net::TcpListener::bind(config.admin_bind).await?;
    let server = async {
        axum::serve(listener, admin::router(state.clone()))
            .await
            .map_err(anyhow::Error::from)
    };
    let refresh = async {
        let mut timer = tokio::time::interval(Duration::from_secs(60));
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            timer.tick().await;
            state.expire_sessions().await;
            if !config.automatic_refresh {
                continue;
            }
            let result = async {
                if !store
                    .refresh_state(config.term_start)
                    .await?
                    .due(Utc::now())
                {
                    return Ok(());
                }
                let source = Parliament(CliTransport::new(
                    config.python.clone(),
                    config.source_directory.clone(),
                ));
                let as_of = Utc::now()
                    .with_timezone(&chrono_tz::Europe::London)
                    .date_naive();
                run_import(
                    ImportKind::Declarations,
                    config.term_start,
                    as_of,
                    &source,
                    &store,
                    &SystemClock,
                    config.notification_interval_seconds,
                    true,
                )
                .await?;
                Ok::<_, ImportError>(())
            }
            .await;
            if let Err(error) = result {
                if !matches!(error, ImportError::Busy) {
                    eprintln!("Scheduled declaration refresh: {error:?}");
                }
            }
        }
        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    };
    let notifications = async {
        let mut timer = tokio::time::interval(Duration::from_secs(30));
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            timer.tick().await;
            if let Some(adapter) = &whatsapp {
                if let Err(error) = queue_refresh_notification(
                    config.term_start,
                    &store,
                    &SystemClock,
                    config.notification_interval_seconds,
                )
                .await
                {
                    eprintln!("Notification policy: {error:?}");
                }
                if let Err(error) = deliver_pending(&store, adapter, &SystemClock).await {
                    eprintln!("Notification queue: {error:?}");
                }
            }
        }
        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    };
    tokio::try_join!(server, refresh, notifications)?;
    Ok(())
}
