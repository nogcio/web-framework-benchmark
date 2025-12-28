mod benchmark_data;
mod cli;
mod exec;
mod verify;
mod run;
mod pipeline;
mod consts;

use std::time::Duration;

use clap::Parser;
use console::style;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use wfb_storage::{DatabaseKind, Environment};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    benchmark_data::ensure_data().await?;
    let args = cli::Args::parse();
    let config = wfb_storage::Config::load(&args.config)?;

    match args.command {
        cli::Commands::Run {
            run_id,
            env,
        } => {          
            let benchmarks = config.get_benchmarks();

            let env_config = config.get_environment(&env)
                .ok_or_else(|| anyhow::anyhow!("Environment '{}' not found in config", env))?.clone();
            
            let m = MultiProgress::new();

            // 1. Build wrkr
            let pb_wrkr = m.add(ProgressBar::new_spinner());
            pb_wrkr.set_style(ProgressStyle::default_spinner().template("{spinner:.blue} {msg}").unwrap());
            pb_wrkr.enable_steady_tick(Duration::from_millis(100));
            pb_wrkr.set_message("Building wrkr...");
            
            let executor = match env_config {
                Environment::Local(_) => exec::local::LocalExecutor::new(),
                other => {
                    anyhow::bail!("Unsupported executor type: {:?}", other);
                }
            };
            
            pipeline::build_wrkr_image(&executor, &pb_wrkr).await?;
            pb_wrkr.set_style(ProgressStyle::default_spinner().template("{msg}").unwrap());
            pb_wrkr.finish_with_message(format!("{} wrkr built", style("✔").green()));

            // 2. Build databases
            let mut unique_dbs = benchmarks.iter()
                .filter_map(|b| b.database)
                .collect::<Vec<_>>();
            unique_dbs.sort();
            unique_dbs.dedup();
            
            pipeline::build_database_images(&env_config, unique_dbs.clone(), &m).await?;

            // 3. Run benchmarks
            let total_duration: u64 = benchmarks.iter()
                .map(|b| b.tests.len() as u64 * consts::BENCHMARK_DURATION_PER_TEST_SECS)
                .sum();
            
            let pb = m.add(ProgressBar::new(total_duration));
            pb.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {msg}")
                .unwrap()
                .progress_chars("#>-"));
            pb.set_message("Running benchmarks...");

            let mut pb_index = unique_dbs.len() + 1; // +1 for wrkr build
            for b in benchmarks {
                pb.set_message(format!("{} running", b.name));
                let executor = match env_config {
                    Environment::Local(_) => exec::local::LocalExecutor::new(),
                    other => {
                        anyhow::bail!("Unsupported executor type: {:?}", other);
                    }
                };
                let _ = run::run_benchmark(executor, b, pb_index, &m, &pb).await;
                pb_index += 1;
            }
            pb.finish_with_message("Done");
        }
        cli::Commands::Verify { env } => {
            if env != "local" {
                anyhow::bail!("Only local environment is supported for verification");
            }

            let benchmarks = config.get_benchmarks();

            let env_config = config.get_environment(&env)
                .ok_or_else(|| anyhow::anyhow!("Environment '{}' not found in config", env))?.clone();
            
            let m = MultiProgress::new();

            let unique_dbs = vec![
                DatabaseKind::Postgres,
                DatabaseKind::Mysql,
                DatabaseKind::Mariadb,
                DatabaseKind::Mongodb,
                DatabaseKind::Mssql,
            ];
            let unique_dbs_count = unique_dbs.len();
            pipeline::build_database_images(&env_config, unique_dbs, &m).await?;

            let pb = m.add(ProgressBar::new(benchmarks.len() as u64));
            pb.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
                .unwrap()
                .progress_chars("#>-"));
            pb.set_message("Verifying benchmarks...");

            let mut pb_index = unique_dbs_count;
            for b in benchmarks {
                pb.set_message(format!("{} verifying", b.name));
                let executor = match env_config {
                    Environment::Local(_) => exec::local::LocalExecutor::new(),
                    other => {
                        anyhow::bail!("Unsupported executor type: {:?}", other);
                    }
                };
                let _ = verify::verify_benchmark(executor,b, pb_index, &m).await;
                pb.inc(1);
                pb_index += 1;
            }
            pb.finish_with_message("Done");
        }
    }

    Ok(())
}
