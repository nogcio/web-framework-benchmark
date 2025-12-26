use clap::{Parser, Subcommand};

use crate::analysis_context::AnalysisLanguage;

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum StaticFilesMode {
    /// Recommended: run static-files tests on a fixed connection matrix (clean methodology)
    Fixed,
    /// Legacy: adaptive connection search with a dynamic p99 budget
    Adaptive,
}

#[derive(Debug, Parser)]
#[command(name = "wfb")]
#[command(about = "Web framework benchmark tool", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Run {
        id: u32,
        #[arg(short, long, default_value = "local")]
        environment: String,
        #[arg(short, long)]
        filter: Option<String>,
        #[arg(long, default_value_t = false)]
        verification: bool,

        /// How to run static-files benchmarks (static_files_small/medium/large)
        #[arg(long, value_enum, default_value_t = StaticFilesMode::Fixed)]
        static_files_mode: StaticFilesMode,
    },

    Serve {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },

    Analyze {
        #[arg(short, long)]
        run_id: Option<u32>,
        #[arg(long, env = "OPENAI_API_KEY")]
        api_key: String,
        #[arg(long, default_value = "gpt-4o")]
        model: String,
        #[arg(long, default_value = "https://api.openai.com/v1/chat/completions")]
        api_url: String,
        #[arg(long, value_enum, default_values_t = vec![AnalysisLanguage::En])]
        languages: Vec<AnalysisLanguage>,
    },
}
