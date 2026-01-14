use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to config directory
    #[arg(short, long, default_value = "./config")]
    pub config: PathBuf,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Run {
        /// Run ID
        run_id: String,

        /// Environment to use
        #[arg(short, long, default_value = "local")]
        env: String,

        /// Skip building and deploying wrkr
        #[arg(long, default_value_t = false)]
        skip_wrkr_build: bool,

        /// Skip building and deploying dbs
        #[arg(long, default_value_t = false)]
        skip_db_build: bool,
    },
    Verify {
        /// Environment to use
        #[arg(short, long, default_value = "local")]
        env: String,

        /// Filter by specific benchmark name
        #[arg(short, long)]
        benchmark: Option<String>,

        /// Filter by specific language
        #[arg(short, long)]
        language: Option<String>,

        /// Filter by specific test case (plaintext, json_aggregate, static_files)
        #[arg(short, long)]
        testcase: Option<String>,
    },
    Dev {
        /// Benchmark to run
        name: String,

        /// Environment to use
        #[arg(short, long, default_value = "local")]
        env: String,
    },
}
