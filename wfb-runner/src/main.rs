mod benchmark_data;
mod cli;
mod exec;
mod db_config;
mod consts;
mod runner;
mod docker;

use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::task::JoinSet;
use wfb_storage::{DatabaseKind, Environment};
use std::{sync::Arc, time::Duration};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    benchmark_data::ensure_data().await?;
    let args = cli::Args::parse();
    let config = wfb_storage::Config::load(&args.config)?;

    match args.command {
        cli::Commands::Run {
            run_id,
            env,
            skip_wrkr_build,
            skip_db_build,
        } => {          
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

            let storage = wfb_storage::Storage::new("data")?;
            let run_id_clone = run_id.clone();
            let env_config_clone = env_config.clone();
            let config_clone = config.clone();

            let runner: Arc<dyn runner::BenchmarkRunner> = match env_config {
                Environment::Local(ref _local_config) => {
                    let executor = exec::local::LocalExecutor::new();
                    let config = runner::RunnerConfig {
                        db_host: "host.docker.internal".to_string(),
                        db_port: consts::DB_PORT_EXTERNAL.to_string(),
                        app_host_url: format!("http://localhost:{}", consts::APP_PORT_EXTERNAL),
                        app_public_host_url: format!("http://localhost:{}", consts::APP_PORT_EXTERNAL),
                        is_remote: false,
                    };
                    Arc::new(runner::Runner::new(
                        executor.clone(), 
                        executor.clone(), 
                        executor, 
                        false, 
                        config,
                        storage.clone(),
                        run_id_clone,
                        env_config_clone,
                        config_clone,
                    ))
                }
                Environment::Ssh(ref ssh_config) => {
                    let app_executor = exec::ssh::SshExecutor::from_config(&ssh_config.app);
                    let db_executor = exec::ssh::SshExecutor::from_config(&ssh_config.db);
                    let wrkr_executor = exec::ssh::SshExecutor::from_config(&ssh_config.wrkr);
                    let config = runner::RunnerConfig {
                        db_host: ssh_config.db.internal_ip.clone(),
                        db_port: consts::DB_PORT_EXTERNAL.to_string(),
                        app_host_url: format!("http://{}:{}", ssh_config.app.internal_ip, consts::APP_PORT_EXTERNAL),
                        app_public_host_url: format!("http://{}:{}", ssh_config.app.ip, consts::APP_PORT_EXTERNAL),
                        is_remote: true,
                    };
                    Arc::new(runner::Runner::new(
                        app_executor, 
                        db_executor, 
                        wrkr_executor, 
                        true, 
                        config,
                        storage.clone(),
                        run_id_clone,
                        env_config_clone,
                        config_clone,
                    ))
                }
            };

            runner.prepare(&m).await?;

            let mut start_actions = JoinSet::new();
            if !skip_wrkr_build {
                let runner_clone = runner.clone();
                let m_clone = m.clone();
                start_actions.spawn(async move {
                    runner_clone.deploy_wrkr(&m_clone).await
                });
            }

            if !skip_db_build {
                let runner_clone = runner.clone();
                let m_clone = m.clone();
                start_actions.spawn(async move {
                    runner_clone.build_database_images(unique_dbs, &m_clone).await
                });
            }

            while let Some(res) = start_actions.join_next().await {
                res??;
            }

            let mut benchmarks_to_run = Vec::new();
            for b in benchmarks {
                let lang = config.get_lang(&b.language)
                    .ok_or_else(|| anyhow::anyhow!("Language '{}' not found", b.language))?;
                
                let mut missing_tests = Vec::new();
                for test in &b.tests {
                    if !storage.has_test_result(&run_id, &env_config, lang, b, *test) {
                        missing_tests.push(*test);
                    }
                }

                if !missing_tests.is_empty() {
                    let mut b_clone = b.clone();
                    b_clone.tests = missing_tests;
                    benchmarks_to_run.push(b_clone);
                }
            }
            
            let pb = m.add(ProgressBar::new(benchmarks_to_run.len() as u64));
            pb.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {msg}")
                .unwrap()
                .progress_chars("#>-"));
            pb.enable_steady_tick(Duration::from_millis(100));
            pb.set_message("Running benchmarks...");

            for b in benchmarks_to_run {
                pb.set_message(format!("{} running", b.name));
                let _ = runner.run_benchmark(&b, &m).await;
                pb.inc(1);
            }
            pb.finish_with_message("Done");
        }
        cli::Commands::Verify { env } => {
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

            let storage = wfb_storage::Storage::new("data")?;
            let run_id_clone = "verify".to_string();
            let env_config_clone = env_config.clone();
            let config_clone = config.clone();

            let runner: Arc<dyn runner::BenchmarkRunner> = match env_config {
                Environment::Local(ref _local_config) => {
                    let executor = exec::local::LocalExecutor::new();
                    let config = runner::RunnerConfig {
                        db_host: "host.docker.internal".to_string(),
                        db_port: consts::DB_PORT_EXTERNAL.to_string(),
                        app_host_url: format!("http://localhost:{}", consts::APP_PORT_EXTERNAL),
                        app_public_host_url: format!("http://localhost:{}", consts::APP_PORT_EXTERNAL),
                        is_remote: false,
                    };
                    Arc::new(runner::Runner::new(
                        executor.clone(), 
                        executor.clone(), 
                        executor, 
                        false, 
                        config,
                        storage.clone(),
                        run_id_clone,
                        env_config_clone,
                        config_clone,
                    ))
                }
                Environment::Ssh(ref ssh_config) => {
                    let app_executor = exec::ssh::SshExecutor::from_config(&ssh_config.app);
                    let db_executor = exec::ssh::SshExecutor::from_config(&ssh_config.db);
                    let wrkr_executor = exec::ssh::SshExecutor::from_config(&ssh_config.wrkr);
                    let config = runner::RunnerConfig {
                        db_host: ssh_config.db.internal_ip.clone(),
                        db_port: consts::DB_PORT_EXTERNAL.to_string(),
                        app_host_url: format!("http://{}:{}", ssh_config.app.ip, consts::APP_PORT_EXTERNAL),
                        app_public_host_url: format!("http://{}:{}", ssh_config.app.ip, consts::APP_PORT_EXTERNAL),
                        is_remote: true,
                    };
                    Arc::new(runner::Runner::new(
                        app_executor, 
                        db_executor, 
                        wrkr_executor, 
                        true, 
                        config,
                        storage,
                        run_id_clone,
                        env_config_clone,
                        config_clone,
                    ))
                }
            };

            runner.prepare(&m).await?;
            
            runner.build_database_images(unique_dbs, &m).await?;
            
            let pb = m.add(ProgressBar::new(benchmarks.len() as u64));
            pb.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {msg}")
                .unwrap()
                .progress_chars("#>-"));
            pb.enable_steady_tick(Duration::from_millis(100));
            pb.set_message("Verifying benchmarks...");

            for b in benchmarks {
                pb.set_message(format!("{} verifying", b.name));
                let _ = runner.verify_benchmark(b, &m).await;
                pb.inc(1);
            }
            pb.finish_with_message("Done");
        }
    }

    Ok(())
}
