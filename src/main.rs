mod benchmark;
mod benchmark_environment;
mod cli;
mod database;
mod db;
mod docker;
mod error;
mod exec_utils;
mod http;
mod http_probe;
mod parsers;
mod wrk;
mod analysis;
mod analysis_context;

pub mod prelude {
    pub use crate::error::*;
    pub use crate::exec_utils::*;
    pub use tracing::{debug, error, info, span, trace, warn};
}

use rand::RngCore;
use rand::rngs::OsRng;
use std::env;
use std::fs::OpenOptions;
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;

use clap::Parser;
use dotenvy::dotenv;
use prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    info!(
        "{} v{}",
        env!("CARGO_PKG_DESCRIPTION"),
        env!("CARGO_PKG_VERSION")
    );

    ensure_benchmark_files().await?;

    let cli = cli::Cli::parse();
    match cli.command {
        cli::Commands::Run {
            id,
            environment,
            filter,
        } => {
            let db = db::Db::open()?;
            let benchmarks = db.get_benchmarks()?;
            let has_only = benchmarks.iter().any(|b| b.only);
            for benchmark in benchmarks {
                if let Some(filter) = &filter
                    && !benchmark.name.contains(filter)
                {
                    continue;
                }
                if has_only && !benchmark.only {
                    debug!(
                        "Skipping benchmark {} because other benchmarks are marked as only",
                        benchmark.name
                    );
                    continue;
                }
                if benchmark.disabled {
                    info!("Skipping disabled benchmark {}", benchmark.name);
                    continue;
                }

                let completed_tests =
                    db.get_completed_tests(id, &environment, &benchmark.language, &benchmark.name)?;
                let allowed_tests: Vec<_> = benchmark
                    .tests
                    .iter()
                    .filter(|t| !completed_tests.contains(t))
                    .cloned()
                    .collect();

                if allowed_tests.is_empty() {
                    info!(
                        "Skipping benchmark {} because all tests are already completed",
                        benchmark.name
                    );
                    continue;
                }

                let mut env = crate::benchmark_environment::load_environment(&environment)?;
                let benchmark_results =
                    benchmark::run_benchmark(&mut *env, &benchmark, &allowed_tests).await?;
                db.save_run(id, &environment, &benchmark, &benchmark_results)?;
            }
        }
        cli::Commands::Serve { host, port } => {
            let db = db::Db::open()?;
            let app = http::create_router(db);
            let addr: std::net::SocketAddr = format!("{}:{}", host, port).parse().unwrap();
            info!("Starting server on {}", addr);
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            axum::serve(listener, app.into_make_service())
                .await
                .unwrap();
        }
        cli::Commands::Analyze {
            run_id,
            api_key,
            model,
            api_url,
            languages,
        } => {
            let db = db::Db::open()?;
            analysis::run_analysis(db, run_id, api_key, model, api_url, languages).await?;
        }
    }

    Ok(())
}

async fn ensure_benchmark_files() -> std::result::Result<(), io::Error> {
    let mut base = std::env::current_dir()?;
    base.push("benchmarks_data");

    fn create_if_missing_random(path: PathBuf, size: u64) -> std::result::Result<(), io::Error> {
        if path.exists() {
            return Ok(());
        }
        let f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;
        let mut w = BufWriter::new(f);
        let mut remaining = size;
        let mut buf = vec![0u8; 64 * 1024]; // 64KB buffer
        let mut rng = OsRng;
        while remaining > 0 {
            let to_write = std::cmp::min(remaining, buf.len() as u64) as usize;
            rng.fill_bytes(&mut buf[..to_write]);
            w.write_all(&buf[..to_write])?;
            remaining -= to_write as u64;
        }
        w.flush()?;
        Ok(())
    }

    // sizes in bytes
    let f15 = base.join("15kb.bin");
    let f1m = base.join("1mb.bin");
    let f10m = base.join("10mb.bin");

    create_if_missing_random(f15, 15 * 1024)?;
    create_if_missing_random(f1m, 1024 * 1024)?;
    create_if_missing_random(f10m, 10 * 1024 * 1024)?;

    Ok(())
}
