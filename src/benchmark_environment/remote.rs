use std::path::Path;
use tokio::process::Command;

use super::common::{Monitor, get_app_env_vars, get_db_env_vars};
use super::config::{RemoteConfig, RemoteHostConfig};
use super::{BenchmarkEnvironment, Endpoint, ServerUsage};
use crate::database::DatabaseKind;
use crate::prelude::*;
use crate::wrk::WrkResult;

pub struct RemoteBenchmarkEnvironment {
    config: RemoteConfig,
    inner: tokio::sync::Mutex<Option<RemoteState>>,
}

struct RemoteState {
    app_image: String,
    db_image: Option<String>,
    app_container: String,
    db_container: Option<String>,
    db_kind: Option<DatabaseKind>,
    app_env: Vec<(String, String)>,
    app_args: Vec<String>,
    monitor: Option<Monitor>,
}

impl RemoteBenchmarkEnvironment {
    pub fn new(config: RemoteConfig) -> Self {
        Self {
            config,
            inner: tokio::sync::Mutex::new(None),
        }
    }

    async fn ssh_output(host: &RemoteHostConfig, command: &str) -> Result<String> {
        debug!("SSH to {}: {}", host.ip, command);
        let mut cmd = Command::new("ssh");
        cmd.arg("-i")
            .arg(&host.ssh_key_path)
            .arg("-o")
            .arg("StrictHostKeyChecking=no")
            .arg(format!("{}@{}", host.user, host.ip))
            .arg(command);

        let output = cmd
            .output()
            .await
            .map_err(|e| Error::System(format!("Failed to execute ssh: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("SSH command failed: {}\nStderr: {}", command, stderr);
            return Err(Error::System(format!(
                "SSH command failed: {}\nStderr: {}",
                command, stderr
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn ssh(host: &RemoteHostConfig, command: &str) -> Result<()> {
        Self::ssh_output(host, command).await?;
        Ok(())
    }

    async fn ssh_check(host: &RemoteHostConfig, command: &str) -> Result<bool> {
        let mut cmd = Command::new("ssh");
        cmd.arg("-i")
            .arg(&host.ssh_key_path)
            .arg("-o")
            .arg("StrictHostKeyChecking=no")
            .arg(format!("{}@{}", host.user, host.ip))
            .arg(command);

        let output = cmd
            .output()
            .await
            .map_err(|e| Error::System(format!("Failed to execute ssh: {}", e)))?;
        Ok(output.status.success())
    }

    async fn rsync(host: &RemoteHostConfig, src: &Path, dest: &str) -> Result<()> {
        debug!("Rsync {:?} to {}:{}", src, host.ip, dest);
        let mut cmd = Command::new("rsync");
        cmd.arg("-avz")
            .arg("-e")
            .arg(format!(
                "ssh -i {} -o StrictHostKeyChecking=no",
                host.ssh_key_path
            ))
            .arg(src)
            .arg(format!("{}@{}:{}", host.user, host.ip, dest));

        let output = cmd
            .output()
            .await
            .map_err(|e| Error::System(format!("Failed to execute rsync: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Rsync failed: {:?} -> {}\nStderr: {}", src, dest, stderr);
            return Err(Error::System(format!(
                "Rsync failed: {:?} -> {}\nStderr: {}",
                src, dest, stderr
            )));
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl BenchmarkEnvironment for RemoteBenchmarkEnvironment {
    async fn prepare(
        &mut self,
        framework_path: &Path,
        database: Option<DatabaseKind>,
        app_env: &[(String, String)],
        app_args: &[String],
    ) -> Result<()> {
        debug!("Preparing remote environment...");
        let app_image = "benchmark_app:latest".to_string();
        let app_container = "benchmark_app".to_string();
        let db_image = database.map(|k| format!("benchmark_db:{:?}", k).to_lowercase());
        let db_container = database.map(|k| format!("benchmark_db_{:?}", k).to_lowercase());

        let app_host = self
            .config
            .hosts
            .get("app")
            .ok_or_else(|| Error::System("Missing app host config".to_string()))?;
        let db_host = self.config.hosts.get("db");

        debug!("Cleaning up containers on app host...");
        Self::ssh(app_host, "sudo docker rm -f $(sudo docker ps -aq) || true").await?;

        if let Some(db_host) = db_host {
            debug!("Cleaning up containers on db host...");
            Self::ssh(db_host, "sudo docker rm -f $(sudo docker ps -aq) || true").await?;
        }

        let remote_app_path = format!("~/benchmark_builds/{}", app_image);
        debug!("Creating remote app directory: {}", remote_app_path);
        Self::ssh(app_host, &format!("mkdir -p {}", remote_app_path)).await?;
        let src_path_str = framework_path
            .to_str()
            .ok_or_else(|| Error::System("Invalid framework path".to_string()))?;
        let src_path_with_slash = if src_path_str.ends_with('/') {
            src_path_str.to_string()
        } else {
            format!("{}/", src_path_str)
        };

        debug!("Syncing framework code to app host...");
        Self::rsync(app_host, Path::new(&src_path_with_slash), &remote_app_path).await?;

        debug!("Syncing benchmarks_data to app host...");
        Self::ssh(app_host, "mkdir -p ~/benchmarks_data").await?;
        Self::rsync(app_host, Path::new("benchmarks_data/"), "~/benchmarks_data").await?;

        let wrk_host = self
            .config
            .hosts
            .get("wrk")
            .ok_or_else(|| Error::System("Missing wrk host config".to_string()))?;
        debug!("Syncing scripts to wrk host...");
        Self::rsync(wrk_host, Path::new("scripts/"), "~/scripts").await?;

        debug!("Building app image on remote host: {}", app_image);
        Self::ssh(
            app_host,
            &format!(
                "cd {} && sudo docker build -t {} .",
                remote_app_path, app_image
            ),
        )
        .await?;

        if let (Some(db_host), Some(db_kind), Some(db_image)) = (db_host, database, &db_image) {
            let remote_db_path = format!("~/benchmark_builds/{}", db_image);
            debug!("Creating remote db directory: {}", remote_db_path);
            Self::ssh(db_host, &format!("mkdir -p {}", remote_db_path)).await?;
            debug!("Syncing db code to db host...");
            let db_src_path = {
                let path = db_kind.dir();
                if path.ends_with('/') {
                    path.to_string()
                } else {
                    format!("{}/", path)
                }
            };
            Self::rsync(db_host, Path::new(&db_src_path), &remote_db_path).await?;

            debug!("Building db image on remote host: {}", db_image);
            Self::ssh(
                db_host,
                &format!(
                    "cd {} && sudo docker build -t {} .",
                    remote_db_path, db_image
                ),
            )
            .await?;
        }

        let mut guard = self.inner.lock().await;
        *guard = Some(RemoteState {
            app_image,
            db_image,
            app_container,
            db_container,
            db_kind: database,
            app_env: app_env.to_vec(),
            app_args: app_args.to_vec(),
            monitor: None,
        });

        debug!("Remote environment prepared successfully.");
        Ok(())
    }

    async fn start_db(&mut self) -> Result<Option<Endpoint>> {
        debug!("Starting database...");
        let guard = self.inner.lock().await;
        let state = guard
            .as_ref()
            .ok_or_else(|| Error::EnvironmentNotPrepared)?;
        let db_host = match self.config.hosts.get("db") {
            Some(host) => host,
            None => return Ok(None),
        };

        let (db_container, db_image, db_kind) =
            match (&state.db_container, &state.db_image, state.db_kind) {
                (Some(container), Some(image), Some(kind)) => (container, image, kind),
                _ => return Ok(None),
            };

        let mut cmd_str = format!(
            "sudo docker run --name {} -d -p {port}:{port}",
            db_container,
            port = db_kind.port()
        );
        for (k, v) in get_db_env_vars(db_kind) {
            cmd_str.push_str(&format!(" -e {}={}", k, v));
        }
        cmd_str.push_str(&format!(" {}", db_image));

        Self::ssh(db_host, &cmd_str).await?;

        debug!("Waiting for database to be ready...");
        let start = std::time::Instant::now();
        loop {
            // MySQL images can take longer to initialize on cold start; allow more time.
            if start.elapsed().as_secs() > 90 {
                return Err(Error::System("Database startup timed out".to_string()));
            }

            let ready = match db_kind {
                DatabaseKind::Postgres => {
                    let check_cmd = format!(
                        "sudo docker exec {} pg_isready -h 127.0.0.1 -U benchmark",
                        db_container
                    );
                    Self::ssh_check(db_host, &check_cmd).await?
                }
                DatabaseKind::Mysql => {
                    let check_cmd = format!(
                        "sudo docker exec {} mysqladmin ping -h 127.0.0.1 -u benchmark -pbenchmark --silent",
                        db_container
                    );
                    Self::ssh_check(db_host, &check_cmd).await?
                }
                DatabaseKind::Mariadb => {
                    let check_cmd = format!(
                        "sudo docker exec {} mariadb-admin ping -h 127.0.0.1 -u benchmark -pbenchmark --silent",
                        db_container
                    );
                    Self::ssh_check(db_host, &check_cmd).await?
                }
                DatabaseKind::Mssql => {
                    let check_cmd = format!(
                        "sudo docker exec {} /usr/config/check_ready.sh",
                        db_container
                    );
                    Self::ssh_check(db_host, &check_cmd).await?
                }
                DatabaseKind::Mongodb => {
                    let check_cmd = format!(
                        "sudo docker exec {} mongosh --eval \"db.adminCommand('ping')\"",
                        db_container
                    );
                    Self::ssh_check(db_host, &check_cmd).await?
                }
            };

            if ready {
                break;
            }

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        debug!(
            "Database started at {}:{}",
            db_host.internal_ip,
            db_kind.port()
        );
        Ok(Some(Endpoint {
            address: db_host.internal_ip.clone(),
            port: db_kind.port(),
        }))
    }

    async fn stop_db(&mut self) -> Result<()> {
        debug!("Stopping database...");
        let guard = self.inner.lock().await;
        if let Some(state) = guard.as_ref()
            && let (Some(db_host), Some(db_container)) =
                (self.config.hosts.get("db"), &state.db_container)
        {
            let _ = Self::ssh(db_host, &format!("sudo docker stop {}", db_container)).await;
            let _ = Self::ssh(db_host, &format!("sudo docker rm {}", db_container)).await;
        }
        Ok(())
    }

    async fn start_app(&mut self, db_endpoint: Option<&Endpoint>) -> Result<Endpoint> {
        debug!("Starting application...");
        let mut guard = self.inner.lock().await;
        let state = guard
            .as_mut()
            .ok_or_else(|| Error::EnvironmentNotPrepared)?;
        let app_host = self
            .config
            .hosts
            .get("app")
            .ok_or_else(|| Error::System("Missing app host config".to_string()))?;

        let mut cmd_str = format!(
            "sudo docker run --name {} -d -p 8000:8000 -v ~/benchmarks_data:/app/benchmarks_data --ulimit nofile=1000000:1000000",
            state.app_container
        );
        let mut all_env = Vec::new();
        if let (Some(kind), Some(db_ep)) = (state.db_kind, db_endpoint) {
            all_env.extend(get_app_env_vars(kind, &db_ep.address, db_ep.port));
        }
        all_env.extend(state.app_env.clone());
        for (k, v) in all_env {
            let escaped = v.replace('"', "\\\"").replace('\\', "\\\\");
            cmd_str.push_str(&format!(" -e {}=\"{}\"", k, escaped));
        }
        cmd_str.push_str(&format!(" {}", state.app_image));
        if !state.app_args.is_empty() {
            for arg in &state.app_args {
                cmd_str.push(' ');
                cmd_str.push_str(arg);
            }
        }

        Self::ssh(app_host, &cmd_str).await?;

        let app_host_clone = app_host.clone();
        let container_id = state.app_container.clone();
        state.monitor = Some(Monitor::new(move || {
            let app_host = app_host_clone.clone();
            let container_id = container_id.clone();
            async move {
                let stats_cmd = format!(
                    "sudo docker stats --no-stream --format \"{{{{.MemUsage}}}}\" {}",
                    container_id
                );
                let mut cmd = Command::new("ssh");
                cmd.arg("-i")
                    .arg(&app_host.ssh_key_path)
                    .arg("-o")
                    .arg("StrictHostKeyChecking=no")
                    .arg(format!("{}@{}", app_host.user, app_host.ip))
                    .arg(&stats_cmd);

                if let Ok(output) = cmd.output().await
                    && output.status.success()
                {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let mem_usage_part = stdout.split('/').next().unwrap_or("").trim();
                    return crate::parsers::parse_mem(mem_usage_part);
                }
                None
            }
        }));

        let probe_url = format!("{}:8000", app_host.ip);
        debug!("Waiting for application to be ready at {}...", probe_url);
        crate::http_probe::wait_server_ready(&probe_url, std::time::Duration::from_secs(60))
            .await?;

        debug!("Application started at {}:8000", app_host.internal_ip);
        Ok(Endpoint {
            address: app_host.internal_ip.clone(),
            port: 8000,
        })
    }

    async fn stop_app(&mut self) -> Result<ServerUsage> {
        debug!("Stopping application...");
        let mut guard = self.inner.lock().await;
        if let Some(state) = guard.as_mut() {
            let app_host = self
                .config
                .hosts
                .get("app")
                .ok_or_else(|| Error::System("Missing app host config".to_string()))?;

            let mem_usage = if let Some(monitor) = state.monitor.take() {
                monitor.stop().await
            } else {
                0
            };

            let _ = Self::ssh(
                app_host,
                &format!("sudo docker stop {}", state.app_container),
            )
            .await;
            let _ = Self::ssh(app_host, &format!("sudo docker rm {}", state.app_container)).await;

            debug!("Application stopped. Max memory usage: {} bytes", mem_usage);
            return Ok(ServerUsage {
                memory_usage_bytes: mem_usage,
            });
        }
        Ok(ServerUsage {
            memory_usage_bytes: 0,
        })
    }

    async fn exec_wrk_with_connections(
        &self,
        app_endpoint: &Endpoint,
        script: String,
        connections: u32,
        duration: u64,
    ) -> Result<WrkResult> {
        debug!(
            "Executing wrk benchmark with {} connections...",
            connections
        );
        let wrk_host = self
            .config
            .hosts
            .get("wrk")
            .ok_or_else(|| Error::System("Missing wrk host config".to_string()))?;

        let mut threads = self.config.wrk.threads;
        if threads > connections {
            threads = connections;
        }
        let script_path = Path::new(&script);
        let script_name = script_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&script);
        let script_opt = format!("-s ~/scripts/{}", script_name);

        let url = format!("http://{}:{}/", app_endpoint.address, app_endpoint.port);

        let cmd = format!(
            "ulimit -n 65535; wrk -t{} -c{} -d{}s --latency {} {}",
            threads, connections, duration, script_opt, url
        );

        debug!("Running wrk command: {}", cmd);
        let output = Self::ssh_output(wrk_host, &cmd).await?;

        let wrk_output_vec: Vec<String> = output.lines().map(|s| s.to_string()).collect();
        for line in &wrk_output_vec {
            debug!("{}", line);
        }
        let wrk_result = crate::wrk::parse_wrk_output(&wrk_output_vec)
            .map_err(|e| Error::WrkParseError(e.to_string()))?;

        debug!("Wrk benchmark completed with {} connections.", connections);
        Ok(wrk_result)
    }

    async fn exec_wrk_warmup(
        &self,
        app_endpoint: &Endpoint,
        script: &str,
        duration_secs: u64,
    ) -> Result<WrkResult> {
        debug!("Executing wrk warmup...");
        let wrk_host = self
            .config
            .hosts
            .get("wrk")
            .ok_or_else(|| Error::System("Missing wrk host config".to_string()))?;

        let duration = duration_secs;
        let threads = 2;
        let connections = 8;

        let script_path = std::path::Path::new(script);
        let script_name = script_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(script);
        let script_opt = format!("-s ~/scripts/{}", script_name);

        let url = format!("http://{}:{}/", app_endpoint.address, app_endpoint.port);

        let cmd = format!(
            "ulimit -n 65535; wrk -t{} -c{} -d{}s --latency {} {}",
            threads, connections, duration, script_opt, url
        );

        debug!("Running wrk command: {}", cmd);
        let output = Self::ssh_output(wrk_host, &cmd).await?;

        let wrk_output_vec: Vec<String> = output.lines().map(|s| s.to_string()).collect();
        for line in &wrk_output_vec {
            debug!("{}", line);
        }
        let wrk_result = crate::wrk::parse_wrk_output(&wrk_output_vec)
            .map_err(|e| Error::WrkParseError(e.to_string()))?;

        debug!("Wrk warmup completed.");
        Ok(wrk_result)
    }

    fn wrk_duration(&self) -> u64 {
        self.config.wrk.duration_secs
    }
}
