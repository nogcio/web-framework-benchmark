use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use humanize_bytes::humanize_bytes_binary;

use crate::benchmark_environment::{BenchmarkEnvironment, run_adaptive_connections};
use crate::database::DatabaseKind;
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
    pub samples: Vec<BenchmarkSample>,
}

#[derive(Debug, Clone)]
pub struct BenchmarkSample {
    pub connections: u32,
    pub result: WrkResult,
    pub p99_latency: Duration,
    pub fail_reason: Option<String>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkTests {
    HelloWorld,
    Json,
    DbReadOne,
    DbReadPaging,
    DbWrite,
    TweetService,
    StaticFilesSmall,
    StaticFilesMedium,
    StaticFilesLarge,
}

impl BenchmarkTests {
    pub fn description(&self) -> &'static str {
        match self {
            BenchmarkTests::HelloWorld => "This test measures the raw throughput of the web framework with minimal overhead. It sends a GET request to `/plaintext` which returns a 'Hello, World!' string. No database or complex logic is involved. This primarily tests the efficiency of the HTTP parser, routing, and basic request handling pipeline. It is CPU-bound and sensitive to overhead in the framework's core.",
            BenchmarkTests::Json => "This test measures the performance of JSON serialization. It sends a GET request to `/json` which returns a JSON object `{\"message\": \"Hello, World!\"}`. This tests the framework's ability to instantiate an object and serialize it to JSON. It is CPU-bound and stresses the JSON serializer and memory allocation.",
            BenchmarkTests::DbReadOne => "This test measures the performance of a single database query. It sends a GET request to `/db/read/one` (or similar) which fetches a single random row from the database and serializes it to JSON. This tests the framework's ORM or database driver, connection pooling, and the overhead of a network round-trip to the database. It is a mix of CPU and I/O bound.",
            BenchmarkTests::DbReadPaging => "This test measures the performance of fetching multiple rows from the database. It sends a GET request to `/db/read/many?offset=N&limit=50` which fetches 50 rows from the database and serializes them to a JSON list. This puts more load on the database driver and the JSON serializer than the single-read test. It tests the efficiency of handling multiple database queries or batch queries.",
            BenchmarkTests::DbWrite => "This test measures the performance of database writes. It sends a request to `/db/write/insert` (or similar) which inserts a new row into the database and returns the result as JSON. This tests the framework's ability to handle write operations and transaction overhead. It is heavily I/O bound and stresses the database's write capabilities.",
            BenchmarkTests::TweetService => "This is a complex scenario simulating a microblogging service. It involves user registration, login, creating tweets, and fetching timelines. It tests the framework's ability to handle complex business logic, multiple dependent database queries, authentication, and state management. It represents a realistic application workload.",
            BenchmarkTests::StaticFilesSmall => "This test measures the performance of serving a small static file (e.g., 1KB). It sends a GET request to retrieve a file from the disk. This tests the framework's static file handler, file I/O, and the efficiency of sending small payloads. It is often limited by the framework's overhead for handling requests.",
            BenchmarkTests::StaticFilesMedium => "This test measures the performance of serving a medium-sized static file (e.g., 100KB). It tests the framework's ability to handle larger payloads and efficiently stream data to the client. It shifts the bottleneck slightly more towards network bandwidth and memory copying compared to the small file test.",
            BenchmarkTests::StaticFilesLarge => "This test measures the performance of serving a large static file (e.g., 1MB). It stresses the network bandwidth and the framework's ability to handle long-lived connections and large data transfers. Efficiency in zero-copy file sending (sendfile) is critical here.",
        }
    }
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
            "tweet_service" => Ok(BenchmarkTests::TweetService),
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
    allowed_tests: &[BenchmarkTests],
) -> Result<BenchmarkResults> {
    let path = Path::new(&bench.path);
    info!(
        "Running benchmark for {} {:?} ({} / {})",
        bench.name, path, bench.language, bench.framework
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
        if !allowed_tests.contains(&test) {
            info!(
                "Skipping test {:?} because it is not in the allowed list",
                test
            );
            continue;
        }
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
            BenchmarkTests::DbReadOne => match bench.database {
                Some(DatabaseKind::Mongodb) => "scripts/wrk_db_read_one_mongo.lua",
                _ => "scripts/wrk_db_read_one.lua",
            },
            BenchmarkTests::DbReadPaging => match bench.database {
                Some(DatabaseKind::Mongodb) => "scripts/wrk_db_read_paging_mongo.lua",
                _ => "scripts/wrk_db_read_paging.lua",
            },
            BenchmarkTests::TweetService => match bench.database {
                Some(DatabaseKind::Mongodb) => "scripts/wrk_tweet_service_mongo.lua",
                _ => "scripts/wrk_tweet_service.lua",
            },
            BenchmarkTests::DbWrite => match bench.database {
                Some(DatabaseKind::Mongodb) => "scripts/wrk_db_write_mongo.lua",
                _ => "scripts/wrk_db_write.lua",
            },
            BenchmarkTests::StaticFilesSmall => "scripts/wrk_static_files_small.lua",
            BenchmarkTests::StaticFilesMedium => "scripts/wrk_static_files_medium.lua",
            BenchmarkTests::StaticFilesLarge => "scripts/wrk_static_files_large.lua",
        };

        info!("Warmup run (10s) with {:?}", test);
        let _ = env.exec_wrk_warmup(&app_ep, script, 10).await?;
        tokio::time::sleep(Duration::from_secs(BENCHMARK_WARMUP_COOL_DOWN_SECS)).await;

        info!("Starting adaptive benchmark run for test: {:?}", test);
        let (wrk_result, samples) =
            run_adaptive_connections(env, &app_ep, script.to_string()).await?;

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
                samples,
            },
        );
    }

    Ok(BenchmarkResults { version, results })
}

fn test_requires_db(test: &BenchmarkTests) -> bool {
    matches!(
        test,
        BenchmarkTests::DbReadOne
            | BenchmarkTests::DbReadPaging
            | BenchmarkTests::DbWrite
            | BenchmarkTests::TweetService
    )
}

impl std::fmt::Display for BenchmarkTests {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            BenchmarkTests::HelloWorld => "hello_world",
            BenchmarkTests::TweetService => "tweet_service",
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
