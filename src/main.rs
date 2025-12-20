mod benchmark;
mod cli;
mod docker;
mod error;
mod exec_utils;
mod http_probe;
mod parsers;
mod wrk;
mod benchmark_environment;

pub mod prelude {
    pub use crate::error::*;
    pub use crate::exec_utils::*;
    pub use tracing::{debug, error, info, span, trace, warn};
}

use std::env;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::io::{self, Write, BufWriter};
use rand::rngs::OsRng;
use rand::RngCore;

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
        cli::Commands::Benchmark { path, environment } => {
            match environment {
                cli::BenchmarkEnvironmentType::Local => {
                    let settings = crate::benchmark_environment::local::LocalConfig::from_file("config/environment.local.yaml")?;
                    let mut env = crate::benchmark_environment::local::LocalBenchmarkEnvironment::new(settings);
                    let result = benchmark::run_benchmark(&mut env, &path).await?;
                    info!("Benchmark completed: {:?}", result);
                }
                cli::BenchmarkEnvironmentType::Remote => {
                    unimplemented!()
                }
            }
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
        let f = OpenOptions::new().write(true).create(true).truncate(true).open(&path)?;
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
