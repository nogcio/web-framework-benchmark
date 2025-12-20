mod benchmark;
mod cli;
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

use std::env;

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

    let cli = cli::Cli::parse();
    match cli.command {
        cli::Commands::Benchmark { path } => {
            let result = benchmark::run_benchmark(&path).await?;
            dbg!(result);
        }
    }

    Ok(())
}
