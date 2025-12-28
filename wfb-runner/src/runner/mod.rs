pub mod build;
pub mod database;
pub mod benchmark;

use crate::exec::Executor;
use crate::docker::DockerManager;
use crate::consts;
use indicatif::{ProgressBar, MultiProgress};
use tokio::time::sleep;
use std::time::Duration;
use wfb_storage::{Benchmark, DatabaseKind};
use async_trait::async_trait;

#[async_trait]
pub trait BenchmarkRunner: Send + Sync {
    async fn prepare(&self, mb: &MultiProgress) -> anyhow::Result<()>;
    async fn build_database_images(&self, db_kinds: Vec<DatabaseKind>, mb: &MultiProgress) -> anyhow::Result<()>;
    async fn deploy_wrkr(&self, mb: &MultiProgress) -> anyhow::Result<()>;
    async fn verify_benchmark(&self, benchmark: &Benchmark, mb: &MultiProgress) -> anyhow::Result<()>;
    async fn run_benchmark(&self, benchmark: &Benchmark, mb: &MultiProgress) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct RunnerConfig {
    pub db_host: String,
    pub db_port: String,
    pub app_host_url: String,
    pub app_public_host_url: String,
    pub is_remote: bool,
}

#[derive(Clone)]
pub struct Runner<E: Executor> {
    executor: E,
    db_executor: E,
    wrkr_executor: E,
    app_docker: DockerManager<E>,
    db_docker: DockerManager<E>,
    wrkr_docker: DockerManager<E>,
    config: RunnerConfig,
}

#[async_trait]
impl<E: Executor + Clone + Send + Sync + 'static> BenchmarkRunner for Runner<E> {
    async fn prepare(&self, mb: &MultiProgress) -> anyhow::Result<()> {
        let pb = mb.add(ProgressBar::new_spinner());
        pb.set_style(
            indicatif::ProgressStyle::default_spinner()
                .template("{spinner:.blue} [{prefix}] {msg}")
                .unwrap(),
        );
        pb.enable_steady_tick(Duration::from_millis(100));
        pb.set_prefix("prepare");
        pb.set_message("Preparing remote environment...");

        
        // Stop and remove all containers on all hosts
        self.wrkr_docker.stop_all_containers(&pb).await;
        self.app_docker.stop_all_containers(&pb).await;
        self.db_docker.stop_all_containers(&pb).await;

        self.executor.rm(consts::REMOTE_APP_PATH).await?;
        self.executor.rm(consts::REMOTE_DB_PATH).await?;
        self.db_executor.rm(consts::REMOTE_DB_PATH).await?;
        self.wrkr_executor.rm(consts::REMOTE_WRKR_PATH).await?;

        self.executor.mkdir(consts::REMOTE_APP_PATH).await?;
        self.executor.mkdir(consts::REMOTE_DB_PATH).await?;
        self.db_executor.mkdir(consts::REMOTE_DB_PATH).await?;
        self.wrkr_executor.mkdir(consts::REMOTE_WRKR_PATH).await?;

        pb.set_style(
            indicatif::ProgressStyle::default_spinner()
                .template("{msg}")
                .unwrap(),
        );
        pb.finish_with_message("Remote environment prepared");
        Ok(())
    }

    async fn build_database_images(&self, db_kinds: Vec<DatabaseKind>, mb: &MultiProgress) -> anyhow::Result<()> {
        self.build_database_images_impl(db_kinds, mb).await
    }

    async fn deploy_wrkr(&self, mb: &MultiProgress) -> anyhow::Result<()> {
        self.deploy_wrkr_impl(mb).await
    }

    async fn verify_benchmark(&self, benchmark: &Benchmark, mb: &MultiProgress) -> anyhow::Result<()> {
        self.verify_benchmark_impl(benchmark, mb).await
    }

    async fn run_benchmark(&self, benchmark: &Benchmark, mb: &MultiProgress) -> anyhow::Result<()> {
        self.run_benchmark_impl(benchmark, mb).await
    }
}

impl<E: Executor + Clone> Runner<E> {
    pub fn new(app_executor: E, db_executor: E, wrkr_executor: E, sudo: bool, config: RunnerConfig) -> Self {
        Self {
            executor: app_executor.clone(),
            db_executor: db_executor.clone(),
            wrkr_executor: wrkr_executor.clone(),
            app_docker: DockerManager::new(app_executor, sudo),
            db_docker: DockerManager::new(db_executor, sudo),
            wrkr_docker: DockerManager::new(wrkr_executor, sudo),
            config,
        }
    }

    async fn wait_for_container_ready(
        &self,
        docker: &DockerManager<E>,
        container_name: &str,
        pb: &ProgressBar,
    ) -> anyhow::Result<()> {
        let max_retries = consts::CONTAINER_HEALTH_RETRIES;
        let mut retries = 0;
    
        loop {
            let format = "{{if .State.Health}}{{.State.Health.Status}}{{else}}none{{end}}";
            let output = docker.inspect(container_name, format).await?;
            let status = output.trim();
    
            if status == "healthy" {
                pb.set_message(format!("Container {} is healthy", container_name));
                return Ok(());
            } else if status == "unhealthy" {
                anyhow::bail!("Container {} is unhealthy", container_name);
            }
    
            pb.set_message(format!(
                "Waiting for {} (Health: {})",
                container_name, status
            ));
    
            if retries >= max_retries {
                anyhow::bail!(
                    "Timeout waiting for container {} to be healthy",
                    container_name
                );
            }
    
            sleep(Duration::from_secs(consts::CONTAINER_HEALTH_INTERVAL_SECS)).await;
            retries += 1;
        }
    }
}
