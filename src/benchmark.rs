use humanize_bytes::humanize_bytes_binary;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{sync::Arc, time::Duration};
use tokio::select;
use tokio_util::sync::CancellationToken;

use crate::{docker, http_probe, prelude::*, wrk, wrk::WrkResult};

const BENCHMARK_WARMUP_SECS: u64 = 5;
const BENCHMARK_WARMUP_COOL_DOWN_SECS: u64 = 2;
const BENCHMARK_DURATION_SECS: u64 = 15;
const BENCHMARK_THREADS: u32 = 2;
const BENCHMARK_CONNECTIONS: u32 = 32;
const BENCHMARK_CPUS: u32 = 4;
const BENCHMARK_MEMORY_MB: u32 = 1024;
const DB_CPUS: u32 = 8;
const DB_MEMORY_MB: u32 = 2048;

const DB_PATH: &str = "benchmarks_db";

#[allow(dead_code)]
#[derive(Debug)]
pub struct BenchmarkResults {
    pub version: String,
    pub results: HashMap<BenchmarkTests, BenchmarkResult>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct BenchmarkResult {
    pub wrk_result: WrkResult,
    pub memory_usage: u64,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum BenchmarkTests {
    HelloWorld,
    Json,
    DbReadOne,
    DbReadPaging,
    DbWrite,
}

pub async fn run_benchmark(path: &Path) -> Result<BenchmarkResults> {
    info!("Running benchmark for path: {:?}", path);
    let docker_tag = uuid::Uuid::new_v4().to_string();
    let container_name = uuid::Uuid::new_v4().to_string();

    info!("Building docker image for server");
    docker::exec_build(path, &docker_tag).await?;

    info!("Building docker image for benchmark DB");
    let db_image_tag = "benchmark_db:latest".to_string();
    docker::exec_build(Path::new(DB_PATH), &db_image_tag).await?;

    docker::exec_run_container(
        "db",
        &db_image_tag,
        None::<String>,
        DB_CPUS,
        DB_MEMORY_MB,
        None::<String>,
    )
    .await?;
    docker::exec_run_container(
        &container_name,
        &docker_tag,
        Some("8000:8000"),
        BENCHMARK_CPUS,
        BENCHMARK_MEMORY_MB,
        Some("db:db"),
    )
    .await?;
    http_probe::wait_server_ready("localhost:8000", Duration::from_secs(60)).await?;
    let server_info = http_probe::get_server_version("localhost:8000").await?;
    info!(
        "Version: {}, Tests: {:?}",
        server_info.version, server_info.supported_tests
    );
    let version = server_info.version.clone();
    docker::exec_stop_container(&container_name).await?;
    docker::exec_stop_container("db").await?;

    let mut results = HashMap::new();
    for test in server_info.supported_tests {
        info!("Running benchmark test: {:?}", test);
        docker::exec_run_container(
            "db",
            &db_image_tag,
            None::<String>,
            DB_CPUS,
            DB_MEMORY_MB,
            None::<String>,
        )
        .await?;
        docker::exec_run_container(
            &container_name,
            &docker_tag,
            Some("8000:8000"),
            BENCHMARK_CPUS,
            BENCHMARK_MEMORY_MB,
            Some("db:db"),
        )
        .await?;
        http_probe::wait_server_ready("localhost:8000", Duration::from_secs(60)).await?;
        let server_info = http_probe::get_server_version("localhost:8000").await?;
        info!(
            "Version: {}, Tests: {:?}",
            server_info.version, server_info.supported_tests
        );

        info!("Warmup run");
        let _ = wrk::start_wrk(
            "http://localhost:8000",
            BENCHMARK_WARMUP_SECS,
            BENCHMARK_THREADS,
            BENCHMARK_CONNECTIONS,
            None,
        )
        .await?;
        tokio::time::sleep(Duration::from_secs(BENCHMARK_WARMUP_COOL_DOWN_SECS)).await;

        let monitor_handle_token = CancellationToken::new();
        let monitor_handle =
            monitor_docker_memory_usage(&container_name, monitor_handle_token.clone());
        info!("Starting benchmark run for test: {:?}", test);
        let wrk_result = match test {
            BenchmarkTests::HelloWorld => {
                wrk::start_wrk(
                    "http://localhost:8000",
                    BENCHMARK_DURATION_SECS,
                    BENCHMARK_THREADS,
                    BENCHMARK_CONNECTIONS,
                    Some("scripts/wrk_hello.lua"),
                )
                .await?
            }
            BenchmarkTests::Json => {
                wrk::start_wrk(
                    "http://localhost:8000",
                    BENCHMARK_DURATION_SECS,
                    BENCHMARK_THREADS,
                    BENCHMARK_CONNECTIONS,
                    Some("scripts/wrk_json.lua"),
                )
                .await?
            }
            BenchmarkTests::DbReadOne => {
                wrk::start_wrk(
                    "http://localhost:8000",
                    BENCHMARK_DURATION_SECS,
                    BENCHMARK_THREADS,
                    BENCHMARK_CONNECTIONS,
                    Some("scripts/wrk_db_read_one.lua"),
                )
                .await?
            }
            BenchmarkTests::DbReadPaging => {
                wrk::start_wrk(
                    "http://localhost:8000",
                    BENCHMARK_DURATION_SECS,
                    BENCHMARK_THREADS,
                    BENCHMARK_CONNECTIONS,
                    Some("scripts/wrk_db_read_paging.lua"),
                )
                .await?
            }
            BenchmarkTests::DbWrite => {
                wrk::start_wrk(
                    "http://localhost:8000",
                    BENCHMARK_DURATION_SECS,
                    BENCHMARK_THREADS,
                    BENCHMARK_CONNECTIONS,
                    Some("scripts/wrk_db_write.lua"),
                )
                .await?
            }
        };
        monitor_handle_token.cancel();
        let memory_usage = monitor_handle.await?;

        info!("Stopping docker container");
        docker::exec_stop_container(&container_name).await?;
        docker::exec_stop_container("db").await?;
        info!(
            "Benchmark completed for test: {:?}, req/sec: {}, mem: {}",
            test,
            wrk_result.requests_per_sec,
            humanize_bytes_binary!(memory_usage)
        );
        results.insert(
            test,
            BenchmarkResult {
                wrk_result,
                memory_usage,
            },
        );
    }

    Ok(BenchmarkResults { version, results })
}

fn monitor_docker_memory_usage(
    container_id: &str,
    cancel_token: CancellationToken,
) -> tokio::task::JoinHandle<u64> {
    let peak_memory = Arc::new(AtomicU64::new(0));
    let container_id = container_id.to_string();
    tokio::spawn(async move {
        loop {
            if let Ok(info) = crate::docker::exec_stats(&container_id).await {
                let mem = info.memory_usage;
                let prev = peak_memory.load(Ordering::Relaxed);
                if mem > prev {
                    peak_memory.store(mem, Ordering::Relaxed);
                }
            }
            select! {
                _ = cancel_token.cancelled() => {
                    break;
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {

                }
            }
        }
        peak_memory.load(Ordering::Relaxed)
    })
}

impl TryFrom<&str> for BenchmarkTests {
    type Error = String;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            "hello_world" => Ok(BenchmarkTests::HelloWorld),
            "json" => Ok(BenchmarkTests::Json),
            "db_read_one" => Ok(BenchmarkTests::DbReadOne),
            "db_read_paging" => Ok(BenchmarkTests::DbReadPaging),
            "db_write" => Ok(BenchmarkTests::DbWrite),
            _ => Err(format!("Unknown benchmark test: {}", value)),
        }
    }
}
