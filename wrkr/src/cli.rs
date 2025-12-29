use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Target URL
    #[arg(short, long)]
    pub url: String,

    /// Path to Lua script
    #[arg(short, long)]
    pub script: PathBuf,
    
    /// Number of connections (VUs) to start with
    #[arg(long, default_value_t = 0)]
    pub start_connections: u64,

    /// Target number of connections (VUs) to end with
    #[arg(short, long, default_value_t = 10)]
    pub connections: u64,

    /// Time to reach target connections in seconds
    #[arg(long)]
    pub ramp_up: Option<u64>,

    /// Comma-separated list of connections (VUs) for step load (e.g. "32,64,128,256")
    #[arg(long)]
    pub step_connections: Option<String>,

    /// Duration to hold each step in seconds
    #[arg(long)]
    pub step_duration: Option<u64>,

    /// Duration of the test in seconds
    #[arg(short, long, default_value_t = 10)]
    pub duration: u64,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Text)]
    pub output: OutputFormat,
}