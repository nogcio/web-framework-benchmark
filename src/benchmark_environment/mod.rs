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
    const LOAD_MULTIPLIER: f64 = 1.7; // exponential growth factor while searching
    const MAX_CONNECTIONS: u32 = 16_384;
    const MAX_ITERATIONS: usize = 32;
    const PRECISION_CONNECTIONS: u32 = 10;
    const PROBE_DURATION: u64 = 10;
    const BASE_P99_LATENCY_LIMIT_MS: u64 = 100;
    const P99_LATENCY_PER_KB_MS: f64 = 0.20; // allow larger payloads to take proportionally longer
    const P99_LATENCY_LIMIT_MAX_MS: u64 = 5_000;
    const RPS_DROP_THRESHOLD: f64 = 0.20;
    const STDEV_LATENCY_RATIO_LIMIT: f64 = 0.80;
    const STDEV_ABS_FAIL_MIN: Duration = Duration::from_millis(1); // avoid tripping on sub-ms jitter
    const LOW_LATENCY_P99_IGNORE: Duration = Duration::from_millis(50); // allow jitter when latency is tiny

    #[derive(Clone)]
    struct Sample {
        connections: u32,
        result: WrkResult,
        p99_latency: Duration,
        stdev_ratio: f64,
        fail_reason: Option<String>,
    }

    let full_duration = env.wrk_duration();

    let mut samples: Vec<Sample> = Vec::new();
    let mut lower_pass: Option<Sample> = None; // best known passing point (lower bound)
    let mut upper_fail: Option<Sample> = None; // first failing point (upper bound)
    let mut best_by_rps: Option<Sample> = None;

    let mut iteration: usize = 0;
    let mut next_connections = START_CONNECTIONS;
    let mut last_sample: Option<Sample> = None;

    while iteration < MAX_ITERATIONS && next_connections > 0 && next_connections <= MAX_CONNECTIONS
    {
        iteration += 1;

        info!(
            "Adaptive iteration {}: {} connections ({}s probe)",
            iteration, next_connections, PROBE_DURATION
        );

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

        let stdev_ratio = if result.latency_avg.as_secs_f64() > 0.0 {
            result.latency_stdev.as_secs_f64() / result.latency_avg.as_secs_f64()
        } else {
            0.0
        };

        let payload_kb = if result.requests_per_sec > 0.0 {
            (result.transfer_per_sec as f64 / result.requests_per_sec) / 1024.0
        } else {
            0.0
        };

        let p99_limit_ms = (BASE_P99_LATENCY_LIMIT_MS as f64 + payload_kb * P99_LATENCY_PER_KB_MS)
            .min(P99_LATENCY_LIMIT_MAX_MS as f64)
            .max(BASE_P99_LATENCY_LIMIT_MS as f64);

        let p99_limit = Duration::from_millis(p99_limit_ms.round() as u64);

        let rps_drop = last_sample.as_ref().and_then(|prev| {
            if next_connections > prev.connections && prev.result.requests_per_sec > 0.0 {
                Some(
                    (prev.result.requests_per_sec - result.requests_per_sec)
                        / prev.result.requests_per_sec,
                )
            } else {
                None
            }
        });

        let mut fail_reason = None;
        if result.errors > 0 {
            fail_reason = Some(format!("errors detected ({})", result.errors));
        } else if p99_latency > p99_limit {
            fail_reason = Some(format!(
                "p99 latency {}ms > {}ms limit",
                p99_latency.as_millis(),
                p99_limit.as_millis()
            ));
        } else if let Some(drop) = rps_drop
            && drop >= RPS_DROP_THRESHOLD
        {
            fail_reason = Some(format!("RPS dropped by {:.1}%", drop * 100.0));
        }

        let unstable = p99_latency >= LOW_LATENCY_P99_IGNORE
            && stdev_ratio > STDEV_LATENCY_RATIO_LIMIT
            && result.latency_stdev >= STDEV_ABS_FAIL_MIN;

        if fail_reason.is_none() && unstable {
            if lower_pass.is_some() {
                // Only start failing on instability after we observed at least one stable point.
                fail_reason = Some(format!(
                    "unstable latency: stdev {:.1}% of avg",
                    stdev_ratio * 100.0
                ));
            } else {
                // First probe: treat as warning so baseline doesn't immediately fail on jitter.
                warn!(
                    "High jitter on baseline: stdev {:.1}% ({}ms) of avg {}ms",
                    stdev_ratio * 100.0,
                    result.latency_stdev.as_millis(),
                    result.latency_avg.as_millis()
                );
            }
        }

        info!(
            "Result: {} conn -> {:.2} rps, avg {:.2}ms, p99 {}ms, stdev {:.1}%, errors {}{}",
            next_connections,
            result.requests_per_sec,
            result.latency_avg.as_secs_f64() * 1000.0,
            p99_latency.as_millis(),
            stdev_ratio * 100.0,
            result.errors,
            rps_drop
                .map(|d| format!("; ΔRPS {:.1}%", -d * 100.0))
                .unwrap_or_default()
        );

        let sample = Sample {
            connections: next_connections,
            result: result.clone(),
            p99_latency,
            stdev_ratio,
            fail_reason: fail_reason.clone(),
        };

        last_sample = Some(sample.clone());
        samples.push(sample.clone());

        let passed = fail_reason.is_none();

        if passed {
            if best_by_rps
                .as_ref()
                .map(|s| s.result.requests_per_sec)
                .unwrap_or(-1.0)
                < sample.result.requests_per_sec
            {
                best_by_rps = Some(sample.clone());
            }

            if lower_pass
                .as_ref()
                .map(|s| s.connections < sample.connections)
                .unwrap_or(true)
            {
                lower_pass = Some(sample.clone());
            }

            if let Some(high) = upper_fail.as_ref() {
                let low_conn = sample.connections;
                let high_conn = high.connections;

                if high_conn <= low_conn + PRECISION_CONNECTIONS {
                    info!(
                        "Boundary pinned between {} and {} connections (±{}), stopping",
                        low_conn, high_conn, PRECISION_CONNECTIONS
                    );
                    break;
                }

                let mut mid = low_conn + (high_conn - low_conn) / 2;
                if mid <= low_conn {
                    mid = low_conn + 1;
                }
                next_connections = mid;
            } else {
                let mut scaled = ((sample.connections as f64) * LOAD_MULTIPLIER).ceil() as u32;
                if scaled <= sample.connections {
                    scaled = sample.connections + 1;
                }

                if scaled > MAX_CONNECTIONS {
                    info!(
                        "Reached max_connections {}, stopping growth",
                        MAX_CONNECTIONS
                    );
                    break;
                }

                next_connections = scaled;
            }
        } else {
            if upper_fail
                .as_ref()
                .map(|s| sample.connections < s.connections)
                .unwrap_or(true)
            {
                upper_fail = Some(sample.clone());
            }

            match lower_pass.as_ref() {
                Some(low) => {
                    let low_conn = low.connections;
                    let high_conn = upper_fail.as_ref().unwrap().connections;

                    if high_conn <= low_conn + PRECISION_CONNECTIONS {
                        info!(
                            "Boundary narrowed to {}..{} connections (±{}), stopping",
                            low_conn, high_conn, PRECISION_CONNECTIONS
                        );
                        break;
                    }

                    let mut mid = low_conn + (high_conn - low_conn) / 2;
                    if mid <= low_conn {
                        mid = low_conn + 1;
                    }

                    next_connections = mid;
                }
                None => {
                    return Err(Error::System(
                        "Server fails baseline load (no successful run)".to_string(),
                    ));
                }
            }
        }

        if next_connections > MAX_CONNECTIONS {
            info!(
                "Next step {} exceeds max_connections {}, stopping",
                next_connections, MAX_CONNECTIONS
            );
            break;
        }
    }

    if samples.is_empty() {
        return Err(Error::System("No benchmark samples collected".to_string()));
    }

    let stable_sample = match (lower_pass.clone(), best_by_rps.clone()) {
        (Some(l), Some(b)) => {
            if b.result.requests_per_sec >= l.result.requests_per_sec {
                b
            } else {
                l
            }
        }
        (Some(l), None) => l,
        (None, Some(b)) => b,
        (None, None) => {
            return Err(Error::System("No successful benchmark run".to_string()));
        }
    };

    let boundary_reason = upper_fail
        .as_ref()
        .and_then(|s| s.fail_reason.clone())
        .unwrap_or_else(|| "limit not reached (max connections/iterations)".to_string());

    samples.sort_by_key(|s| s.connections);
    info!("Performance curve (connections -> rps, p99, stdev%, status):");
    for s in &samples {
        info!(
            "  {} -> {:.2} rps, p99 {}ms, stdev {:.1}%, {}, {}",
            s.connections,
            s.result.requests_per_sec,
            s.p99_latency.as_millis(),
            s.stdev_ratio * 100.0,
            if s.fail_reason.is_none() {
                "pass"
            } else {
                "fail"
            },
            s.fail_reason.as_deref().unwrap_or("stable")
        );
    }

    info!(
        "Stable boundary: {} connections (reason: {})",
        stable_sample.connections, boundary_reason
    );
    info!(
        "Recommendation: keep connections <= {} for p99 {}ms and stdev {:.1}%",
        stable_sample.connections,
        stable_sample.p99_latency.as_millis(),
        stable_sample.stdev_ratio * 100.0
    );

    info!(
        "Running full-duration benchmark at {} connections ({}s)",
        stable_sample.connections, full_duration
    );

    let final_result = env
        .exec_wrk_with_connections(
            app_endpoint,
            script.clone(),
            stable_sample.connections,
            full_duration,
        )
        .await?;

    let final_p99 = percentile_latency(&final_result, 99).ok_or_else(|| {
        Error::System("Missing 99th percentile latency from wrk output".to_string())
    })?;

    info!(
        "Final result: {} connections -> {:.2} rps, avg {:.2}ms, p99 {}ms, errors {}",
        stable_sample.connections,
        final_result.requests_per_sec,
        final_result.latency_avg.as_secs_f64() * 1000.0,
        final_p99.as_millis(),
        final_result.errors
    );

    Ok(final_result)
}
