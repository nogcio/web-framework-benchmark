use clap::{Parser, Subcommand};

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
}
