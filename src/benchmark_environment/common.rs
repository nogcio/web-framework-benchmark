use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::database::DatabaseKind;
use tokio::select;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct Monitor {
    token: CancellationToken,
    handler: JoinHandle<u64>,
}

impl Monitor {
    pub fn new<F, Fut>(fetch_stats: F) -> Self
    where
        F: Fn() -> Fut + Send + 'static,
        Fut: Future<Output = Option<u64>> + Send,
    {
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

pub fn get_db_env_vars(db: DatabaseKind) -> Vec<(String, String)> {
    match db {
        DatabaseKind::Postgres => vec![
            ("POSTGRES_DB".to_string(), "benchmark".to_string()),
            ("POSTGRES_USER".to_string(), "benchmark".to_string()),
            ("POSTGRES_PASSWORD".to_string(), "benchmark".to_string()),
        ],
        DatabaseKind::Mysql => vec![
            ("MYSQL_DATABASE".to_string(), "benchmark".to_string()),
            ("MYSQL_USER".to_string(), "benchmark".to_string()),
            ("MYSQL_PASSWORD".to_string(), "benchmark".to_string()),
            ("MYSQL_ROOT_PASSWORD".to_string(), "benchmark".to_string()),
        ],
        DatabaseKind::Mariadb => vec![
            ("MARIADB_DATABASE".to_string(), "benchmark".to_string()),
            ("MARIADB_USER".to_string(), "benchmark".to_string()),
            ("MARIADB_PASSWORD".to_string(), "benchmark".to_string()),
            ("MARIADB_ROOT_PASSWORD".to_string(), "benchmark".to_string()),
        ],
        DatabaseKind::Mssql => vec![
            ("ACCEPT_EULA".to_string(), "Y".to_string()),
            (
                "MSSQL_SA_PASSWORD".to_string(),
                "Benchmark!12345".to_string(),
            ),
            ("MSSQL_PID".to_string(), "Developer".to_string()),
        ],
        DatabaseKind::Mongodb => vec![
            ("MONGO_INITDB_DATABASE".to_string(), "benchmark".to_string()),
            ("MONGO_INITDB_ROOT_USERNAME".to_string(), "benchmark".to_string()),
            ("MONGO_INITDB_ROOT_PASSWORD".to_string(), "benchmark".to_string()),
        ],
    }
}

pub fn get_app_env_vars(db: DatabaseKind, db_host: &str, db_port: u16) -> Vec<(String, String)> {
    match db {
        DatabaseKind::Postgres => {
            let db_url = format!(
                "postgres://benchmark:benchmark@{}:{}/benchmark",
                db_host, db_port
            );
            vec![
                ("DATABASE_URL".to_string(), db_url),
                ("DB_HOST".to_string(), db_host.to_string()),
                ("DB_PORT".to_string(), db_port.to_string()),
                ("DB_USER".to_string(), "benchmark".to_string()),
                ("DB_PASSWORD".to_string(), "benchmark".to_string()),
                ("DB_NAME".to_string(), "benchmark".to_string()),
                ("DB_KIND".to_string(), "postgres".to_string()),
            ]
        }
        DatabaseKind::Mongodb => {
            let db_url = format!(
                "mongodb://benchmark:benchmark@{}:{}/benchmark?authSource=admin",
                db_host, db_port
            );
            vec![
                ("DATABASE_URL".to_string(), db_url),
                ("DB_HOST".to_string(), db_host.to_string()),
                ("DB_PORT".to_string(), db_port.to_string()),
                ("DB_USER".to_string(), "benchmark".to_string()),
                ("DB_PASSWORD".to_string(), "benchmark".to_string()),
                ("DB_NAME".to_string(), "benchmark".to_string()),
                ("DB_KIND".to_string(), "mongodb".to_string()),
            ]
        }
        DatabaseKind::Mysql => {
            let db_url = format!(
                "mysql://benchmark:benchmark@{}:{}/benchmark",
                db_host, db_port
            );
            vec![
                ("DATABASE_URL".to_string(), db_url),
                ("DB_HOST".to_string(), db_host.to_string()),
                ("DB_PORT".to_string(), db_port.to_string()),
                ("DB_USER".to_string(), "benchmark".to_string()),
                ("DB_PASSWORD".to_string(), "benchmark".to_string()),
                ("DB_NAME".to_string(), "benchmark".to_string()),
                ("DB_KIND".to_string(), "mysql".to_string()),
            ]
        }
        DatabaseKind::Mariadb => {
            let db_url = format!(
                "mysql://benchmark:benchmark@{}:{}/benchmark",
                db_host, db_port
            );
            vec![
                ("DATABASE_URL".to_string(), db_url),
                ("DB_HOST".to_string(), db_host.to_string()),
                ("DB_PORT".to_string(), db_port.to_string()),
                ("DB_USER".to_string(), "benchmark".to_string()),
                ("DB_PASSWORD".to_string(), "benchmark".to_string()),
                ("DB_NAME".to_string(), "benchmark".to_string()),
                ("DB_KIND".to_string(), "mariadb".to_string()),
            ]
        }
        DatabaseKind::Mssql => {
            let db_url = format!(
                "Server=tcp:{},{};User ID=benchmark;Password=Benchmark!12345;Database=benchmark;Encrypt=false;TrustServerCertificate=true;",
                db_host, db_port
            );
            vec![
                ("DATABASE_URL".to_string(), db_url),
                ("DB_HOST".to_string(), db_host.to_string()),
                ("DB_PORT".to_string(), db_port.to_string()),
                ("DB_USER".to_string(), "benchmark".to_string()),
                ("DB_PASSWORD".to_string(), "Benchmark!12345".to_string()),
                ("DB_NAME".to_string(), "benchmark".to_string()),
                ("DB_KIND".to_string(), "mssql".to_string()),
            ]
        }
    }
}
