use std::net::TcpListener;
use std::path::Path;
use std::time::Duration;

use super::common::{Monitor, get_app_env_vars, get_db_env_vars};
use super::config::LocalConfig;
use super::{BenchmarkEnvironment, Endpoint, ServerUsage, WrkResult};

use crate::database::DatabaseKind;
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
    pub db_image: Option<String>,
    pub app_container: String,
    pub db_container: Option<String>,
    pub app_host_port: u16,
    pub db_kind: Option<DatabaseKind>,
    pub app_env: Vec<(String, String)>,
    pub app_args: Vec<String>,
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
    async fn prepare(
        &mut self,
        framework_path: &Path,
        database: Option<DatabaseKind>,
        app_env: &[(String, String)],
        app_args: &[String],
    ) -> Result<()> {
        let app_image = "benchmark_app:latest".to_string();
        let app_container = "benchmark_app".to_string();
        let db_image = database.map(|k| format!("benchmark_db:{:?}", k).to_lowercase());
        let db_container = database.map(|k| format!("benchmark_db_{:?}", k).to_lowercase());

        // Cleanup potential leftovers
        let _ = crate::docker::exec_rm_container(&app_container).await;
        if let Some(ref c) = db_container {
            let _ = crate::docker::exec_rm_container(c).await;
        }

        // Copy benchmarks_data to framework directory to include it in the build context
        let benchmarks_data_src = Path::new("benchmarks_data");
        let benchmarks_data_dst = framework_path.join("benchmarks_data");
        if benchmarks_data_src.exists() {
            let _ = std::process::Command::new("cp")
                .arg("-r")
                .arg(benchmarks_data_src)
                .arg(&benchmarks_data_dst)
                .status();
        }

        let build_result = crate::docker::exec_build(framework_path, &app_image).await;

        // Cleanup benchmarks_data from framework directory
        if benchmarks_data_dst.exists() {
            let _ = std::process::Command::new("rm")
                .arg("-rf")
                .arg(&benchmarks_data_dst)
                .status();
        }

        // Propagate build error if any
        build_result?;

        if let Some(db_kind) = database {
            let _ = crate::docker::exec_build(Path::new(db_kind.dir()), db_image.as_ref().unwrap())
                .await;
        }

        let port = Self::find_free_port();

        let state = LocalState {
            app_image,
            db_image,
            app_container,
            db_container,
            app_host_port: port,
            db_kind: database,
            app_env: app_env.to_vec(),
            app_args: app_args.to_vec(),
            monitor: None,
        };

        let mut guard = self.inner.lock().await;
        *guard = Some(state);
        Ok(())
    }

    async fn start_db(&mut self) -> Result<Option<Endpoint>> {
        let guard = self.inner.lock().await;
        let state = guard
            .as_ref()
            .ok_or_else(|| Error::EnvironmentNotPrepared)?;

        let (db_container, db_image, db_kind) =
            match (&state.db_container, &state.db_image, state.db_kind) {
                (Some(container), Some(image), Some(kind)) => (container, image, kind),
                _ => return Ok(None),
            };

        docker::exec_run_container(
            db_container,
            db_image,
            ContainerOptions {
                ports: None::<String>,
                cpus: self.config.limits.db.as_ref().and_then(|l| l.cpus),
                memory: self.config.limits.db.as_ref().and_then(|l| l.memory_mb),
                link: None::<String>,
                mount: None::<String>,
                envs: Some(get_db_env_vars(db_kind)),
                args: None,
                ulimit: None::<String>,
            },
        )
        .await?;

        Ok(Some(Endpoint {
            address: DB_LINK_NAME.to_string(),
            port: db_kind.port(),
        }))
    }

    async fn stop_db(&mut self) -> Result<()> {
        let guard = self.inner.lock().await;
        let state = guard
            .as_ref()
            .ok_or_else(|| Error::EnvironmentNotPrepared)?;
        if let Some(db_container) = &state.db_container {
            let _ = docker::exec_stop_container(db_container).await?;
        }
        Ok(())
    }

    async fn start_app(&mut self, db_endpoint: Option<&Endpoint>) -> Result<Endpoint> {
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
                link: state
                    .db_container
                    .as_ref()
                    .map(|c| format!("{}:{}", c, DB_LINK_NAME)),
                mount: None,
                envs: Some({
                    let mut envs = Vec::new();
                    if let (Some(kind), Some(db_ep)) = (state.db_kind, db_endpoint) {
                        envs.extend(get_app_env_vars(kind, db_ep.address.as_str(), db_ep.port));
                    }
                    envs.extend(state.app_env.clone());
                    envs
                }),
                args: Some(state.app_args.clone()),
                ulimit: Some("nofile=1000000:1000000".to_string()),
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

    async fn exec_wrk_warmup(
        &self,
        _app_endpoint: &Endpoint,
        script: &str,
        duration_secs: u64,
    ) -> Result<WrkResult> {
        let guard = self.inner.lock().await;
        let state = guard
            .as_ref()
            .ok_or_else(|| Error::EnvironmentNotPrepared)?;
        let url = format!("http://localhost:{}", state.app_host_port);
        let res = wrk::start_wrk(&url, duration_secs, 2, 8, Some(script)).await?;
        Ok(res)
    }

    fn wrk_duration(&self) -> u64 {
        self.config.wrk.duration_secs
    }
}
