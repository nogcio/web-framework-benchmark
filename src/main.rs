mod benchmark;
mod benchmark_environment;
mod cli;
mod db;
mod docker;
mod error;
mod exec_utils;
mod http_probe;
mod parsers;
mod wrk;

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
use std::path::{Path, PathBuf};

use clap::{Parser, ValueEnum};
use dotenvy::dotenv;
use prelude::*;

#[derive(Debug, Clone, ValueEnum, serde::Deserialize, PartialEq)]
pub enum BenchmarkEnvironmentType {
    Local,
    Remote,
}

use crate::benchmark::BenchmarkResults;

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
        cli::Commands::Benchmark { path, environment } => {
            let result = run_benchmark_for_path(&path, &environment).await?;
            info!("Benchmark completed for {:?}: {:?}", path, result);
        }
        cli::Commands::Run { id, environment } => {
            let db = db::Db::open()?;
            let languages = db.get_languages()?;
            for lang in languages {
                for framework in &lang.frameworks {
                    let benchmark_path = PathBuf::from(&framework.path);
                    let benchmark_results =
                        run_benchmark_for_path(&benchmark_path, &environment).await?;
                    db.save_run(id, &environment, &lang, framework, &benchmark_results)?;
                }
            }
        }
    }

    Ok(())
}

async fn run_benchmark_for_path(
    path: &Path,
    environment: &BenchmarkEnvironmentType,
) -> Result<BenchmarkResults> {
    match environment {
        BenchmarkEnvironmentType::Local => {
            let settings = crate::benchmark_environment::local::LocalConfig::from_file(
                "config/environment.local.yaml",
            )?;
            let mut env =
                crate::benchmark_environment::local::LocalBenchmarkEnvironment::new(settings);
            let result = benchmark::run_benchmark(&mut env, path).await?;
            Ok(result)
        }
        BenchmarkEnvironmentType::Remote => {
            unimplemented!()
        }
    }
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

impl std::fmt::Display for BenchmarkEnvironmentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BenchmarkEnvironmentType::Local => write!(f, "local"),
            BenchmarkEnvironmentType::Remote => write!(f, "remote"),
        }
    }
}
