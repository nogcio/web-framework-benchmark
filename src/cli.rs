use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "wfb")]
#[command(about = "Web framework benchmark tool", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum BenchmarkEnvironmentType {
    Local,
    Remote,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(arg_required_else_help = true)]
    Benchmark {
        path: PathBuf,
        #[arg(short, long, default_value = "local")]
        environment: BenchmarkEnvironmentType,
    },
}
