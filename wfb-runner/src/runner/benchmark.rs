use crate::consts;
use crate::db_config::get_db_config;
use crate::exec::Executor;
use crate::runner::Runner;
use anyhow::{Context, bail};
use humanize_bytes::humanize_bytes_binary;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::time::Duration;
use wfb_storage::{Benchmark, BenchmarkTests, DatabaseKind};
use wrkr_api::JsonStats;

impl<E: Executor + Clone + Send + 'static> Runner<E> {
    pub async fn run_app(&self, benchmark: &Benchmark, pb: &ProgressBar) -> anyhow::Result<()> {
        let mut cmd = self
            .app_docker
            .run_command(&benchmark.name, &benchmark.name)
            .port(consts::APP_PORT_EXTERNAL, consts::APP_PORT_INTERNAL)
            .ulimit("nofile=1000000:1000000")
            .add_host("host.docker.internal:host-gateway");

        if let Some(db_kind) = &benchmark.database {
            let db_pass = if matches!(db_kind, DatabaseKind::Mssql) {
                "Benchmark!12345"
            } else {
                consts::DB_PASS
            };

            cmd = cmd
                .env("DB_HOST", &self.config.db_host)
                .env("DB_PORT", &self.config.db_port)
                .env("DB_USER", consts::DB_USER)
                .env("DB_PASSWORD", db_pass)
                .env("DB_NAME", consts::DB_NAME)
                .env("DB_KIND", format!("{}", db_kind))
                .env("PORT", "8080")
                .env("DATA_DIR", "benchmarks_data")
                .env("DB_POOL_SIZE", "256");
        }

        // Add benchmark specific env vars
        for (k, v) in &benchmark.env {
            cmd = cmd.env(k, v);
        }

        self.app_docker.execute_run(cmd, pb).await?;

        Ok(())
    }

    pub async fn wait_for_app_ready(
        &self,
        benchmark: &Benchmark,
        pb: &ProgressBar,
    ) -> anyhow::Result<()> {
        pb.set_message(format!("Waiting for App - {}", benchmark.name));
        self.wait_for_container_ready(&self.app_docker, &benchmark.name, pb)
            .await
    }

    pub async fn run_tests(&self, benchmark: &Benchmark, pb: &ProgressBar) -> anyhow::Result<()> {
        for test in &benchmark.tests {
            let script_path = match test {
                BenchmarkTests::PlainText => consts::SCRIPT_PLAINTEXT,
                BenchmarkTests::JsonAggregate => consts::SCRIPT_JSON,
                BenchmarkTests::StaticFiles => consts::SCRIPT_STATIC,
                BenchmarkTests::DbComplex => consts::SCRIPT_DB_COMPLEX,
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

    pub async fn verify_benchmark_impl(
        &self,
        benchmark: &Benchmark,
        mb: &MultiProgress,
    ) -> anyhow::Result<()> {
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

        let logs = if result.is_err() {
            self.app_docker.logs(&benchmark.name).await.ok()
        } else {
            None
        };

        self.cleanup(benchmark, &pb).await.ok();

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
                if let Some(l) = logs {
                    println!("Application Logs:\n{}", l);
                }
                Err(e)
            }
        }
    }

    pub async fn run_benchmark_impl(
        &self,
        benchmark: &Benchmark,
        mb: &MultiProgress,
    ) -> anyhow::Result<()> {
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

            pb.set_message("benchmarks running...");

            // Run tests via wrkr in docker
            self.run_tests_docker(benchmark, mb).await?;

            Ok::<(), anyhow::Error>(())
        }
        .await;

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

    pub async fn dev_benchmark_impl(
        &self,
        benchmark: &Benchmark,
        mb: &MultiProgress,
    ) -> anyhow::Result<()> {
        let pb = mb.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.blue} {prefix} {msg}")
                .unwrap(),
        );
        pb.set_prefix(format!("[{}]", benchmark.name));
        pb.enable_steady_tick(Duration::from_millis(100));
        pb.set_message(format!("Starting {} in dev mode", benchmark.name));

        // Build and deploy
        self.build_benchmark_image(benchmark, &pb).await?;

        if let Some(db_kind) = &benchmark.database {
            self.build_database_image(db_kind, &pb).await?;
            self.setup_database(db_kind, &pb).await?;
            self.wait_for_db_ready(db_kind, &pb).await?;
        }
        self.run_app(benchmark, &pb).await?;
        self.wait_for_app_ready(benchmark, &pb).await?;

        pb.set_style(ProgressStyle::default_spinner().template("{msg}").unwrap());
        pb.finish_with_message(format!(
            "{} {} Started. URL: {}",
            console::style("✔").green(),
            benchmark.name,
            self.config.app_public_host_url
        ));

        // Spawn log tailing tasks
        let app_name = benchmark.name.to_string();
        let runner_clone = self.clone();
        tokio::spawn(async move {
            let _ = runner_clone
                .app_docker
                .logs_follow(&app_name, move |line| {
                    println!("{} {}", console::style("[APP]").green(), line);
                })
                .await;
        });

        println!("Press Ctrl+C to stop...");

        // Wait forever (until signal)
        tokio::signal::ctrl_c().await?;

        println!("Stopping...");
        self.cleanup(benchmark, &ProgressBar::hidden()).await?;

        Ok(())
    }

    async fn run_tests_docker(
        &self,
        benchmark: &Benchmark,
        mb: &MultiProgress,
    ) -> anyhow::Result<()> {
        let lang = self
            .wfb_config
            .get_lang(&benchmark.language)
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

            let (script_path, step_connections) = match test {
                BenchmarkTests::PlainText => (
                    consts::SCRIPT_PLAINTEXT,
                    consts::BENCHMARK_STEP_CONNECTIONS_PLAINTEXT,
                ),
                BenchmarkTests::JsonAggregate => {
                    (consts::SCRIPT_JSON, consts::BENCHMARK_STEP_CONNECTIONS_JSON)
                }
                BenchmarkTests::StaticFiles => (
                    consts::SCRIPT_STATIC,
                    consts::BENCHMARK_STEP_CONNECTIONS_STATIC,
                ),
                BenchmarkTests::DbComplex => (
                    consts::SCRIPT_DB_COMPLEX,
                    consts::BENCHMARK_STEP_CONNECTIONS_DB_COMPLEX,
                ),
            };

            // --- WARMUP PHASE ---
            {
                let warmup_pb = mb.add(ProgressBar::new(consts::BENCHMARK_WARMUP_DURATION_SECS));
                warmup_pb.set_style(
                    ProgressStyle::default_bar()
                        .template("{spinner:.yellow} {prefix:.yellow} [{bar:40.yellow/white}] {msg:.yellow}")
                        .unwrap()
                        .progress_chars("=>-"),
                );
                warmup_pb.set_prefix(format!("[{}/{}/warmup]", benchmark.name, test));
                warmup_pb.enable_steady_tick(Duration::from_millis(100));

                let warmup_duration = format!("{}", consts::BENCHMARK_WARMUP_DURATION_SECS);

                let cmd = self
                    .wrkr_docker
                    .run_command(consts::WRKR_IMAGE, "wrkr-warmup")
                    .detach(false)
                    .ulimit("nofile=1000000:1000000")
                    .arg("-s")
                    .arg(script_path)
                    .arg("--url")
                    .arg(&self.config.app_host_url)
                    .arg("--duration")
                    .arg(&warmup_duration)
                    .arg("--connections")
                    .arg("8")
                    .arg("--output")
                    .arg("json");

                let warmup_pb_clone = warmup_pb.clone();
                let _ = self
                    .wrkr_docker
                    .execute_run_with_std_out(
                        cmd,
                        move |line| {
                            if let Ok(stats) = serde_json::from_str::<JsonStats>(line) {
                                warmup_pb_clone.set_position(
                                    stats
                                        .elapsed_secs
                                        .min(consts::BENCHMARK_WARMUP_DURATION_SECS),
                                );
                                warmup_pb_clone.set_message(format!(
                                    "RPS: {:.0} | Latency: {} | Errors: {}",
                                    stats.requests_per_sec,
                                    format_latency(stats.latency_p99),
                                    stats.total_errors
                                ));
                            }
                        },
                        &ProgressBar::hidden(),
                    )
                    .await;
                self.wrkr_docker
                    .stop_and_remove("wrkr-warmup", &ProgressBar::hidden())
                    .await;
                warmup_pb.finish_and_clear();
            }
            // --- END WARMUP PHASE ---

            let run_pb = mb.add(ProgressBar::new(100));
            run_pb.set_style(ProgressStyle::default_bar().template("{msg}").unwrap());

            let duration = format!("{}", consts::BENCHMARK_DURATION_PER_TEST_SECS);
            let step_duration = format!("{}", consts::BENCHMARK_STEP_DURATION_SECS);

            let resource_usage = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            let resource_usage_clone = resource_usage.clone();
            let app_docker = self.app_docker.clone();
            let container_name = benchmark.name.clone();

            let monitor_handle = tokio::spawn(async move {
                loop {
                    if let Ok(stats) = app_docker
                        .stats(&container_name, "{{.MemUsage}}::{{.CPUPerc}}")
                        .await
                    {
                        // stats output might be "10MiB / 1GiB::0.05%"
                        let parts: Vec<&str> = stats.split("::").collect();
                        if parts.len() == 2 {
                            let mem_str = parts[0].trim().split('/').next().unwrap_or("0B").trim();
                            let cpu_str = parts[1].trim();

                            let bytes = parse_docker_memory(mem_str);
                            let cpu = parse_docker_cpu(cpu_str);

                            if let Ok(mut guard) = resource_usage_clone.lock() {
                                guard.push((bytes, cpu));
                            }
                        }
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            });

            let cmd = self
                .wrkr_docker
                .run_command(consts::WRKR_IMAGE, "wrkr-runner")
                .detach(false)
                .ulimit("nofile=1000000:1000000")
                .arg("-s")
                .arg(script_path)
                .arg("--url")
                .arg(&self.config.app_host_url)
                .arg("--duration")
                .arg(&duration)
                .arg("--step-connections")
                .arg(step_connections)
                .arg("--step-duration")
                .arg(&step_duration)
                .arg("--output")
                .arg("json");

            let raw_data_collection = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            let raw_data_collection_clone = raw_data_collection.clone();

            let pb_clone = pb.clone();
            let resource_usage_read = resource_usage.clone();
            let _output = self.wrkr_docker.execute_run_with_std_out(cmd, move |line| {
                if let Ok(stats) = serde_json::from_str::<JsonStats>(line) {
                    let (mem_bytes, cpu_usage) = if let Ok(guard) = resource_usage_read.lock() {
                        guard.last().cloned().unwrap_or((0, 0.0))
                    } else {
                        (0, 0.0)
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
                        latency_p75: stats.latency_p75,
                        latency_p90: stats.latency_p90,
                        latency_p99: stats.latency_p99,
                        latency_stdev_pct: stats.latency_stdev_pct,
                        latency_distribution: stats.latency_distribution.clone(),
                        errors: stats.errors.clone(),
                        memory_usage_bytes: mem_bytes,
                        cpu_usage_percent: cpu_usage,
                        req_per_sec_avg: stats.req_per_sec_avg,
                        req_per_sec_stdev: stats.req_per_sec_stdev,
                        req_per_sec_max: stats.req_per_sec_max,
                        req_per_sec_stdev_pct: stats.req_per_sec_stdev_pct,
                    };

                    if let Ok(mut guard) = raw_data_collection_clone.lock() {
                        guard.push(raw_item);
                    }

                    pb_clone.set_position(stats.elapsed_secs.min(consts::BENCHMARK_DURATION_PER_TEST_SECS));
                    pb_clone.set_message(format!(
                        "[{}] RPS: {:.0} | TPS: {} | Latency: {} | Errors: {} | Mem: {} | CPU: {:.2}%",
                        stats.connections,
                        stats.requests_per_sec,
                        humanize_bytes_binary!(stats.bytes_per_sec),
                        format_latency(stats.latency_p99),
                        stats.total_errors,
                        humanize_bytes_binary!(mem_bytes),
                        cpu_usage
                    ));
                }
            }, &run_pb).await?;

            monitor_handle.abort();

            let raw_data = raw_data_collection.lock().unwrap().clone();

            let final_resource_usage = resource_usage
                .lock()
                .unwrap()
                .iter()
                .max_by(|a, b| a.0.cmp(&b.0))
                .cloned()
                .unwrap_or((0, 0.0));
            let final_memory_usage = final_resource_usage.0;
            let final_cpu_usage = final_resource_usage.1;

            let summary =
                find_max_stable_performance(&raw_data, final_memory_usage, final_cpu_usage);

            if let Some(summary) = &summary {
                let manifest = wfb_storage::BenchmarkManifest {
                    language_version: benchmark.language_version.clone(),
                    framework_version: benchmark.framework_version.clone(),
                    tags: benchmark.tags.clone(),
                    database: benchmark.database,
                    path: benchmark.path.clone(),
                };

                self.storage.save_benchmark_result(
                    &self.run_id,
                    &self.environment,
                    lang,
                    benchmark,
                    *test,
                    &manifest,
                    summary,
                    &raw_data,
                )?;
            }

            self.wrkr_docker
                .stop_and_remove("wrkr-runner", &run_pb)
                .await;

            self.cleanup(benchmark, &pb).await?;

            run_pb.finish_and_clear();
            mb.remove(&run_pb);

            pb.set_style(ProgressStyle::default_spinner().template("{msg}").unwrap());

            if let Some(summary) = &summary {
                pb.finish_with_message(format!(
                    "   {} {:?} - RPS: {:.0} | TPS: {} | Latency: {} | Errors: {} | Mem: {}",
                    console::style("✔").green(),
                    test,
                    summary.requests_per_sec,
                    humanize_bytes_binary!(summary.bytes_per_sec),
                    format_latency(summary.latency_p99),
                    summary.total_errors,
                    humanize_bytes_binary!(summary.memory_usage_bytes),
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

fn find_max_stable_performance(
    data: &[wfb_storage::TestCaseRaw],
    final_memory_usage: u64,
    final_cpu_usage: f64,
) -> Option<wfb_storage::TestCaseSummary> {
    if data.is_empty() {
        return None;
    }

    // Window size in seconds (samples)
    let window_size = 5;

    if data.len() < window_size {
        // Not enough data for window, fallback to max RPS
        let best = data
            .iter()
            .max_by(|a, b| a.requests_per_sec.partial_cmp(&b.requests_per_sec).unwrap())?;
        let mut summary = raw_to_summary(best, final_memory_usage, final_cpu_usage);

        // Use final cumulative values from the last sample
        if let Some(last) = data.last() {
            summary.total_requests = last.total_requests;
            summary.total_bytes = last.total_bytes;
            summary.total_errors = last.total_errors;

            // Calculate average bytes_per_sec from total bytes and elapsed time
            summary.bytes_per_sec = if last.elapsed_secs > 0 {
                last.total_bytes / last.elapsed_secs
            } else {
                0
            };

            // Use final RPS statistics (these are cumulative stats over entire test)
            summary.req_per_sec_avg = last.req_per_sec_avg;
            summary.req_per_sec_stdev = last.req_per_sec_stdev;
            summary.req_per_sec_max = last.req_per_sec_max;
            summary.req_per_sec_stdev_pct = last.req_per_sec_stdev_pct;
        }
        // Use max latency across all samples
        summary.latency_max = data.iter().map(|x| x.latency_max).max().unwrap_or(0);

        return Some(summary);
    }

    let mut best_window_avg_rps = 0.0;
    let mut best_window_end_index = 0;

    for i in window_size..=data.len() {
        let window = &data[i - window_size..i];
        let avg_rps: f64 =
            window.iter().map(|x| x.requests_per_sec).sum::<f64>() / window_size as f64;

        if avg_rps > best_window_avg_rps {
            best_window_avg_rps = avg_rps;
            best_window_end_index = i - 1; // Index of the last element in the window
        }
    }

    // We take the sample at the end of the best window as the representative for latency etc.
    // But we use the window's average RPS as the reported RPS.
    let mut summary = raw_to_summary(
        &data[best_window_end_index],
        final_memory_usage,
        final_cpu_usage,
    );
    summary.requests_per_sec = best_window_avg_rps;

    // FIX: Use final cumulative values from the last sample, not from the best window
    if let Some(last) = data.last() {
        summary.total_requests = last.total_requests;
        summary.total_bytes = last.total_bytes;
        summary.total_errors = last.total_errors;

        // Calculate average bytes_per_sec from total bytes and elapsed time
        summary.bytes_per_sec = if last.elapsed_secs > 0 {
            last.total_bytes / last.elapsed_secs
        } else {
            0
        };

        // Use final RPS statistics (these are cumulative stats over entire test)
        summary.req_per_sec_avg = last.req_per_sec_avg;
        summary.req_per_sec_stdev = last.req_per_sec_stdev;
        summary.req_per_sec_max = last.req_per_sec_max;
        summary.req_per_sec_stdev_pct = last.req_per_sec_stdev_pct;
    }

    // Use the maximum latency observed across the entire test
    summary.latency_max = data
        .iter()
        .map(|x| x.latency_max)
        .max()
        .unwrap_or(summary.latency_max);

    Some(summary)
}

fn raw_to_summary(
    raw: &wfb_storage::TestCaseRaw,
    memory_usage: u64,
    cpu_usage: f64,
) -> wfb_storage::TestCaseSummary {
    wfb_storage::TestCaseSummary {
        requests_per_sec: raw.requests_per_sec,
        bytes_per_sec: raw.bytes_per_sec,
        total_requests: raw.total_requests,
        total_bytes: raw.total_bytes,
        total_errors: raw.total_errors,
        latency_mean: raw.latency_mean,
        latency_stdev: raw.latency_stdev,
        latency_max: raw.latency_max,
        latency_p50: raw.latency_p50,
        latency_p75: raw.latency_p75,
        latency_p90: raw.latency_p90,
        latency_p99: raw.latency_p99,
        latency_stdev_pct: raw.latency_stdev_pct,
        latency_distribution: raw.latency_distribution.clone(),
        errors: raw.errors.clone(),
        memory_usage_bytes: memory_usage,
        cpu_usage_percent: cpu_usage,
        req_per_sec_avg: raw.req_per_sec_avg,
        req_per_sec_stdev: raw.req_per_sec_stdev,
        req_per_sec_max: raw.req_per_sec_max,
        req_per_sec_stdev_pct: raw.req_per_sec_stdev_pct,
    }
}

fn parse_docker_cpu(s: &str) -> f64 {
    let s = s.trim().trim_end_matches('%');
    s.parse::<f64>().unwrap_or(0.0)
}
