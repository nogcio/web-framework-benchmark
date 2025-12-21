use std::net::TcpListener;
use std::path::Path;
use std::time::Duration;

use super::common::{Monitor, get_app_env_vars, get_db_env_vars};
use super::config::LocalConfig;
use super::{BenchmarkEnvironment, Endpoint, ServerUsage, WrkResult};

use crate::docker::{self, ContainerOptions};
use crate::prelude::*;
use crate::wrk;

const DB_LINK_NAME: &str = "db";

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
        LocalBenchmarkEnvironment {
            inner: tokio::sync::Mutex::new(None),
            config,
        }
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
        let app_image = format!("benchmark_app:{}", uuid::Uuid::new_v4());
        let db_image = format!("benchmark_db:{}", uuid::Uuid::new_v4());
        let app_container = format!("app-{}", uuid::Uuid::new_v4());
        let db_container = format!("db-{}", uuid::Uuid::new_v4());

        let _ = crate::docker::exec_build(framework_path, &app_image).await;
        let _ = crate::docker::exec_build(Path::new("benchmarks_db"), &db_image).await;

        let port = Self::find_free_port();

        let state = LocalState {
            app_image,
            db_image,
            app_container,
            db_container,
            app_host_port: port,
            monitor: None,
        };

        let mut guard = self.inner.lock().await;
        *guard = Some(state);
        Ok(())
    }

    async fn start_db(&mut self) -> Result<Endpoint> {
        let guard = self.inner.lock().await;
        let state = guard
            .as_ref()
            .ok_or_else(|| Error::EnvironmentNotPrepared)?;

        docker::exec_run_container(
            &state.db_container,
            &state.db_image,
            ContainerOptions {
                ports: None::<String>,
                cpus: self.config.limits.db.as_ref().and_then(|l| l.cpus),
                memory: self.config.limits.db.as_ref().and_then(|l| l.memory_mb),
                link: None::<String>,
                mount: None::<String>,
                envs: Some(get_db_env_vars()),
            },
        )
        .await?;

        Ok(Endpoint {
            address: DB_LINK_NAME.to_string(),
            port: 5432,
        })
    }

    async fn stop_db(&mut self) -> Result<()> {
        let guard = self.inner.lock().await;
        let state = guard
            .as_ref()
            .ok_or_else(|| Error::EnvironmentNotPrepared)?;
        let _ = docker::exec_stop_container(&state.db_container).await?;
        Ok(())
    }

    async fn start_app(&mut self, db_endpoint: &Endpoint) -> Result<Endpoint> {
        let mut guard = self.inner.lock().await;
        let state = guard
            .as_mut()
            .ok_or_else(|| Error::EnvironmentNotPrepared)?;

        docker::exec_run_container(
            &state.app_container,
            &state.app_image,
            ContainerOptions {
                ports: Some(format!("{}:8000", state.app_host_port)),
                cpus: self.config.limits.app.as_ref().and_then(|l| l.cpus),
                memory: self.config.limits.app.as_ref().and_then(|l| l.memory_mb),
                link: Some(format!("{}:{}", state.db_container, DB_LINK_NAME)),
                mount: Some("./benchmarks_data:/app/benchmarks_data".to_string()),
                envs: Some(get_app_env_vars(
                    db_endpoint.address.as_str(),
                    db_endpoint.port,
                )),
            },
        )
        .await?;

        let container_id = state.app_container.clone();
        state.monitor = Some(Monitor::new(move || {
            let container_id = container_id.clone();
            async move {
                crate::docker::exec_stats(&container_id)
                    .await
                    .ok()
                    .map(|s| s.memory_usage)
            }
        }));

        crate::http_probe::wait_server_ready(
            &format!("{}:{}", "localhost", state.app_host_port),
            Duration::from_secs(60),
        )
        .await?;

        Ok(Endpoint {
            address: "localhost".to_string(),
            port: state.app_host_port,
        })
    }

    async fn stop_app(&mut self) -> Result<ServerUsage> {
        let mut guard = self.inner.lock().await;
        let state = guard
            .as_mut()
            .ok_or_else(|| Error::EnvironmentNotPrepared)?;

        let mem = if let Some(monitor) = state.monitor.take() {
            monitor.stop().await
        } else {
            0
        };
        let _ = crate::docker::exec_stop_container(&state.app_container).await?;

        Ok(ServerUsage {
            memory_usage_bytes: mem,
        })
    }

    async fn get_app_info(
        &self,
        _app_endpoint: &Endpoint,
    ) -> Result<crate::http_probe::ServerInfo> {
        let guard = self.inner.lock().await;
        let state = guard
            .as_ref()
            .ok_or_else(|| Error::EnvironmentNotPrepared)?;
        let target = format!("{}:{}", "localhost", state.app_host_port);
        crate::http_probe::get_server_version(&target).await
    }

    async fn exec_wrk_with_connections(
        &self,
        _app_endpoint: &Endpoint,
        script: String,
        connections: u32,
        duration: u64,
    ) -> Result<WrkResult> {
        let guard = self.inner.lock().await;
        let state = guard
            .as_ref()
            .ok_or_else(|| Error::EnvironmentNotPrepared)?;
        let url = format!("http://localhost:{}", state.app_host_port);
        let res = wrk::start_wrk(
            &url,
            duration,
            self.config.wrk.threads,
            connections,
            Some(&script),
        )
        .await?;
        Ok(res)
    }

    async fn exec_wrk_warmup(&self, _app_endpoint: &Endpoint, use_db: bool) -> Result<WrkResult> {
        let guard = self.inner.lock().await;
        let state = guard
            .as_ref()
            .ok_or_else(|| Error::EnvironmentNotPrepared)?;
        let url = if use_db {
            format!("http://localhost:{}/db/read/one?id=1", state.app_host_port)
        } else {
            format!("http://localhost:{}/", state.app_host_port)
        };
        let res = wrk::start_wrk(&url, 5, 2, 8, None).await?;
        Ok(res)
    }

    fn wrk_duration(&self) -> u64 {
        self.config.wrk.duration_secs
    }
}
