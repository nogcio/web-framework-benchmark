use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use humanize_bytes::humanize_bytes_binary;

use crate::benchmark_environment::{BenchmarkEnvironment, run_adaptive_connections};
use crate::prelude::*;
use crate::wrk::WrkResult;

const BENCHMARK_WARMUP_COOL_DOWN_SECS: u64 = 2;

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

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum BenchmarkTests {
    HelloWorld,
    Json,
    DbReadOne,
    DbReadPaging,
    DbWrite,
    StaticFiles,
}

pub async fn run_benchmark(env: &mut dyn BenchmarkEnvironment, path: &Path) -> Result<BenchmarkResults> {
    info!("Running benchmark for path: {:?}", path);
    env.prepare(path).await?;

    // start once to query server info
    let db_ep = env.start_db().await?;
    let app_ep = env.start_app(&db_ep).await?;
    let server_info = env.get_app_info(&app_ep).await?;
    info!(
        "Version: {}, Tests: {:?}",
        server_info.version, server_info.supported_tests
    );
    let version = server_info.version.clone();
    // stop the temporary run
    let _ = env.stop_app().await?;
    env.stop_db().await?;

    let mut tests = server_info.supported_tests.clone();
    tests.sort();
    tests.dedup();

    let mut results = HashMap::new();
    for test in tests {
        info!("Running benchmark test: {:?}", test);
        let db_ep = env.start_db().await?;
        let app_ep = env.start_app(&db_ep).await?;

        info!("Warmup run");
        let _ = env.exec_wrk_warmup(&app_ep).await?;
        tokio::time::sleep(Duration::from_secs(BENCHMARK_WARMUP_COOL_DOWN_SECS)).await;

        info!("Starting adaptive benchmark run for test: {:?}", test);
        let script = match test {
            BenchmarkTests::HelloWorld => "scripts/wrk_hello.lua",
            BenchmarkTests::Json => "scripts/wrk_json.lua",
            BenchmarkTests::DbReadOne => "scripts/wrk_db_read_one.lua",
            BenchmarkTests::DbReadPaging => "scripts/wrk_db_read_paging.lua",
            BenchmarkTests::DbWrite => "scripts/wrk_db_write.lua",
            BenchmarkTests::StaticFiles => "scripts/wrk_static_files.lua",
        };
        let wrk_result = run_adaptive_connections(env, &app_ep, script.to_string()).await?;

        // stop app and db, get memory usage from env
        let usage = env.stop_app().await?;
        env.stop_db().await?;

        info!(
            "Benchmark completed for test: {:?}, req/sec: {}, mem: {}, errors: {}",
            test,
            wrk_result.requests_per_sec,
            humanize_bytes_binary!(usage.memory_usage_bytes),
            wrk_result.errors
        );
        results.insert(
            test,
            BenchmarkResult {
                wrk_result,
                memory_usage: usage.memory_usage_bytes,
            },
        );
    }

    Ok(BenchmarkResults { version, results })
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
            "static_files" => Ok(BenchmarkTests::StaticFiles),
            _ => Err(format!("Unknown benchmark test: {}", value)),
        }
    }
}

impl std::fmt::Display for BenchmarkTests {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            BenchmarkTests::HelloWorld => "hello_world",
            BenchmarkTests::Json => "json",
            BenchmarkTests::DbReadOne => "db_read_one",
            BenchmarkTests::DbReadPaging => "db_read_paging",
            BenchmarkTests::DbWrite => "db_write",
            BenchmarkTests::StaticFiles => "static_files",
        };
        write!(f, "{}", s)
    }
}
