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
    },
    Verify {
        /// Environment to use
        env: String,
    },
}