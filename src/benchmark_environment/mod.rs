pub mod common;
pub mod config;
pub mod local;
pub mod remote;

use crate::database::DatabaseKind;
use crate::{prelude::*, wrk::WrkResult};
use config::{EnvironmentFile, EnvironmentKind};
use std::{path::PathBuf, time::Duration};

pub struct Endpoint {
    pub address: String,
    pub port: u16,
}

pub struct ServerUsage {
    pub memory_usage_bytes: u64,
}

fn percentile_latency(result: &WrkResult, percentile: u8) -> Option<Duration> {
    result
        .latency_distribution
        .iter()
        .find(|(p, _)| *p == percentile)
        .map(|(_, d)| *d)
}

#[async_trait::async_trait]
pub trait BenchmarkEnvironment: Send + Sync {
    async fn prepare(
        &mut self,
        framework_path: &std::path::Path,
        database: Option<DatabaseKind>,
        app_env: &[(String, String)],
        app_args: &[String],
    ) -> Result<()>;

    async fn start_db(&mut self) -> Result<Option<Endpoint>>;
    async fn stop_db(&mut self) -> Result<()>;

    async fn start_app(&mut self, db_endpoint: Option<&Endpoint>) -> Result<Endpoint>;
    async fn stop_app(&mut self) -> Result<ServerUsage>;

    async fn exec_wrk_warmup(
        &self,
        app_endpoint: &Endpoint,
        script: &str,
        duration_secs: u64,
    ) -> Result<WrkResult>;

    fn wrk_duration(&self) -> u64;

    async fn exec_wrk_with_connections(
        &self,
        app_endpoint: &Endpoint,
        script: String,
        connections: u32,
        duration: u64,
    ) -> Result<WrkResult>;
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
    const START_CONNECTIONS: u32 = 16;
    const MAX_CONNECTIONS: u32 = 16_384;
    const PROBE_DURATION: u64 = 10;
    const LATENCY_LIMIT_MS: u64 = 100;
    const RPS_DROP_RATIO: f64 = 0.8; // Fail if RPS < 80% of peak
    const PRECISION_CONNECTIONS: u32 = 10;

    #[derive(Clone)]
    struct Sample {
        connections: u32,
        result: WrkResult,
        p99_latency: Duration,
        fail_reason: Option<String>,
    }

    let full_duration = env.wrk_duration();
    let mut samples: Vec<Sample> = Vec::new();
    let mut peak_rps = 0.0;
    let mut best_sample: Option<Sample> = None;

    let mut next_connections = START_CONNECTIONS;
    let mut lower_bound: Option<u32> = None;
    let mut upper_bound: Option<u32> = None;

    // Phase 1: Growth
    loop {
        if next_connections > MAX_CONNECTIONS {
            break;
        }

        info!("Adaptive probe: {} connections", next_connections);
        let result = env
            .exec_wrk_with_connections(
                app_endpoint,
                script.clone(),
                next_connections,
                PROBE_DURATION,
            )
            .await?;

        let p99_latency = percentile_latency(&result, 99).ok_or_else(|| {
            Error::System("Missing 99th percentile latency from wrk output".to_string())
        })?;

        let mut fail_reason = None;
        if result.errors > 0 {
            fail_reason = Some(format!("errors detected ({})", result.errors));
        } else if p99_latency.as_millis() as u64 > LATENCY_LIMIT_MS {
            fail_reason = Some(format!(
                "p99 latency {}ms > {}ms limit",
                p99_latency.as_millis(),
                LATENCY_LIMIT_MS
            ));
        } else if peak_rps > 0.0 && result.requests_per_sec < peak_rps * RPS_DROP_RATIO {
            fail_reason = Some(format!(
                "RPS {:.2} < 80% of peak {:.2}",
                result.requests_per_sec, peak_rps
            ));
        }

        let sample = Sample {
            connections: next_connections,
            result: result.clone(),
            p99_latency,
            fail_reason: fail_reason.clone(),
        };
        samples.push(sample.clone());

        info!(
            "Result: {} conn -> {:.2} rps, p99 {}ms, {}",
            next_connections,
            result.requests_per_sec,
            p99_latency.as_millis(),
            fail_reason.as_deref().unwrap_or("pass")
        );

        if fail_reason.is_none() {
            // Pass
            if result.requests_per_sec > peak_rps {
                peak_rps = result.requests_per_sec;
            }
            if best_sample
                .as_ref()
                .map_or(true, |b| result.requests_per_sec > b.result.requests_per_sec)
            {
                best_sample = Some(sample.clone());
            }
            lower_bound = Some(next_connections);
            next_connections *= 2;
        } else {
            // Fail
            upper_bound = Some(next_connections);
            break; // Stop growth, move to refinement
        }
    }

