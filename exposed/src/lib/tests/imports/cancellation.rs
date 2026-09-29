use crate::{
    domain::imports::{ImportError, ports::ImportStore},
    outbound::ExposedDatabase,
};
use sqlx::{
    PgPool,
    postgres::{PgPoolOptions, PgSslMode},
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Notify,
    task::JoinSet,
};

// Abort the listener and all its connections even if an assertion fails.
struct ProxyTask(tokio::task::JoinHandle<()>);
impl Drop for ProxyTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[sqlx::test(migrations = "../db/migrations")]
async fn cancellation_while_acquiring_releases_the_lock_without_closing_the_pool(pool: PgPool) {
    let options = pool.connect_options();
    let upstream = (options.get_host().to_owned(), options.get_port());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let hold = Arc::new(AtomicBool::new(false));
    let paused = Arc::new(Notify::new());
    let resume = Arc::new(Notify::new());
    let _proxy = ProxyTask(tokio::spawn({
        let hold = hold.clone();
        let paused = paused.clone();
        let resume = resume.clone();
        async move {
            let mut connections = JoinSet::new();
            loop {
                let (client, _) = listener.accept().await.unwrap();
                let upstream = upstream.clone();
                let hold = hold.clone();
                let paused = paused.clone();
                let resume = resume.clone();
                connections.spawn(async move {
                    let server = TcpStream::connect(upstream).await.unwrap();
                    let (mut client_read, mut client_write) = client.into_split();
                    let (mut server_read, mut server_write) = server.into_split();
                    let requests = tokio::io::copy(&mut client_read, &mut server_write);
                    let responses = async {
                        let mut buffer = [0; 65536];
                        loop {
                            let size = server_read.read(&mut buffer).await?;
                            if size == 0 {
                                return Ok::<(), std::io::Error>(());
                            }
                            if hold.swap(false, Ordering::SeqCst) {
                                paused.notify_one();
                                resume.notified().await;
                            }
                            client_write.write_all(&buffer[..size]).await?;
                        }
                    };
                    tokio::select! {
                        _ = requests => {},
                        _ = responses => {},
                    }
                });
            }
        }
    }));
    let proxied = PgPoolOptions::new()
        .max_connections(1)
        .test_before_acquire(false)
        .connect_with(
            options
                .as_ref()
                .clone()
                .host("127.0.0.1")
                .port(port)
                .ssl_mode(PgSslMode::Disable),
        )
        .await
        .unwrap();

    // Warm the exact prepared statement so the paused response is the lock result,
    // rather than a prepare response sent before PostgreSQL executes the query.
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT pg_try_advisory_lock(20260925, 1)")
            .fetch_one(&proxied)
            .await
            .unwrap()
    );
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT pg_advisory_unlock(20260925, 1)")
            .fetch_one(&proxied)
            .await
            .unwrap()
    );
    tokio::time::timeout(Duration::from_secs(5), async {
        while proxied.num_idle() == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();

    hold.store(true, Ordering::SeqCst);
    let acquisition = tokio::spawn({
        let store = ExposedDatabase::from(proxied.clone());
        async move { store.acquire().await }
    });
    tokio::time::timeout(Duration::from_secs(5), paused.notified())
        .await
        .unwrap();
    let locked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND classid=20260925 AND objid=1 AND database=(SELECT oid FROM pg_database WHERE datname=current_database()))")
        .fetch_one(&pool).await.unwrap();
    assert!(locked, "PostgreSQL must hold the lock before cancellation");
    acquisition.abort();
    assert!(acquisition.await.unwrap_err().is_cancelled());
    resume.notify_one();

    let other_import = ExposedDatabase::from(pool);
    let lease = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match other_import.acquire().await {
                Ok(lease) => break lease,
                Err(ImportError::Busy) => tokio::time::sleep(Duration::from_millis(10)).await,
                Err(error) => panic!("Unexpected acquisition failure: {error:?}"),
            }
        }
    })
    .await
    .expect("cancellation during acquisition must release the advisory lock");
    assert!(!proxied.is_closed());
    assert_eq!(
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&proxied)
            .await
            .unwrap(),
        1
    );
    drop(lease);
    proxied.close().await;
}
