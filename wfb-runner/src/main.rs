mod benchmark_data;
mod cli;
mod consts;
mod db_config;
mod docker;
mod exec;
mod runner;

use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::{sync::Arc, time::Duration};
use tokio::task::JoinSet;
use wfb_storage::Environment;

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

            let env_config = config
                .get_environment(&env)
                .ok_or_else(|| anyhow::anyhow!("Environment '{}' not found in config", env))?
                .clone();
            let storage = wfb_storage::Storage::new("data")?;

            let mut benchmarks_to_run = Vec::new();
            for b in benchmarks {
                let lang = config
                    .get_lang(&b.language)
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

            let m = MultiProgress::new();
            let unique_dbs = benchmarks_to_run
                .iter()
                .flat_map(|b| b.database)
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();

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
                        app_public_host_url: format!(
                            "http://localhost:{}",
                            consts::APP_PORT_EXTERNAL
                        ),
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
                    let app_config = ssh_config
                        .app
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("SSH Config: app section missing"))?;
                    let db_config = ssh_config
                        .db
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("SSH Config: db section missing"))?;
                    let wrkr_config = ssh_config
                        .wrkr
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("SSH Config: wrkr section missing"))?;

                    let app_executor = exec::ssh::SshExecutor::from_config(app_config);
                    let db_executor = exec::ssh::SshExecutor::from_config(db_config);
                    let wrkr_executor = exec::ssh::SshExecutor::from_config(wrkr_config);
                    let config = runner::RunnerConfig {
                        db_host: db_config.internal_ip.clone(),
                        db_port: consts::DB_PORT_EXTERNAL.to_string(),
                        app_host_url: format!(
                            "http://{}:{}",
                            app_config.internal_ip,
                            consts::APP_PORT_EXTERNAL
                        ),
                        app_public_host_url: format!(
                            "http://{}:{}",
                            app_config.ip,
                            consts::APP_PORT_EXTERNAL
                        ),
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
                start_actions.spawn(async move { runner_clone.deploy_wrkr(&m_clone).await });
            }

            if !skip_db_build {
                let runner_clone = runner.clone();
                let m_clone = m.clone();
                start_actions.spawn(async move {
                    runner_clone
                        .build_database_images(unique_dbs, &m_clone)
                        .await
                });
            }

            while let Some(res) = start_actions.join_next().await {
                res??;
            }

            let pb = m.add(ProgressBar::new(benchmarks_to_run.len() as u64));
            let style = match ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {msg}")
            {
                Ok(style) => style.progress_chars("#>-"),
                Err(_) => ProgressStyle::default_bar().progress_chars("#>-"),
            };
            pb.set_style(style);
            pb.enable_steady_tick(Duration::from_millis(100));
            pb.set_message("Running benchmarks...");

            for b in benchmarks_to_run {
                pb.set_message(format!("{} running", b.name));
                let _ = runner.run_benchmark(&b, &m).await;
                pb.inc(1);
            }
            pb.finish_with_message("Done");
        }
        cli::Commands::Verify {
            env,
            benchmark,
            language,
            testcase,
        } => {
            let mut benchmarks = config
                .get_benchmarks()
                .into_iter()
                .cloned()
                .collect::<Vec<_>>();

            // Filter by benchmark name if specified
            if let Some(ref benchmark_name) = benchmark {
                benchmarks.retain(|b| b.name == *benchmark_name);
                if benchmarks.is_empty() {
                    return Err(anyhow::anyhow!(
                        "No benchmark found with name: {}",
                        benchmark_name
                    ));
                }
            }

            // Filter by language if specified
            if let Some(ref lang) = language {
                benchmarks.retain(|b| b.language == *lang);
                if benchmarks.is_empty() {
                    return Err(anyhow::anyhow!(
                        "No benchmarks found for language: {}",
                        lang
                    ));
                }
            }

            // Filter by test case if specified
            if let Some(ref tc) = testcase {
                let tc = match tc.to_lowercase().as_str() {
                    "plaintext" => wfb_storage::BenchmarkTests::PlainText,
                    "json_aggregate" => wfb_storage::BenchmarkTests::JsonAggregate,
                    "static_files" => wfb_storage::BenchmarkTests::StaticFiles,
                    "db_complex" => wfb_storage::BenchmarkTests::DbComplex,
                    "grpc_aggregate" => wfb_storage::BenchmarkTests::GrpcAggregate,
                    _ => {
                        eprintln!("Unknown testcase: {}", tc);
                        return Ok(());
                    }
                };
                for b in &mut benchmarks {
                    b.tests.retain(|test| *test == tc);
                }
            }

            benchmarks.retain(|b| !b.tests.is_empty());

            let env_config = config
                .get_environment(&env)
                .ok_or_else(|| anyhow::anyhow!("Environment '{}' not found in config", env))?
                .clone();

            let m = MultiProgress::new();
            let unique_dbs = benchmarks
                .iter()
                .flat_map(|b| b.database)
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();

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
                        app_public_host_url: format!(
                            "http://localhost:{}",
                            consts::APP_PORT_EXTERNAL
                        ),
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
                    let app_config = ssh_config
                        .app
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("SSH Config: app section missing"))?;
                    let db_config = ssh_config
                        .db
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("SSH Config: db section missing"))?;
                    let wrkr_config = ssh_config
                        .wrkr
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("SSH Config: wrkr section missing"))?;

                    let app_executor = exec::ssh::SshExecutor::from_config(app_config);
                    let db_executor = exec::ssh::SshExecutor::from_config(db_config);
                    let wrkr_executor = exec::ssh::SshExecutor::from_config(wrkr_config);
                    let config = runner::RunnerConfig {
                        db_host: db_config.internal_ip.clone(),
                        db_port: consts::DB_PORT_EXTERNAL.to_string(),
                        app_host_url: format!(
                            "http://{}:{}",
                            app_config.internal_ip,
                            consts::APP_PORT_EXTERNAL
                        ),
                        app_public_host_url: format!(
                            "http://{}:{}",
                            app_config.ip,
                            consts::APP_PORT_EXTERNAL
                        ),
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
            runner.deploy_wrkr(&m).await?;

            runner.build_database_images(unique_dbs, &m).await?;

            let pb = m.add(ProgressBar::new(benchmarks.len() as u64));
            let style = match ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {msg}")
            {
                Ok(style) => style.progress_chars("#>-"),
                Err(_) => ProgressStyle::default_bar().progress_chars("#>-"),
            };
            pb.set_style(style);
            pb.enable_steady_tick(Duration::from_millis(100));
            pb.set_message("Verifying benchmarks...");

            let mut failed = false;

            for b in &benchmarks {
                pb.set_message(format!("{} verifying", b.name));
                if let Err(e) = runner.verify_benchmark(b, &m).await {
                    pb.println(format!("Benchmark {} verification failed: {}", b.name, e));
                    failed = true;
                }
                pb.inc(1);
            }
            pb.finish_with_message("Done");

            if failed {
                return Err(anyhow::anyhow!("Verification failed for some benchmarks"));
            }
        }
        cli::Commands::Dev { name, env } => {
            let benchmark = config
                .get_benchmarks()
                .iter()
                .find(|b| b.name == name)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Benchmark '{}' not found", name))?;

            let env_config = config
                .get_environment(&env)
                .ok_or_else(|| anyhow::anyhow!("Environment '{}' not found in config", env))?
                .clone();

            let m = MultiProgress::new();
            let storage = wfb_storage::Storage::new("data")?;
            let run_id_clone = "dev".to_string();
            let env_config_clone = env_config.clone();
            let config_clone = config.clone();

            let runner: Arc<dyn runner::BenchmarkRunner> = match env_config {
                Environment::Local(ref _local_config) => {
                    let executor = exec::local::LocalExecutor::new();
                    let config = runner::RunnerConfig {
                        db_host: "host.docker.internal".to_string(),
                        db_port: consts::DB_PORT_EXTERNAL.to_string(),
                        app_host_url: format!("http://localhost:{}", consts::APP_PORT_EXTERNAL),
                        app_public_host_url: format!(
                            "http://localhost:{}",
                            consts::APP_PORT_EXTERNAL
                        ),
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
                    let app_config = ssh_config
                        .app
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("SSH Config: app section missing"))?;
                    let db_config = ssh_config
                        .db
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("SSH Config: db section missing"))?;
                    let wrkr_config = ssh_config
                        .wrkr
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("SSH Config: wrkr section missing"))?;

                    let app_executor = exec::ssh::SshExecutor::from_config(app_config);
                    let db_executor = exec::ssh::SshExecutor::from_config(db_config);
                    let wrkr_executor = exec::ssh::SshExecutor::from_config(wrkr_config);
                    let config = runner::RunnerConfig {
                        db_host: db_config.internal_ip.clone(),
                        db_port: consts::DB_PORT_EXTERNAL.to_string(),
                        app_host_url: format!(
                            "http://{}:{}",
                            app_config.ip,
                            consts::APP_PORT_EXTERNAL
                        ),
                        app_public_host_url: format!(
                            "http://{}:{}",
                            app_config.ip,
                            consts::APP_PORT_EXTERNAL
                        ),
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
            #[allow(clippy::needless_borrow)]
            runner.dev_benchmark(&benchmark, &m).await?;
        }
    }

    Ok(())
}