    // Phase 2: Refinement (Binary Search)
    if let (Some(low), Some(high)) = (lower_bound, upper_bound) {
        let mut l = low;
        let mut r = high;
        let mut iterations = 0;

        while r - l > PRECISION_CONNECTIONS {
            if iterations >= 20 {
                info!("Refinement iteration limit reached (20)");
                break;
            }
            // Stop if the range is within 5% of the lower bound
            if (r - l) as f64 / l as f64 <= 0.05 {
                info!("Refinement precision reached (5%)");
                break;
            }
            iterations += 1;

            let mid = l + (r - l) / 2;
            if mid <= l {
                break;
            } // Should not happen given condition

            info!("Refining probe: {} connections", mid);
            let result = env
                .exec_wrk_with_connections(
                    app_endpoint,
                    script.clone(),
                    mid,
                    PROBE_DURATION,
                )
                .await?;

            let p99_latency = percentile_latency(&result, 99).ok_or_else(|| {
                Error::System("Missing 99th percentile latency from wrk output".to_string())
            })?;

            let mut fail_reason = None;
            if result.errors > 0 {
                fail_reason = Some(format!("errors detected ({})", result.errors));
            } else if p99_latency.as_millis() as u64 > LATENCY_LIMIT_MS {
                fail_reason = Some(format!(
                    "p99 latency {}ms > {}ms limit",
                    p99_latency.as_millis(),
                    LATENCY_LIMIT_MS
                ));
            } else if peak_rps > 0.0 && result.requests_per_sec < peak_rps * RPS_DROP_RATIO {
                fail_reason = Some(format!(
                    "RPS {:.2} < 80% of peak {:.2}",
                    result.requests_per_sec, peak_rps
                ));
            }

            let sample = Sample {
                connections: mid,
                p99_latency,
                result: result.clone(),
                fail_reason: fail_reason.clone(),
            };
            samples.push(sample.clone());

            info!(
                "Result: {} conn -> {:.2} rps, p99 {}ms, {}",
                mid,
                result.requests_per_sec,
                p99_latency.as_millis(),
                fail_reason.as_deref().unwrap_or("pass")
            );

            if fail_reason.is_none() {
                if result.requests_per_sec > peak_rps {
                    peak_rps = result.requests_per_sec;
                }
                if best_sample
                    .as_ref()
                    .map_or(true, |b| result.requests_per_sec > b.result.requests_per_sec)
                {
                    best_sample = Some(sample.clone());
                }
                l = mid;
            } else {
                r = mid;
            }
        }
    }

    samples.sort_by_key(|s| s.connections);
    info!("Benchmark results:");
    for sample in &samples {
        info!(
            "  {} connections -> {:.2} rps, p99 {}ms, {}",
            sample.connections,
            sample.result.requests_per_sec,
            sample.p99_latency.as_millis(),
            sample.fail_reason.as_deref().unwrap_or("pass")
        );
    }

    // Final Selection
    let final_sample = best_sample
        .or_else(|| samples.iter().min_by_key(|s| s.connections).cloned())
        .ok_or_else(|| Error::System("No benchmark samples collected".to_string()))?;

    if let Some(reason) = &final_sample.fail_reason {
        warn!(
            "No successful benchmark run found. Using fallback run with {} connections (Reason: {}).",
            final_sample.connections,
            reason
        );
    } else {
        info!(
            "Selected best run: {} connections, {:.2} rps. Running full benchmark ({}s)...",
            final_sample.connections, final_sample.result.requests_per_sec, full_duration
        );
    }

    let final_result = env
        .exec_wrk_with_connections(
            app_endpoint,
            script.clone(),
            final_sample.connections,
            full_duration,
        )
        .await?;

    let final_p99 = percentile_latency(&final_result, 99).ok_or_else(|| {
        Error::System("Missing 99th percentile latency from wrk output".to_string())
    })?;

    info!(
        "Final result: {} connections -> {:.2} rps, avg {:.2}ms, p99 {}ms, errors {}",
        final_sample.connections,
        final_result.requests_per_sec,
        final_result.latency_avg.as_secs_f64() * 1000.0,
        final_p99.as_millis(),
        final_result.errors
    );

    Ok(final_result)
}
