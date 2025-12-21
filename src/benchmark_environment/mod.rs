pub mod config;
pub mod local;
pub mod remote;
pub mod common;

use crate::{http_probe::ServerInfo, prelude::*, wrk::WrkResult};
use config::{EnvironmentFile, EnvironmentKind};
use std::path::PathBuf;

pub use config::WrkConfig;

pub struct Endpoint {
    pub address: String,
    pub port: u16,
}

pub struct ServerUsage {
    pub memory_usage_bytes: u64,
}

#[async_trait::async_trait]
pub trait BenchmarkEnvironment: Send + Sync {
    async fn prepare(&mut self, framework_path: &std::path::Path) -> Result<()>;

    async fn start_db(&mut self) -> Result<Endpoint>;
    async fn stop_db(&mut self) -> Result<()>;

    async fn start_app(&mut self, db_endpoint: &Endpoint) -> Result<Endpoint>;
    async fn stop_app(&mut self) -> Result<ServerUsage>;

    async fn get_app_info(&self, app_endpoint: &Endpoint) -> Result<ServerInfo>;

    async fn exec_wrk_warmup(&self, app_endpoint: &Endpoint) -> Result<WrkResult>;

    async fn exec_wrk_with_connections(
        &self,
        app_endpoint: &Endpoint,
        script: String,
        connections: u32,
    ) -> Result<WrkResult>;
}

pub fn list_environments() -> Result<Vec<String>> {
    let mut envs = Vec::new();
    let paths = std::fs::read_dir("config/environments")?;
    for path in paths {
        let path = path?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                envs.push(stem.to_string());
            }
        }
    }
    Ok(envs)
}

pub fn load_environment(name: &str) -> Result<Box<dyn BenchmarkEnvironment>> {
    let config = get_environment_config(name)?;

    match config.kind {
        EnvironmentKind::Local(local_config) => {
            Ok(Box::new(local::LocalBenchmarkEnvironment::new(local_config)))
        }
        EnvironmentKind::Remote(remote_config) => {
            Ok(Box::new(remote::RemoteBenchmarkEnvironment::new(remote_config)))
        }
    }
}

pub fn get_environment_config(name: &str) -> Result<EnvironmentFile> {
    let path = PathBuf::from(format!("config/environments/{}.yaml", name));
    if !path.exists() {
        return Err(Error::InvalidEnvironment(name.to_string()));
    }

    let content = std::fs::read_to_string(&path)?;
    let config: EnvironmentFile = serde_yaml::from_str(&content)?;
    Ok(config)
}

/// Runs benchmark tests with adaptive connection counts
/// Tries connections: 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192
/// Stops when RPS doesn't increase from previous run
/// Returns the best WrkResult by RPS
pub async fn run_adaptive_connections(
    env: &dyn BenchmarkEnvironment,
    app_endpoint: &Endpoint,
    script: String,
) -> Result<WrkResult> {
    const CONNECTION_COUNTS: &[u32] = &[
        32, 64, 128, 256, 512, 1024, 1536, 2048, 3072, 4096, 6144, 8192,
    ];
    
    let mut best_result: Option<WrkResult> = None;
    let mut best_rps = 0.0;
    
    for &connections in CONNECTION_COUNTS {
        info!("Running benchmark with {} connections", connections);
        let result = env
            .exec_wrk_with_connections(app_endpoint, script.clone(), connections)
            .await?;
        
        info!(
            "Result with {} connections: {:.2} req/sec",
            connections, result.requests_per_sec
        );
        
        // Check if RPS dropped significantly (more than 1%)
        if result.requests_per_sec < best_rps * 0.99 {
            info!(
                "RPS dropped significantly ({:.2} < {:.2} * 0.99), stopping",
                result.requests_per_sec, best_rps
            );
            break;
        }
        
        if result.requests_per_sec > best_rps {
            best_rps = result.requests_per_sec;
            best_result = Some(result);
        }
    }
    
    best_result.ok_or_else(|| Error::System("No valid benchmark results".to_string()))
}
