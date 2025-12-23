use clap::{Parser, Subcommand};

use crate::analysis_context::AnalysisLanguage;

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
