use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use humanize_bytes::humanize_bytes_binary;

use crate::benchmark_environment::{BenchmarkEnvironment, run_adaptive_connections};
use crate::db::benchmarks::Benchmark;
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

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkTests {
    HelloWorld,
    Json,
    DbReadOne,
    DbReadPaging,
    DbWrite,
    StaticFilesSmall,
    StaticFilesMedium,
    StaticFilesLarge,
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
            "static_files_small" => Ok(BenchmarkTests::StaticFilesSmall),
            "static_files_medium" => Ok(BenchmarkTests::StaticFilesMedium),
            "static_files_large" => Ok(BenchmarkTests::StaticFilesLarge),
            _ => Err(format!("Unknown benchmark test: {}", value)),
        }
    }
}

pub async fn run_benchmark(
    env: &mut dyn BenchmarkEnvironment,
    bench: &Benchmark,
) -> Result<BenchmarkResults> {
    let path = Path::new(&bench.path);
    info!(
        "Running benchmark for {:?} ({} / {})",
        path, bench.language, bench.framework
    );
    let app_env: Vec<(String, String)> = bench
        .env
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    env.prepare(path, bench.database, &app_env, &bench.arguments)
        .await?;

    let version = bench.framework_version.clone();

    let mut tests = bench.tests.clone();
    tests.sort();
    tests.dedup();

    let db_configured = bench.database.is_some();

    let mut results = HashMap::new();
    for test in tests {
        info!("Running benchmark test: {:?}", test);
        let requires_db = test_requires_db(&test);
        if requires_db && !db_configured {
            info!(
                "Skipping test {:?} because database is not configured for this benchmark",
                test
            );
            continue;
        }

        let db_ep = if db_configured {
            env.start_db().await?
        } else {
            None
        };
        let app_ep = env.start_app(db_ep.as_ref()).await?;

        let script = match test {
            BenchmarkTests::HelloWorld => "scripts/wrk_hello.lua",
            BenchmarkTests::Json => "scripts/wrk_json.lua",
            BenchmarkTests::DbReadOne => "scripts/wrk_db_read_one.lua",
            BenchmarkTests::DbReadPaging => "scripts/wrk_db_read_paging.lua",
            BenchmarkTests::DbWrite => "scripts/wrk_db_write.lua",
            BenchmarkTests::StaticFilesSmall => "scripts/wrk_static_files_small.lua",
            BenchmarkTests::StaticFilesMedium => "scripts/wrk_static_files_medium.lua",
            BenchmarkTests::StaticFilesLarge => "scripts/wrk_static_files_large.lua",
        };

        info!("Warmup run (10s) with {:?}", test);
        let _ = env.exec_wrk_warmup(&app_ep, script, 10).await?;
        tokio::time::sleep(Duration::from_secs(BENCHMARK_WARMUP_COOL_DOWN_SECS)).await;

        info!("Starting adaptive benchmark run for test: {:?}", test);
        let wrk_result = run_adaptive_connections(env, &app_ep, script.to_string()).await?;

        let usage = env.stop_app().await?;
        if db_configured {
            env.stop_db().await?;
        }

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

fn test_requires_db(test: &BenchmarkTests) -> bool {
    matches!(
        test,
        BenchmarkTests::DbReadOne | BenchmarkTests::DbReadPaging | BenchmarkTests::DbWrite
    )
}

impl std::fmt::Display for BenchmarkTests {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            BenchmarkTests::HelloWorld => "hello_world",
            BenchmarkTests::Json => "json",
            BenchmarkTests::DbReadOne => "db_read_one",
            BenchmarkTests::DbReadPaging => "db_read_paging",
            BenchmarkTests::DbWrite => "db_write",
            BenchmarkTests::StaticFilesSmall => "static_files_small",
            BenchmarkTests::StaticFilesMedium => "static_files_medium",
            BenchmarkTests::StaticFilesLarge => "static_files_large",
        };
        write!(f, "{}", s)
    }
}
