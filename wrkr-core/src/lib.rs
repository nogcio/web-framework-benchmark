use std::time::Duration;

mod error;
mod lua_env;
mod response;
mod runner;
mod stats;

pub use error::*;
pub use stats::StatsSnapshot;

#[derive(Debug, Clone)]
pub struct WrkConfig {
    pub script_content: String,
    pub host_url: String,
}

#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub duration: Duration,
    pub connections: u64,       // Target connections
    pub start_connections: u64, // Start connections
    pub ramp_up: Option<Duration>,
    pub step_connections: Option<Vec<u64>>,
    pub step_duration: Option<Duration>,
    pub timeout: Option<Duration>,
    pub wrk: WrkConfig,
}

pub async fn run_benchmark<F>(
    config: BenchmarkConfig,
    on_progress: Option<F>,
) -> error::Result<StatsSnapshot>
where
    F: FnMut(StatsSnapshot) + Send + 'static,
{
    runner::run_benchmark(config, on_progress).await
}

pub async fn run_once(config: WrkConfig) -> error::Result<StatsSnapshot> {
    runner::run_once(config).await
}
