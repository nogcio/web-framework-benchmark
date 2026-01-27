use crate::consts;
use crate::db_config::get_db_config;
use crate::exec::Executor;
use crate::runner::Runner;
use anyhow::{Context, bail};
use humanize_bytes::humanize_bytes_binary;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;
use wfb_storage::{Benchmark, BenchmarkTests, DatabaseKind};

impl<E: Executor + Clone + Send + 'static> Runner<E> {
    fn scripts_mount_host_path(&self) -> anyhow::Result<String> {
        if self.config.is_remote {
            Ok(format!("{}/scripts", consts::REMOTE_WRKR_PATH))
        } else {
            let cwd = std::env::current_dir().context("Failed to resolve current_dir")?;
            Ok(cwd.join("scripts").to_string_lossy().to_string())
        }
    }

    fn wrkr_env_for_test(
        &self,
        _test: BenchmarkTests,
        mode: &str,
        duration: &str,
        max_vus: u64,
    ) -> Vec<(&'static str, String)> {
        vec![
            ("WFB_MODE", mode.to_string()),
            ("WFB_DURATION", duration.to_string()),
            ("WFB_MAX_VUS", max_vus.to_string()),
            ("BASE_URL", self.config.app_host_url.clone()),
        ]
    }

    fn max_vus_for_test(test: BenchmarkTests) -> u64 {
        match test {
            BenchmarkTests::PlainText => consts::UVS_PLAINTEXT,
            BenchmarkTests::JsonAggregate => consts::UVS_JSON,
            BenchmarkTests::DbComplex => consts::UVS_DB_COMPLEX,
            BenchmarkTests::GrpcAggregate => consts::UVS_GRPC,
            BenchmarkTests::StaticFiles => consts::UVS_STATIC,
        }
    }

    fn warmup_vus_for_test(test: BenchmarkTests) -> u64 {
        let max_vus = Self::max_vus_for_test(test);

        // Keep warmup much lighter than the real run:
        //  - scale down to ~1/16 of target VUs (rounding up)
        //  - cap to a small absolute maximum
        let scaled = max_vus.div_ceil(16);
        let capped = scaled.min(consts::BENCHMARK_WARMUP_MAX_VUS);
        capped.max(1)
    }

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
                BenchmarkTests::GrpcAggregate => consts::SCRIPT_GRPC_AGGREGATE,
            };

            pb.set_message(format!("Running test {:?} - {}", test, benchmark.name));

            // Verify run: short constant VUs to validate correctness.
            let scripts_mount = self.scripts_mount_host_path()?;
            let duration_str = format!("{}s", consts::VERIFY_DURATION_SECS);
            let envs = self.wrkr_env_for_test(
                *test,
                "verify",
                duration_str.as_str(),
                consts::VERIFY_MAX_VUS,
            );

            let mut cmd = self
                .wrkr_docker
                .run_command(consts::WRKR_IMAGE, "wrkr-verify")
                .detach(false)
                .ulimit("nofile=1000000:1000000")
                .volume(scripts_mount.as_str(), "/scripts")
                .arg("run")
                .arg(script_path)
                .arg("--output")
                .arg("json");

            for (k, v) in envs {
                cmd = cmd.env(k, v);
            }

            let last_progress: std::sync::Arc<std::sync::Mutex<Option<WrkrJsonProgressLine>>> =
                std::sync::Arc::new(std::sync::Mutex::new(None));
            let last_summary: std::sync::Arc<std::sync::Mutex<Option<WrkrJsonSummaryLine>>> =
                std::sync::Arc::new(std::sync::Mutex::new(None));

            let last_progress_clone = last_progress.clone();
            let last_summary_clone = last_summary.clone();
            let run_res = self
                .wrkr_docker
                .execute_run_with_std_out(
                    cmd,
                    move |line| {
                        let Ok(v) = serde_json::from_str::<WrkrJsonLine>(line) else {
                            return;
                        };
                        match v {
                            WrkrJsonLine::Progress(p) => {
                                if let Ok(mut guard) = last_progress_clone.lock() {
                                    *guard = Some(p);
                                }
                            }
                            WrkrJsonLine::Summary(s) => {
                                if let Ok(mut guard) = last_summary_clone.lock() {
                                    *guard = Some(s);
                                }
                            }
                        }
                    },
                    &ProgressBar::hidden(),
                )
                .await;

            self.wrkr_docker
                .stop_and_remove("wrkr-verify", &ProgressBar::hidden())
                .await;

            let run_err = run_res.err();

            let summary = last_summary
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            let progress = last_progress
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();

            let (checks_failed_total, checks_failed) = if let Some(s) = summary {
                let mut merged: HashMap<String, u64> = HashMap::new();
                for scenario in &s.scenarios {
                    for (k, v) in &scenario.checks_failed {
                        *merged.entry(k.clone()).or_insert(0) =
                            merged.get(k).copied().unwrap_or(0).saturating_add(*v);
                    }
                }
                (s.totals.checks_failed_total, merged)
            } else if let Some(p) = progress {
                (p.checks_failed_total, p.checks_failed)
            } else {
                if let Some(e) = run_err {
                    bail!("wrkr verify run failed: {}", e);
                }
                bail!(
                    "wrkr produced no JSON progress/summary lines during verify for {:?}; check script/runtime",
                    test
                );
            };

            if checks_failed_total > 0 {
                let errors_str = checks_failed
                    .iter()
                    .map(|(code, count)| format!("{}: {}", code, count))
                    .collect::<Vec<_>>()
                    .join("\n");
                bail!(
                    "Test {:?} failed with {} errors\n{}",
                    test,
                    checks_failed_total,
                    errors_str
                );
            }

            if checks_failed_total > 0 {
                bail!(
                    "Test {:?} failed: checks_failed_total={}",
                    test,
                    checks_failed_total
                );
            }

            if let Some(e) = run_err {
                bail!("wrkr verify run failed: {}", e);
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
        let style =
            match ProgressStyle::default_spinner().template("{spinner:.blue} {prefix} {msg}") {
                Ok(style) => style,
                Err(_) => ProgressStyle::default_spinner(),
            };
        pb.set_style(style);
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

        let style = match ProgressStyle::default_spinner().template("{msg}") {
            Ok(style) => style,
            Err(_) => ProgressStyle::default_spinner(),
        };
        pb.set_style(style);

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
        let style =
            match ProgressStyle::default_spinner().template("{spinner:.blue} {prefix} {msg}") {
                Ok(style) => style,
                Err(_) => ProgressStyle::default_spinner(),
            };
        pb.set_style(style);
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

        let style = match ProgressStyle::default_spinner().template("{msg}") {
            Ok(style) => style,
            Err(_) => ProgressStyle::default_spinner(),
        };
        pb.set_style(style);

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
        let style =
            match ProgressStyle::default_spinner().template("{spinner:.blue} {prefix} {msg}") {
                Ok(style) => style,
                Err(_) => ProgressStyle::default_spinner(),
            };
        pb.set_style(style);
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

        let style = match ProgressStyle::default_spinner().template("{msg}") {
            Ok(style) => style,
            Err(_) => ProgressStyle::default_spinner(),
        };
        pb.set_style(style);
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
            let style = match ProgressStyle::default_spinner()
                .template("{spinner:.blue} {prefix} [{bar:40.cyan/blue}] {msg}")
            {
                Ok(style) => style.progress_chars("#>-"),
                Err(_) => ProgressStyle::default_spinner().progress_chars("#>-"),
            };
            pb.set_style(style);
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

            let script_path = match test {
                BenchmarkTests::PlainText => consts::SCRIPT_PLAINTEXT,
                BenchmarkTests::JsonAggregate => consts::SCRIPT_JSON,
                BenchmarkTests::StaticFiles => consts::SCRIPT_STATIC,
                BenchmarkTests::DbComplex => consts::SCRIPT_DB_COMPLEX,
                BenchmarkTests::GrpcAggregate => consts::SCRIPT_GRPC_AGGREGATE,
            };

            // --- WARMUP PHASE ---
            {
                let warmup_pb = mb.add(ProgressBar::new(consts::BENCHMARK_WARMUP_DURATION_SECS));
                warmup_pb.set_style(
                    match ProgressStyle::default_bar().template(
                        "{spinner:.yellow} {prefix:.yellow} [{bar:40.yellow/white}] {msg:.yellow}",
                    ) {
                        Ok(style) => style.progress_chars("=>-"),
                        Err(_) => ProgressStyle::default_bar().progress_chars("=>-"),
                    },
                );
                warmup_pb.set_prefix(format!("[{}/{}/warmup]", benchmark.name, test));
                warmup_pb.enable_steady_tick(Duration::from_millis(100));

                let scripts_mount = self.scripts_mount_host_path()?;
                let warmup_vus = Self::warmup_vus_for_test(*test);
                let warmup_duration_str = format!("{}s", consts::BENCHMARK_WARMUP_DURATION_SECS);
                let envs = self.wrkr_env_for_test(
                    *test,
                    "warmup",
                    warmup_duration_str.as_str(),
                    warmup_vus,
                );

                let mut cmd = self
                    .wrkr_docker
                    .run_command(consts::WRKR_IMAGE, "wrkr-warmup")
                    .detach(false)
                    .ulimit("nofile=1000000:1000000")
                    .volume(scripts_mount.as_str(), "/scripts")
                    .arg("run")
                    .arg(script_path)
                    .arg("--output")
                    .arg("json");

                for (k, v) in envs {
                    cmd = cmd.env(k, v);
                }

                let warmup_pb_clone = warmup_pb.clone();
                let _ = self
                    .wrkr_docker
                    .execute_run_with_std_out(
                        cmd,
                        move |line| {
                            let Ok(line) = serde_json::from_str::<WrkrJsonLine>(line) else {
                                return;
                            };
                            let WrkrJsonLine::Progress(stats) = line else {
                                return;
                            };

                            warmup_pb_clone.set_position(
                                stats
                                    .elapsed_secs
                                    .min(consts::BENCHMARK_WARMUP_DURATION_SECS),
                            );
                            warmup_pb_clone.set_message(format!(
                                "RPS: {:.0} | Latency: {} | Errors: {}",
                                stats.requests_per_sec,
                                format_latency(stats.latency_p99),
                                stats.checks_failed_total
                            ));
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
            let style = match ProgressStyle::default_bar().template("{msg}") {
                Ok(style) => style,
                Err(_) => ProgressStyle::default_bar(),
            };
            run_pb.set_style(style);

            let duration = format!("{}", consts::BENCHMARK_DURATION_PER_TEST_SECS);
            let duration_str = format!("{}s", duration);

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

            let scripts_mount = self.scripts_mount_host_path()?;

            let mut cmd = self
                .wrkr_docker
                .run_command(consts::WRKR_IMAGE, "wrkr-runner")
                .detach(false)
                .ulimit("nofile=1000000:1000000")
                .volume(scripts_mount.as_str(), "/scripts")
                .arg("run")
                .arg(script_path)
                .arg("--output")
                .arg("json");

            let max_vus = Self::max_vus_for_test(*test);
            let envs = self.wrkr_env_for_test(*test, "run", &duration_str, max_vus);
            for (k, v) in envs {
                cmd = cmd.env(k, v);
            }

            let raw_data_collection = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            let raw_data_collection_clone = raw_data_collection.clone();

            let pb_clone = pb.clone();
            let resource_usage_read = resource_usage.clone();
            let _output = self.wrkr_docker.execute_run_with_std_out(cmd, move |line| {
                let Ok(line) = serde_json::from_str::<WrkrJsonLine>(line) else {
                    return;
                };
                let WrkrJsonLine::Progress(stats) = line else {
                    return;
                };
                    let total_errors = stats.checks_failed.values().copied().sum();

                    let (mem_bytes, cpu_usage) = if let Ok(guard) = resource_usage_read.lock() {
                        guard.last().cloned().unwrap_or((0, 0.0))
                    } else {
                        (0, 0.0)
                    };

                    let latency_mean_us = stats.latency_mean;
                    let latency_stdev_us = stats.latency_stdev;
                    let latency_max_us = stats.latency_max;
                    let latency_p50_us = stats.latency_p50;
                    let latency_p75_us = stats.latency_p75;
                    let latency_p90_us = stats.latency_p90;
                    let latency_p99_us = stats.latency_p99;

                    let raw_item = wfb_storage::TestCaseRaw {
                        elapsed_secs: stats.elapsed_secs,
                        connections: stats.connections,
                        requests_per_sec: stats.requests_per_sec,
                        bytes_per_sec: stats.bytes_received_per_sec + stats.bytes_sent_per_sec,
                        total_requests: stats.total_requests,
                        total_bytes: stats.total_bytes_received + stats.total_bytes_sent,
                        total_errors,
                        latency_mean: latency_mean_us,
                        latency_stdev: latency_stdev_us,
                        latency_max: latency_max_us,
                        latency_p50: latency_p50_us,
                        latency_p75: latency_p75_us,
                        latency_p90: latency_p90_us,
                        latency_p99: latency_p99_us,
                        latency_stdev_pct: stats.latency_stdev_pct,
                        latency_distribution: Vec::new(),
                        errors: stats.checks_failed.clone(),
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
                        humanize_bytes_binary!(stats.bytes_received_per_sec + stats.bytes_sent_per_sec),
                        format_latency(latency_p99_us),
                        total_errors,
                        humanize_bytes_binary!(mem_bytes),
                        cpu_usage
                    ));
            }, &run_pb).await?;

            monitor_handle.abort();

            let raw_data = raw_data_collection
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();

            if raw_data.is_empty() {
                bail!(
                    "wrkr produced no JSON progress lines for {:?}; check that --output json is supported and the script is valid",
                    test
                );
            }

            let final_resource_usage = resource_usage
                .lock()
                .unwrap_or_else(|e| e.into_inner())
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

            let style = match ProgressStyle::default_spinner().template("{msg}") {
                Ok(style) => style,
                Err(_) => ProgressStyle::default_spinner(),
            };
            pb.set_style(style);

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

#[derive(Debug, Clone)]
enum WrkrJsonLine {
    Progress(WrkrJsonProgressLine),
    Summary(WrkrJsonSummaryLine),
}

impl<'de> Deserialize<'de> for WrkrJsonLine {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;

        // New wrkr output (as of wrkr.ndjson.v1): every line includes schema + camelCase keys.
        let is_v1 = value
            .get("schema")
            .and_then(|v| v.as_str())
            .is_some_and(|s| s == "wrkr.ndjson.v1");

        if !is_v1 {
            let schema = value
                .get("schema")
                .and_then(|v| v.as_str())
                .unwrap_or("<missing>");
            return Err(serde::de::Error::custom(format!(
                "unsupported wrkr JSON output schema: {schema}; expected wrkr.ndjson.v1. Ensure you are using a recent nogcio/wrkr and `--output json`."
            )));
        }

        let v1: WrkrNdjsonV1Line = serde_json::from_value(value)
            .map_err(|e| serde::de::Error::custom(format!("wrkr v1 parse error: {e}")))?;

        Ok(match v1 {
            WrkrNdjsonV1Line::Progress(p) => WrkrJsonLine::Progress(p.into_legacy_progress()),
            WrkrNdjsonV1Line::Summary(s) => WrkrJsonLine::Summary(s.into_legacy_summary()),
        })
    }
}

#[derive(Debug, Deserialize, Clone)]
struct WrkrJsonProgressLine {
    // Interval timing in seconds.
    // We don't currently use it, but we keep it for potential future smoothing.
    #[allow(dead_code)]
    pub interval_secs: f64,

    pub elapsed_secs: u64,
    pub connections: u64,

    pub requests_per_sec: f64,
    pub bytes_received_per_sec: u64,
    pub bytes_sent_per_sec: u64,

    pub total_requests: u64,
    pub total_bytes_received: u64,
    pub total_bytes_sent: u64,
    pub checks_failed_total: u64,

    // Latency metrics are stored as microseconds.
    pub latency_mean: f64,
    pub latency_stdev: f64,
    pub latency_max: u64,
    pub latency_p50: u64,
    pub latency_p75: u64,
    pub latency_p90: u64,
    pub latency_p99: u64,
    pub latency_stdev_pct: f64,

    pub checks_failed: HashMap<String, u64>,

    pub req_per_sec_avg: f64,
    pub req_per_sec_stdev: f64,
    pub req_per_sec_max: f64,
    pub req_per_sec_stdev_pct: f64,
}

#[derive(Debug, Deserialize, Clone)]
struct WrkrJsonSummaryLine {
    pub scenarios: Vec<WrkrJsonScenarioSummary>,
    pub totals: WrkrJsonTotals,
}

#[derive(Debug, Deserialize, Clone)]
struct WrkrJsonScenarioSummary {
    #[allow(dead_code)]
    pub scenario: String,

    #[allow(dead_code)]
    pub requests_total: u64,
    #[allow(dead_code)]
    pub failed_requests_total: u64,
    #[allow(dead_code)]
    pub bytes_received_total: u64,
    #[allow(dead_code)]
    pub bytes_sent_total: u64,
    #[allow(dead_code)]
    pub iterations_total: u64,

    #[allow(dead_code)]
    pub checks_failed_total: u64,
    pub checks_failed: HashMap<String, u64>,
}

#[derive(Debug, Deserialize, Clone)]
struct WrkrJsonTotals {
    #[allow(dead_code)]
    pub requests_total: u64,
    #[allow(dead_code)]
    pub failed_requests_total: u64,
    #[allow(dead_code)]
    pub bytes_received_total: u64,
    #[allow(dead_code)]
    pub bytes_sent_total: u64,
    #[allow(dead_code)]
    pub iterations_total: u64,

    pub checks_failed_total: u64,
}

// --- wrkr.ndjson.v1 (new output schema) ---

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum WrkrNdjsonV1Line {
    Progress(WrkrNdjsonV1ProgressLine),
    Summary(WrkrNdjsonV1SummaryLine),
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct WrkrNdjsonV1ProgressLine {
    #[allow(dead_code)]
    pub schema: String,

    #[allow(dead_code)]
    pub tick: u64,

    pub elapsed_seconds: f64,
    pub interval_seconds: f64,

    #[allow(dead_code)]
    pub scenario: String,
    #[allow(dead_code)]
    pub exec: String,

    pub executor: WrkrNdjsonV1ExecutorProgress,
    pub metrics: WrkrNdjsonV1ProgressMetrics,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct WrkrNdjsonV1ExecutorProgress {
    #[allow(dead_code)]
    pub kind: String,
    pub vus_active: u64,
    #[allow(dead_code)]
    pub vus_max: Option<u64>,
    #[allow(dead_code)]
    pub dropped_iterations_total: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct WrkrNdjsonV1ProgressMetrics {
    pub requests_per_sec: f64,
    pub bytes_received_per_sec: u64,
    pub bytes_sent_per_sec: u64,

    pub total_requests: u64,
    #[allow(dead_code)]
    pub total_failed_requests: u64,
    #[allow(dead_code)]
    pub total_iterations: u64,
    pub total_bytes_received: u64,
    pub total_bytes_sent: u64,
    pub checks_failed_total: u64,

    pub latency_seconds: WrkrNdjsonV1LatencySecondsProgress,

    pub req_per_sec_avg: f64,
    pub req_per_sec_stdev: f64,
    pub req_per_sec_max: f64,
    pub req_per_sec_stdev_pct: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct WrkrNdjsonV1LatencySecondsProgress {
    pub mean: f64,
    pub stdev: f64,
    pub max: f64,
    pub p50: f64,
    pub p75: f64,
    pub p90: f64,
    pub p99: f64,
    pub stdev_pct: f64,
}

impl WrkrNdjsonV1ProgressLine {
    fn into_legacy_progress(self) -> WrkrJsonProgressLine {
        let mut checks_failed: HashMap<String, u64> = HashMap::new();
        if self.metrics.checks_failed_total > 0 {
            checks_failed.insert(
                "checks_failed_total".to_string(),
                self.metrics.checks_failed_total,
            );
        }

        WrkrJsonProgressLine {
            interval_secs: self.interval_seconds,
            elapsed_secs: self.elapsed_seconds.floor().max(0.0) as u64,
            connections: self.executor.vus_active,
            requests_per_sec: self.metrics.requests_per_sec,
            bytes_received_per_sec: self.metrics.bytes_received_per_sec,
            bytes_sent_per_sec: self.metrics.bytes_sent_per_sec,
            total_requests: self.metrics.total_requests,
            total_bytes_received: self.metrics.total_bytes_received,
            total_bytes_sent: self.metrics.total_bytes_sent,
            checks_failed_total: self.metrics.checks_failed_total,
            latency_mean: secs_f64_to_micros_f64(self.metrics.latency_seconds.mean),
            latency_stdev: secs_f64_to_micros_f64(self.metrics.latency_seconds.stdev),
            latency_max: secs_f64_to_micros_u64(self.metrics.latency_seconds.max),
            latency_p50: secs_f64_to_micros_u64(self.metrics.latency_seconds.p50),
            latency_p75: secs_f64_to_micros_u64(self.metrics.latency_seconds.p75),
            latency_p90: secs_f64_to_micros_u64(self.metrics.latency_seconds.p90),
            latency_p99: secs_f64_to_micros_u64(self.metrics.latency_seconds.p99),
            latency_stdev_pct: self.metrics.latency_seconds.stdev_pct,
            checks_failed,
            req_per_sec_avg: self.metrics.req_per_sec_avg,
            req_per_sec_stdev: self.metrics.req_per_sec_stdev,
            req_per_sec_max: self.metrics.req_per_sec_max,
            req_per_sec_stdev_pct: self.metrics.req_per_sec_stdev_pct,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct WrkrNdjsonV1SummaryLine {
    #[allow(dead_code)]
    pub schema: String,

    pub scenarios: Vec<WrkrNdjsonV1ScenarioSummary>,
    pub totals: WrkrNdjsonV1Totals,

    #[allow(dead_code)]
    pub thresholds: WrkrNdjsonV1Thresholds,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct WrkrNdjsonV1ScenarioSummary {
    pub scenario: String,

    #[allow(dead_code)]
    pub exec: Option<String>,
    #[allow(dead_code)]
    pub executor: Option<serde_json::Value>,

    pub requests_total: u64,
    pub failed_requests_total: u64,
    pub bytes_received_total: u64,
    pub bytes_sent_total: u64,
    pub iterations_total: u64,

    pub checks: Option<WrkrNdjsonV1ChecksSummary>,

    #[allow(dead_code)]
    pub latency_seconds: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct WrkrNdjsonV1ChecksSummary {
    #[allow(dead_code)]
    pub total: u64,
    #[allow(dead_code)]
    pub passed: u64,
    pub failed: u64,
    pub by_series: Vec<WrkrNdjsonV1CheckSeries>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct WrkrNdjsonV1CheckSeries {
    pub name: String,
    pub group: Option<String>,
    pub tags: HashMap<String, String>,
    #[allow(dead_code)]
    pub passed: u64,
    pub failed: u64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct WrkrNdjsonV1Totals {
    #[allow(dead_code)]
    pub requests_total: u64,
    #[allow(dead_code)]
    pub failed_requests_total: u64,
    #[allow(dead_code)]
    pub bytes_received_total: u64,
    #[allow(dead_code)]
    pub bytes_sent_total: u64,
    #[allow(dead_code)]
    pub iterations_total: u64,
    pub checks_failed_total: u64,
}

#[derive(Debug, Deserialize, Clone)]
struct WrkrNdjsonV1Thresholds {
    #[allow(dead_code)]
    pub violations: Vec<serde_json::Value>,
}

impl WrkrNdjsonV1SummaryLine {
    fn into_legacy_summary(self) -> WrkrJsonSummaryLine {
        let scenarios = self
            .scenarios
            .into_iter()
            .map(|s| {
                let (checks_failed_total, checks_failed) = if let Some(checks) = &s.checks {
                    (checks.failed, checks_series_to_map(&checks.by_series))
                } else {
                    (0, HashMap::new())
                };

                WrkrJsonScenarioSummary {
                    scenario: s.scenario,
                    requests_total: s.requests_total,
                    failed_requests_total: s.failed_requests_total,
                    bytes_received_total: s.bytes_received_total,
                    bytes_sent_total: s.bytes_sent_total,
                    iterations_total: s.iterations_total,
                    checks_failed_total,
                    checks_failed,
                }
            })
            .collect();

        WrkrJsonSummaryLine {
            scenarios,
            totals: WrkrJsonTotals {
                requests_total: self.totals.requests_total,
                failed_requests_total: self.totals.failed_requests_total,
                bytes_received_total: self.totals.bytes_received_total,
                bytes_sent_total: self.totals.bytes_sent_total,
                iterations_total: self.totals.iterations_total,
                checks_failed_total: self.totals.checks_failed_total,
            },
        }
    }
}

fn checks_series_to_map(series: &[WrkrNdjsonV1CheckSeries]) -> HashMap<String, u64> {
    let mut out = HashMap::new();
    for s in series {
        let mut key = String::new();
        if let Some(group) = &s.group {
            if !group.is_empty() {
                key.push_str(group);
                key.push_str(": ");
            }
        }
        key.push_str(&s.name);

        if !s.tags.is_empty() {
            let mut tags: Vec<(&String, &String)> = s.tags.iter().collect();
            tags.sort_by(|a, b| a.0.cmp(b.0));
            let tags_str = tags
                .into_iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(",");
            key.push_str(" [");
            key.push_str(&tags_str);
            key.push(']');
        }

        if s.failed > 0 {
            out.insert(key, s.failed);
        }
    }
    out
}

fn secs_f64_to_micros_u64(secs: f64) -> u64 {
    if !secs.is_finite() || secs <= 0.0 {
        return 0;
    }
    (secs * 1_000_000.0).round() as u64
}

fn secs_f64_to_micros_f64(secs: f64) -> f64 {
    if !secs.is_finite() || secs <= 0.0 {
        return 0.0;
    }
    secs * 1_000_000.0
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
            .max_by(|a, b| a.requests_per_sec.total_cmp(&b.requests_per_sec))?;
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

#[cfg(test)]
mod tests {
    use super::WrkrJsonLine;

    #[test]
    fn wrkr_progress_line_v1_seconds_is_converted_to_us() {
        // Matches wrkr.ndjson.v1: camelCase keys and latency values are seconds (floats).
        let js = r#"{
            "schema": "wrkr.ndjson.v1",
            "kind": "progress",
            "tick": 1,
            "elapsedSeconds": 1.9,
            "intervalSeconds": 1.0,
            "scenario": "Default",
            "exec": "Default",
            "executor": {"kind": "constant-vus", "vusActive": 4, "vusMax": 4, "droppedIterationsTotal": 0},
            "metrics": {
                "requestsPerSec": 1000.0,
                "bytesReceivedPerSec": 10,
                "bytesSentPerSec": 20,
                "totalRequests": 1000,
                "totalFailedRequests": 0,
                "totalIterations": 1000,
                "totalBytesReceived": 100,
                "totalBytesSent": 200,
                "checksFailedTotal": 2,
                "latencySeconds": {"mean": 0.00025, "stdev": 0.00001, "max": 0.001, "p50": 0.0002, "p75": 0.0003, "p90": 0.0004, "p99": 0.0005, "stdevPct": 0.0},
                "reqPerSecAvg": 900.0,
                "reqPerSecStdev": 1.0,
                "reqPerSecMax": 1100.0,
                "reqPerSecStdevPct": 0.1
            }
        }"#;

        let v: WrkrJsonLine = serde_json::from_str(js).expect("parse");
        match v {
            WrkrJsonLine::Progress(p) => {
                // elapsedSeconds=1.9 -> floor -> 1
                assert_eq!(p.elapsed_secs, 1);
                // p99=0.0005s -> 500us
                assert_eq!(p.latency_p99, 500);
                assert!((p.latency_mean - 250.0).abs() < 1e-9);
                assert_eq!(p.checks_failed_total, 2);
            }
            WrkrJsonLine::Summary(_) => panic!("expected progress"),
        }
    }

    #[test]
    fn wrkr_summary_line_v1_parses_and_extracts_failed_checks() {
        let js = r#"{
            "schema": "wrkr.ndjson.v1",
            "kind": "summary",
            "scenarios": [
                {
                    "scenario": "Default",
                    "exec": "Default",
                    "executor": null,
                    "requestsTotal": 10,
                    "failedRequestsTotal": 0,
                    "bytesReceivedTotal": 100,
                    "bytesSentTotal": 200,
                    "iterationsTotal": 10,
                    "checks": {
                        "total": 10,
                        "passed": 8,
                        "failed": 2,
                        "bySeries": [
                            {"name": "status is 200", "group": null, "tags": {"method": "GET"}, "passed": 8, "failed": 2}
                        ]
                    },
                    "latencySeconds": null
                }
            ],
            "totals": {
                "requestsTotal": 10,
                "failedRequestsTotal": 0,
                "bytesReceivedTotal": 100,
                "bytesSentTotal": 200,
                "iterationsTotal": 10,
                "checksFailedTotal": 2
            },
            "thresholds": {"violations": []}
        }"#;

        let v: WrkrJsonLine = serde_json::from_str(js).expect("parse");
        match v {
            WrkrJsonLine::Summary(s) => {
                assert_eq!(s.totals.checks_failed_total, 2);
                assert_eq!(s.scenarios.len(), 1);
                assert_eq!(s.scenarios[0].checks_failed_total, 2);
                assert_eq!(
                    s.scenarios[0]
                        .checks_failed
                        .get("status is 200 [method=GET]")
                        .copied()
                        .unwrap_or(0),
                    2
                );
            }
            WrkrJsonLine::Progress(_) => panic!("expected summary"),
        }
    }
}
