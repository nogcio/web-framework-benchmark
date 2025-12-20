use std::net::TcpListener;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::select;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::{BenchmarkEnvironment, Endpoint, ServerUsage, WrkConfig, WrkResult};
use serde::Deserialize;

use crate::docker::{self, ContainerOptions};
use crate::prelude::*;
use crate::wrk;

#[derive(Debug, Deserialize)]
pub struct LocalConfig {
    pub wrk: WrkConfig,
    pub limits: LimitsConfig,
}

#[derive(Debug, Deserialize)]
pub struct LimitsConfig {
    pub db: Option<ResourceLimitSpec>,
    pub app: Option<ResourceLimitSpec>,
}

#[derive(Debug, Deserialize)]
pub struct ResourceLimitSpec {
    pub cpus: Option<u32>,
    pub memory_mb: Option<u32>,
}

const DB_LINK_NAME: &str = "db";

#[derive(Debug)]
pub(crate) struct Monitor {
    token: CancellationToken,
    handler: JoinHandle<u64>,
}

#[derive(Debug)]
pub struct LocalBenchmarkEnvironment {
    inner: tokio::sync::Mutex<Option<LocalState>>,
    config: LocalConfig,
}

#[derive(Debug)]
pub struct LocalState {
    pub app_image: String,
    pub db_image: String,
    pub app_container: String,
    pub db_container: String,
    pub app_host_port: u16,
    pub monitor: Option<Monitor>,
}

impl LocalBenchmarkEnvironment {
    pub fn new(config: LocalConfig) -> Self {
        LocalBenchmarkEnvironment { inner: tokio::sync::Mutex::new(None), config }
    }

    fn find_free_port() -> u16 {
        TcpListener::bind("127.0.0.1:0")
            .expect("Failed to bind to address")
            .local_addr()
            .unwrap()
            .port()
    }
}

#[async_trait::async_trait]
impl BenchmarkEnvironment for LocalBenchmarkEnvironment {
    async fn prepare(&mut self, framework_path: &Path) -> Result<()> {
        // generate unique image tags and container names
        let app_image = format!("benchmark_app:{}", uuid::Uuid::new_v4());
        let db_image = format!("benchmark_db:{}", uuid::Uuid::new_v4());
        let app_container = format!("app-{}", uuid::Uuid::new_v4());
        let db_container = format!("db-{}", uuid::Uuid::new_v4());

        // build images (best-effort)
        let _ = crate::docker::exec_build(framework_path, &app_image).await;
        let _ = crate::docker::exec_build(Path::new("benchmarks_db"), &db_image).await;

        let port = Self::find_free_port();

        let state = LocalState {
            app_image,
            db_image,
            app_container,
            db_container,
            app_host_port: port,
            monitor: None
        };
        
        let mut guard = self.inner.lock().await;
        *guard = Some(state);
        Ok(())
    }

    async fn start_db(&mut self) -> Result<Endpoint> {
        let guard = self.inner.lock().await;
        let state = guard.as_ref().ok_or_else(|| Error::EnvironmentNotPrepared)?;
        
        docker::exec_run_container(
            &state.db_container,
            &state.db_image,
            ContainerOptions {
                ports: None::<String>,
                cpus: self.config.limits.db.as_ref().and_then(|l| l.cpus),
                memory: self.config.limits.db.as_ref().and_then(|l| l.memory_mb),
                link: None::<String>,
                mount: None::<String>,
                envs: Some(vec![
                    ("POSTGRES_DB".to_string(), "benchmark".to_string()),
                    ("POSTGRES_USER".to_string(), "benchmark".to_string()),
                    ("POSTGRES_PASSWORD".to_string(), "benchmark".to_string()),
                ]),
            }
        )
        .await?;

        Ok(Endpoint { address: DB_LINK_NAME.to_string(), port: 5432 })
    }

    async fn stop_db(&mut self) -> Result<()> {
        let guard = self.inner.lock().await;
        let state = guard.as_ref().ok_or_else(|| Error::EnvironmentNotPrepared)?;
        let _ = docker::exec_stop_container(&state.db_container).await?;
        Ok(())
    }

    async fn start_app(&mut self, db_endpoint: &Endpoint) -> Result<Endpoint> {
        let mut guard = self.inner.lock().await;
        let state = guard.as_mut().ok_or_else(|| Error::EnvironmentNotPrepared)?;
        
        docker::exec_run_container(
            &state.app_container,
            &state.app_image,
            ContainerOptions {
                ports: Some(format!("{}:8000", state.app_host_port)),
                cpus: self.config.limits.app.as_ref().and_then(|l| l.cpus),
                memory: self.config.limits.app.as_ref().and_then(|l| l.memory_mb),
                link: Some(format!("{}:{}", state.db_container, DB_LINK_NAME)),
                mount: Some("./benchmarks_data:/app/benchmarks_data".to_string()),
                envs:
            Some(vec![
                ("DB_HOST".to_string(), db_endpoint.address.clone()),
                ("DB_PORT".to_string(), db_endpoint.port.to_string()),
                ("DB_USER".to_string(), "benchmark".to_string()),
                ("DB_PASSWORD".to_string(), "benchmark".to_string()),
                ("DB_NAME".to_string(), "benchmark".to_string()),
            ])
            }
        )
        .await?;

        state.monitor = Some(Monitor::new(state.app_container.clone()));

        crate::http_probe::wait_server_ready(&format!("{}:{}", "localhost", state.app_host_port), Duration::from_secs(60)).await?;

        Ok(Endpoint { address: "localhost".to_string(), port: state.app_host_port })
    }

    async fn stop_app(&mut self) -> Result<ServerUsage> {
        let mut guard = self.inner.lock().await;
        let state = guard.as_mut().ok_or_else(|| Error::EnvironmentNotPrepared)?;

        let mem = if let Some(monitor) = state.monitor.take() {
            monitor.stop().await
        } else {
            0
        };
        let _ = crate::docker::exec_stop_container(&state.app_container).await?;

        Ok(ServerUsage { memory_usage_bytes: mem })
    }

    async fn get_app_info(&self, _app_endpoint: &Endpoint) -> Result<crate::http_probe::ServerInfo> {
        let guard = self.inner.lock().await;
        let state = guard.as_ref().ok_or_else(|| Error::EnvironmentNotPrepared)?;
        let target = format!("{}:{}", "localhost", state.app_host_port);
        crate::http_probe::get_server_version(&target).await
    }

    async fn exec_wrk(&self, _app_endpoint: &Endpoint, script: Option<String>) -> Result<WrkResult> {
        let guard = self.inner.lock().await;
        let state = guard.as_ref().ok_or_else(|| Error::EnvironmentNotPrepared)?;
        let url = format!("http://localhost:{}", state.app_host_port);
        let res = wrk::start_wrk(
            &url,
            self.config.wrk.duration_secs,
            self.config.wrk.threads,
            self.config.wrk.connections,
            script.as_deref(),
        )
        .await?;
        Ok(res)
    }
}

impl LocalConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: LocalConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}

impl Monitor {
    fn new(app_container: String) -> Self {
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
                        if let Ok(info) = crate::docker::exec_stats(&app_container).await {
                            let mem = info.memory_usage;
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
        Monitor { token, handler: metrics_handler }
    }

    async fn stop(self) -> u64 {
        self.token.cancel();
        self.handler.await.unwrap_or(0)
    }
}
