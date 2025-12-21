use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::select;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use std::future::Future;

#[derive(Debug)]
pub struct Monitor {
    token: CancellationToken,
    handler: JoinHandle<u64>,
}

impl Monitor {
    pub fn new<F, Fut>(fetch_stats: F) -> Self 
    where F: Fn() -> Fut + Send + 'static, Fut: Future<Output = Option<u64>> + Send {
        let token = CancellationToken::new();
        let token_child = token.clone();
        let metrics_handler = tokio::spawn(async move {
            let peak = Arc::new(AtomicU64::new(0));
            let peak_child = peak.clone();
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                select! {
                    _ = token_child.cancelled() => {
                        break;
                    }
                    _ = interval.tick() => {
                        if let Some(mem) = fetch_stats().await {
                            let prev = peak_child.load(Ordering::Relaxed);
                            if mem > prev {
                                peak_child.store(mem, Ordering::Relaxed);
                            }
                        }
                    }
                }
            }
            peak.load(Ordering::Relaxed)
        });
        Monitor {
            token,
            handler: metrics_handler,
        }
    }

    pub async fn stop(self) -> u64 {
        self.token.cancel();
        self.handler.await.unwrap_or(0)
    }
}

pub fn get_db_env_vars() -> Vec<(String, String)> {
    vec![
        ("POSTGRES_DB".to_string(), "benchmark".to_string()),
        ("POSTGRES_USER".to_string(), "benchmark".to_string()),
        ("POSTGRES_PASSWORD".to_string(), "benchmark".to_string()),
    ]
}

pub fn get_app_env_vars(db_host: &str, db_port: u16) -> Vec<(String, String)> {
    let db_url = format!("postgres://benchmark:benchmark@{}:{}/benchmark", db_host, db_port);
    vec![
        ("DATABASE_URL".to_string(), db_url),
        ("DB_HOST".to_string(), db_host.to_string()),
        ("DB_PORT".to_string(), db_port.to_string()),
        ("DB_USER".to_string(), "benchmark".to_string()),
        ("DB_PASSWORD".to_string(), "benchmark".to_string()),
        ("DB_NAME".to_string(), "benchmark".to_string()),
    ]
}
