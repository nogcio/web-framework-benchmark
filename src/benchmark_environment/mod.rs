pub mod common;
pub mod config;
pub mod local;
pub mod remote;

use crate::{http_probe::ServerInfo, prelude::*, wrk::WrkResult};
use config::{EnvironmentFile, EnvironmentKind};
use std::path::PathBuf;

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

    async fn exec_wrk_warmup(&self, app_endpoint: &Endpoint, use_db: bool) -> Result<WrkResult>;

    fn wrk_duration(&self) -> u64;

    async fn exec_wrk_with_connections(
        &self,
        app_endpoint: &Endpoint,
        script: String,
        connections: u32,
        duration: u64,
    ) -> Result<WrkResult>;
}

pub fn list_environments() -> Result<Vec<String>> {
    let mut envs = Vec::new();
    let paths = std::fs::read_dir("config/environments")?;
    for path in paths {
        let path = path?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("yaml")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            envs.push(stem.to_string());
        }
    }
    Ok(envs)
}

pub fn load_environment(name: &str) -> Result<Box<dyn BenchmarkEnvironment>> {
    let config = get_environment_config(name)?;

    match config.kind {
        EnvironmentKind::Local(local_config) => Ok(Box::new(
            local::LocalBenchmarkEnvironment::new(local_config),
        )),
        EnvironmentKind::Remote(remote_config) => Ok(Box::new(
            remote::RemoteBenchmarkEnvironment::new(remote_config),
        )),
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

pub async fn run_adaptive_connections(
    env: &dyn BenchmarkEnvironment,
    app_endpoint: &Endpoint,
    script: String,
) -> Result<WrkResult> {
    const CONNECTION_COUNTS: &[u32] = &[
        16, 32, 48, 64, 96, 128, 256, 512, 1024, 1536, 2048, 3072, 4096, 6144, 8192,
    ];
    const SCAN_DURATION: u64 = 5;
    const RPS_DROP_THRESHOLD: f64 = 0.03;
    const FULL_BENCHMARK_RANGE: f64 = 0.25;
    const MAX_ALLOWED_ERRORS: i64 = 0;

    let full_duration = env.wrk_duration();

    info!(
        "Starting scan phase with duration {}s, stopping at 3% RPS drop",
        SCAN_DURATION
    );

    struct ScanResult {
        connections: u32,
        rps: f64,
    }

    let mut scan_results: Vec<ScanResult> = Vec::new();
    let mut max_scan_rps = 0.0;

    for &connections in CONNECTION_COUNTS {
        info!("Scanning with {} connections", connections);
        let result = env
            .exec_wrk_with_connections(app_endpoint, script.clone(), connections, SCAN_DURATION)
            .await?;

        let latency_avg_ms = result.latency_avg.as_secs_f64() * 1000.0;

        info!(
            "Scan result: {} connections -> {:.2} req/sec, {:.2}ms avg latency, {} errors",
            connections, result.requests_per_sec, latency_avg_ms, result.errors
        );

        if result.errors > MAX_ALLOWED_ERRORS {
            info!(
                "Errors detected at {} connections, stopping scan",
                connections
            );
            break;
        }

        scan_results.push(ScanResult {
            connections,
            rps: result.requests_per_sec,
        });

        if result.requests_per_sec > max_scan_rps {
            max_scan_rps = result.requests_per_sec;
        } else if max_scan_rps > 0.0
            && result.requests_per_sec < max_scan_rps * (1.0 - RPS_DROP_THRESHOLD)
        {
            info!(
                "RPS dropped by >3% ({:.2} < {:.2}), stopping scan",
                result.requests_per_sec, max_scan_rps
            );
            break;
        }
    }

    if scan_results.is_empty() {
        return Err(Error::System("No benchmark results from scan".to_string()));
    }

    let (best_scan_idx, _) = scan_results
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            a.rps
                .partial_cmp(&b.rps)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap();

    let best_scan_conn = scan_results[best_scan_idx].connections;
    let best_scan_rps = scan_results[best_scan_idx].rps;

    info!(
        "Best scan result: {} connections with {:.2} req/sec",
        best_scan_conn, best_scan_rps
    );

    let min_candidate = ((best_scan_conn as f64) * (1.0 - FULL_BENCHMARK_RANGE)) as u32;
    let max_candidate = ((best_scan_conn as f64) * (1.0 + FULL_BENCHMARK_RANGE)) as u32;

    let scanned_min = scan_results.first().map(|r| r.connections).unwrap_or(1);
    let scanned_max = scan_results
        .last()
        .map(|r| r.connections)
        .unwrap_or(best_scan_conn);

    let mut candidate_connections: Vec<u32> =
        vec![min_candidate.max(1), best_scan_conn, max_candidate.max(1)];

    candidate_connections.retain(|c| *c >= scanned_min && *c <= scanned_max);
    candidate_connections.sort_unstable();
    candidate_connections.dedup();

    info!(
        "Starting full benchmark phase (duration {}s) for {} connection counts in range [{}, {}]: {:?}",
        full_duration,
        candidate_connections.len(),
        min_candidate,
        max_candidate,
        candidate_connections
    );

    let mut all_full_results: Vec<(u32, f64, WrkResult)> = Vec::new();

    for connections in candidate_connections {
        info!("Running full benchmark with {} connections", connections);

        let result = env
            .exec_wrk_with_connections(app_endpoint, script.clone(), connections, full_duration)
            .await?;

        let latency_ms = result.latency_avg.as_secs_f64() * 1000.0;

        info!(
            "Full result: {} connections -> {:.2} req/sec, {:.2}ms avg latency, {} errors",
            connections, result.requests_per_sec, latency_ms, result.errors
        );

        if result.errors <= MAX_ALLOWED_ERRORS {
            all_full_results.push((connections, latency_ms, result));
        } else {
            info!(
                "Skipping result with errors ({} errors) at {} connections",
                result.errors, connections
            );
        }
    }

    if all_full_results.is_empty() {
        return Err(Error::System(
            "No valid benchmark results from full phase".to_string(),
        ));
    }

    all_full_results.sort_by(|a, b| {
        b.2.requests_per_sec
            .partial_cmp(&a.2.requests_per_sec)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let top_3_count = std::cmp::min(3, all_full_results.len());
    let best_from_top3 = all_full_results
        .iter()
        .take(top_3_count)
        .max_by(|a, b| {
            a.2.requests_per_sec
                .partial_cmp(&b.2.requests_per_sec)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap();

    info!(
        "Top 3 results by RPS: {}",
        all_full_results
            .iter()
            .take(3)
            .map(|(c, _, r)| format!("{} conn: {:.2} req/sec", c, r.requests_per_sec))
            .collect::<Vec<_>>()
            .join(", ")
    );

    info!(
        "Selected final result: {} connections with {:.2} req/sec, {:.2}ms latency",
        best_from_top3.0, best_from_top3.2.requests_per_sec, best_from_top3.1
    );

    Ok(best_from_top3.2.clone())
}
