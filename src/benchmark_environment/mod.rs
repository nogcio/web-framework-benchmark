pub mod local;

use crate::{http_probe::ServerInfo, prelude::*, wrk::WrkResult};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct WrkConfig {
    pub duration_secs: u64,
    pub threads: u32,
    pub connections: u32,
}

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

    async fn exec_wrk(&self, app_endpoint: &Endpoint, script: Option<String>) -> Result<WrkResult>;
}