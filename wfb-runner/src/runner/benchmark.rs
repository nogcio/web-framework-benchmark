use humanize_bytes::humanize_bytes_binary;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::time::Duration;
use std::vec;
use anyhow::{Context, bail};
use wfb_storage::{Benchmark, BenchmarkTests};
use crate::exec::Executor;
use crate::consts;
use crate::db_config::get_db_config;
use crate::runner::Runner;
use wrkr_api::JsonStats;

impl<E: Executor + Clone + Send + 'static> Runner<E> {
    pub async fn run_app(&self, benchmark: &Benchmark, pb: &ProgressBar) -> anyhow::Result<()> {
        let mut cmd = self.app_docker.run_command(&benchmark.name, &benchmark.name)
            .port(consts::APP_PORT_EXTERNAL, consts::APP_PORT_INTERNAL)
            .ulimit("nofile=1000000:1000000")
            .sysctl("net.core.somaxconn", "65535");

        if let Some(db_kind) = &benchmark.database {
            cmd = cmd
                .env("DB_HOST", &self.config.db_host)
                .env("DB_PORT", &self.config.db_port)
                .env("DB_USER", consts::DB_USER)
                .env("DB_PASSWORD", consts::DB_PASS)
                .env("DB_NAME", consts::DB_NAME)
                .env("DB_KIND", format!("{:?}", db_kind));
        }

        // Add benchmark specific env vars
        for (k, v) in &benchmark.env {
            cmd = cmd.env(k, v);
        }

        self.app_docker.execute_run(cmd, pb).await?;

        Ok(())
    }

    pub async fn wait_for_app_ready(&self, benchmark: &Benchmark, pb: &ProgressBar) -> anyhow::Result<()> {
        pb.set_message(format!("Waiting for App - {}", benchmark.name));
        self.wait_for_container_ready(&self.app_docker, &benchmark.name, pb).await
    }

    pub async fn run_tests(&self, benchmark: &Benchmark, pb: &ProgressBar) -> anyhow::Result<()> {
        for test in &benchmark.tests {
            let script_path = match test {
                BenchmarkTests::PlainText => consts::SCRIPT_PLAINTEXT,
                BenchmarkTests::JsonAggregate => consts::SCRIPT_JSON,
                BenchmarkTests::StaticFiles => consts::SCRIPT_STATIC,
            };

            pb.set_message(format!("Running test {:?} - {}", test, benchmark.name));

            let script_content = std::fs::read_to_string(script_path)
                .with_context(|| format!("Failed to read script file: {}", script_path))?;

            let config = wrkr_core::WrkConfig {
                script_content,
                host_url: self.config.app_public_host_url.clone(),
            };

            pb.set_message(format!("Running test {:?} - {}", test, benchmark.name));
            let stats = wrkr_core::run_once(config.clone()).await?;

            if stats.total_errors > 0 {
                let errors = stats
                    .errors
                    .iter()
                    .map(|(code, count)| format!("{}: {}", code, count))
                    .collect::<Vec<_>>()
                    .join("\n");
                bail!(
                    "Test {:?} failed with {} errors\n{}",
                    test,
                    stats.total_errors,
                    errors
                );
            }
        }
        Ok(())
    }

    pub async fn cleanup(&self, benchmark: &Benchmark, pb: &ProgressBar) -> anyhow::Result<()> {
        self.app_docker.stop_and_remove(&benchmark.name, pb).await;

        // Stop and remove db if exists
        if let Some(db_kind) = &benchmark.database {
            let config = get_db_config(db_kind);
            self.db_docker.stop_and_remove(config.image_name, pb).await;
        }
        Ok(())
    }

    pub async fn verify_benchmark_impl(&self, benchmark: &Benchmark, mb: &MultiProgress) -> anyhow::Result<()> {
        let pb = mb.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.blue} {prefix} {msg}")
                .unwrap(),
        );
        pb.set_prefix(format!("[{}]", benchmark.name));
        pb.enable_steady_tick(Duration::from_millis(100));
        pb.set_message(format!("Verifying {}", benchmark.name));
    
        let result = async {
            self.build_benchmark_image(benchmark, &pb).await?;
            if let Some(db_kind) = &benchmark.database {
                self.setup_database(db_kind, &pb).await?;
                self.wait_for_db_ready(db_kind, &pb).await?;
            }
            self.run_app(benchmark, &pb).await?;
            self.wait_for_app_ready(benchmark, &pb).await?;
            self.run_tests(benchmark, &pb).await?;
    
            Ok::<(), anyhow::Error>(())
        }
        .await;
    
        self.cleanup(benchmark, &pb)
            .await
            .ok();
    
        pb.set_style(ProgressStyle::default_spinner().template("{msg}").unwrap());
    
        match result {
            Ok(_) => {
                pb.finish_with_message(format!(
                    "{} {} Verified",
                    console::style("✔").green(),
                    benchmark.name
                ));
                Ok(())
            }
            Err(e) => {
                pb.finish_with_message(format!(
                    "{} {} Failed: {}",
                    console::style("✘").red(),
                    benchmark.name,
                    e
                ));
                Err(e)
            }
        }
    }

    pub async fn run_benchmark_impl(&self, benchmark: &Benchmark, mb: &MultiProgress) -> anyhow::Result<()> {
        let pb = mb.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.blue} {prefix} {msg}")
                .unwrap(),
        );
        pb.set_prefix(format!("[{}]", benchmark.name));
        pb.enable_steady_tick(Duration::from_millis(100));
        pb.set_message(format!("Running {}", benchmark.name));
    
        let result = async {
            // Build and deploy
            self.build_benchmark_image(benchmark, &pb).await?;

            if let Some(db_kind) = &benchmark.database {
                self.setup_database(db_kind, &pb).await?;
                self.wait_for_db_ready(db_kind, &pb).await?;
            }
            self.run_app(benchmark, &pb).await?;
            self.wait_for_app_ready(benchmark, &pb).await?;
            
            // Verify via public IP
            self.run_tests(benchmark, &pb).await?;

            // Cleanup before running actual benchmarks to ensure clean state for each test
            self.cleanup(benchmark, &pb).await?;

            // Run tests via wrkr in docker
            self.run_tests_docker(benchmark, mb).await?;
    
            Ok::<(), anyhow::Error>(())
        }
        .await;
        
        self.cleanup(benchmark, &pb)
            .await
            .ok();
    
        pb.set_style(ProgressStyle::default_spinner().template("{msg}").unwrap());
    
        match result {
            Ok(_) => {
                pb.finish_with_message(format!(
                    "{} {} Finished",
                    console::style("✔").green(),
                    benchmark.name
                ));
                Ok(())
            }
            Err(e) => {
                pb.finish_with_message(format!(
                    "{} {} Failed: {}",
                    console::style("✘").red(),
                    benchmark.name,
                    e
                ));
                Err(e)
            }
        }
    }

    async fn run_tests_docker(&self, benchmark: &Benchmark, mb: &MultiProgress) -> anyhow::Result<()> {
        let lang = self.wfb_config.get_lang(&benchmark.language)
            .ok_or_else(|| anyhow::anyhow!("Language '{}' not found", benchmark.language))?;

        for test in &benchmark.tests {
            let pb = mb.add(ProgressBar::new_spinner());
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.blue} {prefix} [{bar:40.cyan/blue}] {msg}")
                    .unwrap()
                    .progress_chars("#>-"),
            );
            pb.set_prefix(format!("[{}/{}]", benchmark.name, test));
            pb.enable_steady_tick(Duration::from_millis(100));
            pb.set_length(consts::BENCHMARK_DURATION_PER_TEST_SECS);
            pb.set_position(0);

            if let Some(db_kind) = &benchmark.database {
                self.setup_database(db_kind, &pb).await?;
                self.wait_for_db_ready(db_kind, &pb).await?;
            }
            self.run_app(benchmark, &pb).await?;
            self.wait_for_app_ready(benchmark, &pb).await?;

            let run_pb = mb.add(ProgressBar::new(100));
            run_pb.set_style(
                ProgressStyle::default_bar()
                    .template("{msg}")
                    .unwrap(),
            );
            
            let script_path = match test {
                BenchmarkTests::PlainText => consts::SCRIPT_PLAINTEXT,
                BenchmarkTests::JsonAggregate => consts::SCRIPT_JSON,
                BenchmarkTests::StaticFiles => consts::SCRIPT_STATIC,
            };
            
            let duration = format!("{}", consts::BENCHMARK_DURATION_PER_TEST_SECS);
            let ramp_up = format!("{}", consts::BENCHMARK_RAMP_UP_SECS);
            let connections = consts::BENCHMARK_CONNECTIONS.to_string();

            let memory_usage = std::sync::Arc::new(std::sync::Mutex::new(0u64));
            let memory_usage_clone = memory_usage.clone();
            let app_docker = self.app_docker.clone();
            let container_name = benchmark.name.clone();
            
            let monitor_handle = tokio::spawn(async move {
                loop {
                    if let Ok(stats) = app_docker.stats(&container_name, "{{.MemUsage}}").await {
                        // stats output might be "10MiB / 1GiB"
                        let usage_str = stats.trim().split('/').next().unwrap_or("0B").trim();
                        let bytes = parse_docker_memory(usage_str);
                        if let Ok(mut guard) = memory_usage_clone.lock() {
                            *guard = (*guard).max(bytes);
                        }
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            });

            let cmd = self.wrkr_docker.run_command(consts::WRKR_IMAGE, "wrkr-runner")
                .detach(false)
                .network("host")
                .ulimit("nofile=1000000:1000000")
                .arg("-s")
                .arg(script_path)
                .arg("--url")
                .arg(&self.config.app_host_url)
                .arg("--duration")
                .arg(&duration)
                .arg("--ramp-up")
                .arg(&ramp_up)
                .arg("--connections")
                .arg(&connections)
                .arg("--output")
                .arg("json");
            
            let raw_data_collection = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            let raw_data_collection_clone = raw_data_collection.clone();

            let pb_clone = pb.clone();
            let memory_usage_read = memory_usage.clone();
            let output = self.wrkr_docker.execute_run_with_std_out(cmd, move |line| {
                if let Ok(stats) = serde_json::from_str::<JsonStats>(line) {
                    let mem_bytes = if let Ok(guard) = memory_usage_read.lock() {
                        *guard
                    } else {
                        0
                    };

                    let raw_item = wfb_storage::TestCaseRaw {
                        elapsed_secs: stats.elapsed_secs,
                        connections: stats.connections,
                        requests_per_sec: stats.requests_per_sec,
                        bytes_per_sec: stats.bytes_per_sec,
                        total_requests: stats.total_requests,
                        total_bytes: stats.total_bytes,
                        total_errors: stats.total_errors,
                        latency_mean: stats.latency_mean,
                        latency_stdev: stats.latency_stdev,
                        latency_max: stats.latency_max,
                        latency_p50: stats.latency_p50,
                        latency_p90: stats.latency_p90,
                        latency_p99: stats.latency_p99,
                        errors: stats.errors.clone(),
                        memory_usage_bytes: mem_bytes,
                    };

                    if let Ok(mut guard) = raw_data_collection_clone.lock() {
                        guard.push(raw_item);
                    }

                    pb_clone.set_position(stats.elapsed_secs.min(consts::BENCHMARK_DURATION_PER_TEST_SECS));
                    pb_clone.set_message(format!(
                        "[{}] RPS: {:.0} | TPS: {} | Latency: {} | Errors: {} | Mem: {}",
                        stats.connections,
                        stats.requests_per_sec,
                        humanize_bytes_binary!(stats.bytes_per_sec),
                        format_latency(stats.latency_p99),
                        stats.total_errors,
                        humanize_bytes_binary!(mem_bytes)
                    ));
                }
            }, &run_pb).await?;
            
            monitor_handle.abort();

            let raw_data = raw_data_collection.lock().unwrap().clone();

            let final_memory_usage = *memory_usage.lock().unwrap();

            if let Some(last_stat) = raw_data.last().cloned() {
                 let summary = wfb_storage::TestCaseSummary {
                    requests_per_sec: last_stat.requests_per_sec,
                    bytes_per_sec: last_stat.bytes_per_sec,
                    total_requests: last_stat.total_requests,
                    total_bytes: last_stat.total_bytes,
                    total_errors: last_stat.total_errors,
                    latency_mean: last_stat.latency_mean,
                    latency_stdev: last_stat.latency_stdev,
                    latency_max: last_stat.latency_max,
                    latency_p50: last_stat.latency_p50,
                    latency_p90: last_stat.latency_p90,
                    latency_p99: last_stat.latency_p99,
                    errors: last_stat.errors,
                    memory_usage_bytes: final_memory_usage,
                };
                
                let manifest = wfb_storage::BenchmarkManifest {
                    language_version: benchmark.language_version.clone(),
                    framework_version: benchmark.framework_version.clone(),
                    tags: benchmark.tags.clone(),
                };

                self.storage.save_benchmark_result(
                    &self.run_id,
                    &self.environment,
                    lang,
                    benchmark,
                    *test,
                    &manifest,
                    &summary,
                    &raw_data,
                )?;
            }
            
            self.wrkr_docker.stop_and_remove("wrkr-runner", &run_pb).await;

            self.cleanup(benchmark, &pb).await?;

            run_pb.finish_and_clear();
            mb.remove(&run_pb);

            // Parse output to find max RPS
            let mut stats_vec = vec![];

            for line in output.lines() {
                if let Ok(stats) = serde_json::from_str::<JsonStats>(line) {
                    stats_vec.push(stats);
                }
            }
            stats_vec.sort_by(|a, b| a.requests_per_sec.partial_cmp(&b.requests_per_sec).unwrap());

            pb.set_style(ProgressStyle::default_spinner().template("{msg}").unwrap());

            if let Some(max_stats) = stats_vec.last() {
                pb.finish_with_message(format!(
                "   {} {:?} - Connections: [{}] | RPS: {:.0} | TPS: {} | Latency: {} | Errors: {}",
                console::style("✔").green(),
                test,
                max_stats.connections,
                max_stats.requests_per_sec,
                humanize_bytes_binary!(max_stats.bytes_per_sec),
                format_latency(max_stats.latency_p99),
                max_stats.total_errors
            ));
            } else {
                pb.finish_with_message(format!(
                    "   {} {:?} - No stats collected",
                    console::style("✘").red(),
                    test
                ));
            }
        }
        Ok(())
    }
}

fn format_latency(micros: u64) -> String {
    if micros >= 1_000_000 {
        format!("{:.2}s", micros as f64 / 1_000_000.0)
    } else if micros >= 1_000 {
        format!("{:.2}ms", micros as f64 / 1_000.0)
    } else {
        format!("{:.2}us", micros)
    }
}

fn parse_docker_memory(s: &str) -> u64 {
    let s = s.trim();
    let (num_str, multiplier) = if s.ends_with("GiB") {
        (s.trim_end_matches("GiB"), 1024.0 * 1024.0 * 1024.0)
    } else if s.ends_with("MiB") {
        (s.trim_end_matches("MiB"), 1024.0 * 1024.0)
    } else if s.ends_with("KiB") {
        (s.trim_end_matches("KiB"), 1024.0)
    } else if s.ends_with("B") {
        (s.trim_end_matches("B"), 1.0)
    } else {
        return 0;
    };
    
    if let Ok(num) = num_str.trim().parse::<f64>() {
        (num * multiplier) as u64
    } else {
        0
    }
}