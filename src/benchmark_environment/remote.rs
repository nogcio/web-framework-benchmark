use std::path::Path;
use tokio::process::Command;

use crate::prelude::*;
use crate::http_probe::ServerInfo;
use crate::wrk::WrkResult;
use super::{BenchmarkEnvironment, Endpoint, ServerUsage};
use super::config::{RemoteConfig, RemoteHostConfig};
use super::common::{Monitor, get_db_env_vars, get_app_env_vars};

pub struct RemoteBenchmarkEnvironment {
    config: RemoteConfig,
    inner: tokio::sync::Mutex<Option<RemoteState>>,
}

struct RemoteState {
    app_image: String,
    db_image: String,
    app_container: String,
    db_container: String,
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

        let output = cmd.output().await.map_err(|e| Error::System(format!("Failed to execute ssh: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("SSH command failed: {}\nStderr: {}", command, stderr);
            return Err(Error::System(format!("SSH command failed: {}\nStderr: {}", command, stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn ssh(host: &RemoteHostConfig, command: &str) -> Result<()> {
        Self::ssh_output(host, command).await?;
        Ok(())
    }

    async fn rsync(host: &RemoteHostConfig, src: &Path, dest: &str) -> Result<()> {
        debug!("Rsync {:?} to {}:{}", src, host.ip, dest);
        let mut cmd = Command::new("rsync");
        cmd.arg("-avz")
            .arg("-e")
            .arg(format!("ssh -i {} -o StrictHostKeyChecking=no", host.ssh_key_path))
            .arg(src)
            .arg(format!("{}@{}:{}", host.user, host.ip, dest));

        let output = cmd.output().await.map_err(|e| Error::System(format!("Failed to execute rsync: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Rsync failed: {:?} -> {}\nStderr: {}", src, dest, stderr);
            return Err(Error::System(format!("Rsync failed: {:?} -> {}\nStderr: {}", src, dest, stderr)));
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl BenchmarkEnvironment for RemoteBenchmarkEnvironment {
    async fn prepare(&mut self, framework_path: &Path) -> Result<()> {
        debug!("Preparing remote environment...");
        let app_image = format!("benchmark_app:{}", uuid::Uuid::new_v4());
        let db_image = format!("benchmark_db:{}", uuid::Uuid::new_v4());
        let app_container = format!("app-{}", uuid::Uuid::new_v4());
        let db_container = format!("db-{}", uuid::Uuid::new_v4());

        let app_host = self.config.hosts.get("app").ok_or_else(|| Error::System("Missing app host config".to_string()))?;
        let db_host = self.config.hosts.get("db").ok_or_else(|| Error::System("Missing db host config".to_string()))?;

        // Cleanup existing containers
        debug!("Cleaning up containers on app host...");
        Self::ssh(app_host, "docker rm -f $(docker ps -aq) || true").await?;
        
        debug!("Cleaning up containers on db host...");
        Self::ssh(db_host, "docker rm -f $(docker ps -aq) || true").await?;

        // Prepare App
        // Copy framework code
        let remote_app_path = format!("~/benchmark_builds/{}", app_image);
        debug!("Creating remote app directory: {}", remote_app_path);
        Self::ssh(app_host, &format!("mkdir -p {}", remote_app_path)).await?;
        // We need to copy the contents of framework_path into remote_app_path
        // rsync src/ dest/ puts contents of src into dest
        let src_path_str = framework_path.to_str().ok_or_else(|| Error::System("Invalid framework path".to_string()))?;
        // Ensure trailing slash to copy contents
        let src_path_with_slash = if src_path_str.ends_with('/') {
            src_path_str.to_string()
        } else {
            format!("{}/", src_path_str)
        };
        
        debug!("Syncing framework code to app host...");
        Self::rsync(app_host, Path::new(&src_path_with_slash), &remote_app_path).await?;

        // Sync benchmarks_data
        debug!("Syncing benchmarks_data to app host...");
        Self::ssh(app_host, "mkdir -p ~/benchmarks_data").await?;
        Self::rsync(app_host, Path::new("benchmarks_data/"), "~/benchmarks_data").await?;

        // Build App Image
        debug!("Building app image on remote host: {}", app_image);
        Self::ssh(app_host, &format!("cd {} && docker build -t {} .", remote_app_path, app_image)).await?;


        // Prepare DB
        // Copy db code
        let remote_db_path = format!("~/benchmark_builds/{}", db_image);
        debug!("Creating remote db directory: {}", remote_db_path);
        Self::ssh(db_host, &format!("mkdir -p {}", remote_db_path)).await?;
        debug!("Syncing db code to db host...");
        Self::rsync(db_host, Path::new("benchmarks_db/"), &remote_db_path).await?;

        // Build DB Image
        debug!("Building db image on remote host: {}", db_image);
        Self::ssh(db_host, &format!("cd {} && docker build -t {} .", remote_db_path, db_image)).await?;

        let mut guard = self.inner.lock().await;
        *guard = Some(RemoteState {
            app_image,
            db_image,
            app_container,
            db_container,
            monitor: None,
        });

        debug!("Remote environment prepared successfully.");
        Ok(())
    }

    async fn start_db(&mut self) -> Result<Endpoint> {
        debug!("Starting database...");
        let guard = self.inner.lock().await;
        let state = guard.as_ref().ok_or_else(|| Error::EnvironmentNotPrepared)?;
        let db_host = self.config.hosts.get("db").ok_or_else(|| Error::System("Missing db host config".to_string()))?;

        // Run DB container
        // We assume standard postgres port 5432 for now, or we should make it configurable/discoverable
        // The benchmarks_db/Dockerfile usually exposes 5432
        let mut cmd_str = format!(
            "docker run --name {} -d -p 5432:5432",
            state.db_container
        );
        for (k, v) in get_db_env_vars() {
            cmd_str.push_str(&format!(" -e {}={}", k, v));
        }
        cmd_str.push_str(&format!(" {}", state.db_image));
        
        Self::ssh(db_host, &cmd_str).await?;

        debug!("Database started at {}:5432", db_host.internal_ip);
        Ok(Endpoint {
            address: db_host.internal_ip.clone(),
            port: 5432,
        })
    }

    async fn stop_db(&mut self) -> Result<()> {
        debug!("Stopping database...");
        let guard = self.inner.lock().await;
        if let Some(state) = guard.as_ref() {
            let db_host = self.config.hosts.get("db").ok_or_else(|| Error::System("Missing db host config".to_string()))?;
            let _ = Self::ssh(db_host, &format!("docker stop {}", state.db_container)).await;
            let _ = Self::ssh(db_host, &format!("docker rm {}", state.db_container)).await;
        }
        Ok(())
    }

    async fn start_app(&mut self, db_endpoint: &Endpoint) -> Result<Endpoint> {
        debug!("Starting application...");
        let mut guard = self.inner.lock().await;
        let state = guard.as_mut().ok_or_else(|| Error::EnvironmentNotPrepared)?;
        let app_host = self.config.hosts.get("app").ok_or_else(|| Error::System("Missing app host config".to_string()))?;

        // Run App container
        // We need to pass DB connection info.
        // Usually frameworks expect DATABASE_URL or similar.
        // The local environment passes it via env vars or link.
        // Here we pass the internal IP of the DB.
        
        // We need to know which port the app listens on.
        // In local env, we bind to a random port on host.
        // Here we can bind to 8080 on host (internal IP).
        // But we need to know what port the container exposes.
        // Usually it's 8080 or 3000.
        // We might need to inspect the image or assume a standard port.
        // For now, let's assume the app container listens on 8080 and we map it to 8080 on host.
        // Or we can use --net=host? No, that's linux only and might conflict.
        
        // Let's assume we map 8080:8080.
        // And we pass DATABASE_URL.
        // The format depends on the database. Assuming Postgres.
        
        let mut cmd_str = format!(
            "docker run --name {} -d -p 8000:8000 -v ~/benchmarks_data:/app/benchmarks_data",
            state.app_container
        );
        for (k, v) in get_app_env_vars(&db_endpoint.address, db_endpoint.port) {
            cmd_str.push_str(&format!(" -e {}={}", k, v));
        }
        cmd_str.push_str(&format!(" {}", state.app_image));
        
        Self::ssh(app_host, &cmd_str).await?;

        let app_host_clone = app_host.clone();
        let container_id = state.app_container.clone();
        state.monitor = Some(Monitor::new(move || {
            let app_host = app_host_clone.clone();
            let container_id = container_id.clone();
            async move {
                let stats_cmd = format!(
                    "docker stats --no-stream --format \"{{{{.MemUsage}}}}\" {}",
                    container_id
                );
                let mut cmd = Command::new("ssh");
                cmd.arg("-i")
                    .arg(&app_host.ssh_key_path)
                    .arg("-o")
                    .arg("StrictHostKeyChecking=no")
                    .arg(format!("{}@{}", app_host.user, app_host.ip))
                    .arg(&stats_cmd);

                if let Ok(output) = cmd.output().await {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let mem_usage_part = stdout.split('/').next().unwrap_or("").trim();
                        return crate::parsers::parse_mem(mem_usage_part);
                    }
                }
                None
            }
        }));

        let probe_url = format!("{}:8000", app_host.ip);
        debug!("Waiting for application to be ready at {}...", probe_url);
        crate::http_probe::wait_server_ready(&probe_url, std::time::Duration::from_secs(60)).await?;

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
            let app_host = self.config.hosts.get("app").ok_or_else(|| Error::System("Missing app host config".to_string()))?;
            
            let mem_usage = if let Some(monitor) = state.monitor.take() {
                monitor.stop().await
            } else {
                0
            };
            
            let _ = Self::ssh(app_host, &format!("docker stop {}", state.app_container)).await;
            let _ = Self::ssh(app_host, &format!("docker rm {}", state.app_container)).await;

            debug!("Application stopped. Max memory usage: {} bytes", mem_usage);
            return Ok(ServerUsage {
                memory_usage_bytes: mem_usage,
            });
        }
        Ok(ServerUsage { memory_usage_bytes: 0 })
    }

    async fn get_app_info(&self, _app_endpoint: &Endpoint) -> Result<ServerInfo> {
        debug!("Getting app info...");
        let app_host = self.config.hosts.get("app").ok_or_else(|| Error::System("Missing app host config".to_string()))?;
        let url = format!("{}:8000", app_host.ip);
        crate::http_probe::get_server_version(&url).await
    }

    async fn exec_wrk_with_connections(
        &self,
        app_endpoint: &Endpoint,
        script: String,
        connections: u32,
    ) -> Result<WrkResult> {
        debug!("Executing wrk benchmark with {} connections...", connections);
        let wrk_host = self.config.hosts.get("wrk").ok_or_else(|| Error::System("Missing wrk host config".to_string()))?;

        // Sync scripts
        debug!("Syncing scripts to wrk host...");
        Self::rsync(wrk_host, Path::new("scripts/"), "~/scripts").await?;

        let duration = self.config.wrk.duration_secs;
        let threads = self.config.wrk.threads;
        let script_path = Path::new(&script);
        let script_name = script_path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&script);
        let script_opt = format!("-s ~/scripts/{}", script_name);

        let url = format!("http://{}:{}/", app_endpoint.address, app_endpoint.port);

        // Run wrk directly
        let cmd = format!(
            "wrk -t{} -c{} -d{}s --latency {} {}",
            threads, connections, duration, script_opt, url
        );

        debug!("Running wrk command: {}", cmd);
        let output = Self::ssh_output(wrk_host, &cmd).await?;
        debug!("Wrk output: {}", output);

        // Parse output
        let wrk_output_vec: Vec<String> = output.lines().map(|s| s.to_string()).collect();
        let wrk_result = crate::wrk::parse_wrk_output(&wrk_output_vec).map_err(|e| Error::WrkParseError(e.to_string()))?;

        debug!("Wrk benchmark completed with {} connections.", connections);
        Ok(wrk_result)
    }

    async fn exec_wrk_warmup(&self, app_endpoint: &Endpoint) -> Result<WrkResult> {
        debug!("Executing wrk warmup...");
        let wrk_host = self.config.hosts.get("wrk").ok_or_else(|| Error::System("Missing wrk host config".to_string()))?;

        let duration = 5;
        let threads = 4;
        let connections = 32;
        
        let url = format!("http://{}:{}/", app_endpoint.address, app_endpoint.port);

        // Run wrk directly
        let cmd = format!(
            "wrk -t{} -c{} -d{}s --latency {}",
            threads, connections, duration, url
        );

        debug!("Running wrk command: {}", cmd);
        let output = Self::ssh_output(wrk_host, &cmd).await?;
        debug!("Wrk output: {}", output);

        // Parse output
        let wrk_output_vec: Vec<String> = output.lines().map(|s| s.to_string()).collect();
        let wrk_result = crate::wrk::parse_wrk_output(&wrk_output_vec).map_err(|e| Error::WrkParseError(e.to_string()))?;

        debug!("Wrk warmup completed.");
        Ok(wrk_result)
    }
}
