use std::{collections::{HashMap, VecDeque}, sync::{Arc, Mutex}, time::Duration};

use anyhow::{Context, bail};
use console::style;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::time::sleep;
use wfb_storage::{Benchmark, BenchmarkTests, DatabaseKind, Environment};
use wrkr_api::JsonStats;

use crate::{consts, exec::{self, Executor}};

pub async fn build_database_images(env_config: &Environment, db_kinds: Vec<DatabaseKind>, m: &MultiProgress) -> anyhow::Result<()> {
    if !db_kinds.is_empty() {
        let mut handles = vec![];
        for db in db_kinds {
            let pb = m.add( ProgressBar::new_spinner());
            pb.set_style(ProgressStyle::default_spinner().template("{spinner:.blue} {msg}").unwrap());
            pb.enable_steady_tick(Duration::from_millis(100));
            pb.set_message(format!("Building: {:?}", db));
            let env_config = env_config.clone();
            handles.push(tokio::spawn(async move {
                let executor = match env_config {
                    Environment::Local(_) => exec::local::LocalExecutor::new(),
                    other => {
                        anyhow::bail!("Unsupported executor type: {:?}", other);
                    }
                };
                let res = build_database_image(&executor, &db, &pb).await;
                pb.set_style(ProgressStyle::default_spinner().template("{msg}").unwrap());
                match res {
                    Ok(_) => {
                        pb.finish_with_message(format!("{} {:?}", style("✔").green(), db));
                        Ok(())
                    }
                    Err(e) => {
                        pb.finish_with_message(format!("{} {:?} Failed: {}", style("✘").red(), db, e));
                        Err(e)
                    }
                }
            }));
        }

        for h in handles {
            h.await??;
        }
    }
    Ok(())
}

pub async fn setup_database(executor: &impl Executor, benchmark_name: &str, db_kind: &DatabaseKind, pb: &ProgressBar) -> anyhow::Result<()> {
    let (image_name, _, port, env_vars) = get_db_config(db_kind);

    let mut run_cmd = format!(
        "docker run -d --name {} -p 54350:{} ",
        image_name, port
    );
    for (k, v) in env_vars {
        run_cmd.push_str(&format!("-e {}={} ", k, v));
    }
    run_cmd.push_str(&format!("{}:latest", image_name));

    execute_with_progress(executor, &run_cmd, pb).await?;

    pb.set_message(format!("Waiting for DB - {}", benchmark_name));

    Ok(())
}

pub async fn run_app(executor: &impl Executor, benchmark: &Benchmark, pb: &ProgressBar) -> anyhow::Result<()> {
    let mut run_cmd = format!(
        "docker run -d --name {} -p 54320:8080 ", 
        benchmark.name
    );

    if let Some(db_kind) = &benchmark.database {
        let (db_host, db_port) = ("host.docker.internal", "54350");
        run_cmd.push_str(&format!("-e DB_HOST={} ", db_host));
        run_cmd.push_str(&format!("-e DB_PORT={} ", db_port));
        run_cmd.push_str("-e DB_USER=benchmarkdbuser ");
        run_cmd.push_str("-e DB_PASSWORD=benchmarkdbpass ");
        run_cmd.push_str("-e DB_NAME=hello_world ");
        run_cmd.push_str(&format!("-e DB_KIND={:?} ", db_kind));
    }

    // Add benchmark specific env vars
    for (k, v) in &benchmark.env {
        run_cmd.push_str(&format!("-e {}={} ", k, v));
    }

    run_cmd.push_str(&format!("{}:latest", benchmark.name));

    execute_with_progress(executor, &run_cmd, pb).await?;

    // Wait for App to be ready
    pb.set_message(format!("Waiting for App - {}", benchmark.name));
    wait_for_container_health(executor, &benchmark.name, pb).await?;

    Ok(())
}

async fn wait_for_container_health(executor: &impl Executor, container_name: &str, pb: &ProgressBar) -> anyhow::Result<()> {
    let max_retries = 30; 
    let mut retries = 0;
    
    loop {
        let cmd = format!("docker inspect --format \"{{{{if .State.Health}}}}{{{{.State.Health.Status}}}}{{{{else}}}}none{{{{end}}}}\" {}", container_name);
        
        let output = executor.execute(&cmd, |_| {}, |_| {}).await?;
        let status = output.trim();

        if status == "healthy" {
            pb.set_message(format!("Container {} is healthy", container_name));
            return Ok(());
        } else if status == "unhealthy" {
            anyhow::bail!("Container {} is unhealthy", container_name);
        }
        
        pb.set_message(format!("Waiting for App - {} (Health: {})", container_name, status));
        
        if retries >= max_retries {
             anyhow::bail!("Timeout waiting for container {} to be healthy", container_name);
        }
        
        sleep(Duration::from_secs(1)).await;
        retries += 1;
    }
}


pub async fn run_tests(benchmark: &Benchmark, host_url: &str, pb: &ProgressBar) -> anyhow::Result<()> {
    for test in &benchmark.tests {
        let script_path = match test {
            BenchmarkTests::PlainText => "scripts/wrkr_plaintext.lua",
            BenchmarkTests::JsonAggregate => "scripts/wrkr_json_aggregate.lua",
            BenchmarkTests::StaticFiles => "scripts/wrkr_static_files.lua",
        };

        pb.set_message(format!("Running test {:?} - {}", test, benchmark.name));
        
        let script_content = std::fs::read_to_string(script_path)
            .with_context(|| format!("Failed to read script file: {}", script_path))?;

        let config = wrkr_core::WrkConfig {
            script_content,
            host_url: host_url.to_string(),
        };

        pb.set_message(format!("Running test {:?} - {}", test, benchmark.name));
        let stats = wrkr_core::run_once(config.clone()).await?;
        
        if stats.total_errors > 0 {
            let errors = stats.errors.iter()
                .map(|(code, count)| format!("{}: {}", code, count))
                .collect::<Vec<_>>()
                .join("\n");
            bail!("Test {:?} failed with {} errors\n{}", test, stats.total_errors, errors);
        }
    }
    Ok(())
}

pub async fn run_benchmarks<T: Executor>(executor: &T, benchmark: &Benchmark, host_url: &str, pb: &ProgressBar, global_pb: &ProgressBar) -> anyhow::Result<HashMap<BenchmarkTests, JsonStats>> {
    let mut results = HashMap::new();
    
    for test in &benchmark.tests {
        let script_path = match test {
            BenchmarkTests::PlainText => "./scripts/wrkr_plaintext.lua",
            BenchmarkTests::JsonAggregate => "./scripts/wrkr_json_aggregate.lua",
            BenchmarkTests::StaticFiles => "./scripts/wrkr_static_files.lua",
        };

        pb.set_message(format!("Benchmarking {:?} - {}", test, benchmark.name));
        
        let cmd = format!(
            "docker run --rm --network host {} --url {} --script {} --output json --duration {} --connections {}",
            consts::WRKR_IMAGE, host_url, script_path, consts::BENCHMARK_DURATION_PER_TEST_SECS, consts::BENCHMARK_CONNECTIONS
        );

        let output = Arc::new(Mutex::new(Vec::new()));
        let output_clone = output.clone();
        let pb_clone = pb.clone();
        let global_pb_clone = global_pb.clone();
        let test_name = format!("{:?}", test);
        let benchmark_name = benchmark.name.clone();
        let last_elapsed = Arc::new(Mutex::new(0u64));
        let last_elapsed_clone = last_elapsed.clone();
        
        let exec_result = executor.execute(&cmd, move |line| {            
            if let Ok(stats) = serde_json::from_str::<JsonStats>(line) {
                output_clone.lock().unwrap().push(stats.clone());
                 let msg = format!("{} - {} | RPS: {:.0} | TPS: {} | Conns: {} | Errs: {}", 
                    benchmark_name,
                    test_name,
                    stats.requests_per_sec,
                    humanize_bytes::humanize_bytes_binary!(stats.bytes_per_sec),
                    stats.connections,
                    stats.total_errors
                );
                pb_clone.set_message(msg);
                
                let mut last = last_elapsed_clone.lock().unwrap();
                if stats.elapsed_secs > *last {
                    let diff = stats.elapsed_secs - *last;
                    global_pb_clone.inc(diff);
                    *last = stats.elapsed_secs;
                }
            }
        }, |_| {}).await;

        let final_elapsed = *last_elapsed.lock().unwrap();
        if final_elapsed < consts::BENCHMARK_DURATION_PER_TEST_SECS {
            global_pb.inc(consts::BENCHMARK_DURATION_PER_TEST_SECS - final_elapsed);
        }

        exec_result?;

        let mut output_vec = output.lock().unwrap();
        output_vec.sort_by(|a,b| a.requests_per_sec.partial_cmp(&b.requests_per_sec).unwrap());
        
        if let Some(stats) = output_vec.last() {
            results.insert(test.clone(), stats.clone());
        } else {
            bail!("No valid stats received for test {:?}", test);
        }
    }
    Ok(results)
}

pub async fn build_image(executor: &impl Executor, name: &str, path: &str, pb: &ProgressBar) -> anyhow::Result<()> {
    let cmd = format!("docker build -t {}:latest {}", name, path);
    execute_with_progress(executor, &cmd, pb).await
}

pub async fn build_wrkr_image(executor: &impl Executor, pb: &ProgressBar) -> anyhow::Result<()> {
    let cmd = format!("docker buildx build --platform linux/amd64,linux/arm64 -f Dockerfile.wrkr -t {} .", consts::WRKR_IMAGE);
    execute_with_progress(executor, &cmd, pb).await
}

pub async fn build_database_image(executor: &impl Executor, db_kind: &DatabaseKind, pb: &ProgressBar) -> anyhow::Result<()> {
    let (image_name, build_path, _, _) = get_db_config(db_kind);       
    let build_cmd = format!("docker build -t {}:latest {}", image_name, build_path);
    execute_with_progress(executor, &build_cmd, pb).await
}

pub fn get_db_config(db_kind: &DatabaseKind) -> (&'static str, &'static str, u16, Vec<(&'static str, &'static str)>) {
    match db_kind {
        DatabaseKind::Postgres => ("postgres", "benchmarks_db/pg", 5432, vec![
            ("POSTGRES_PASSWORD", "benchmarkdbpass"),
            ("POSTGRES_USER", "benchmarkdbuser"),
            ("POSTGRES_DB", "hello_world"),
        ]),
        DatabaseKind::Mysql => ("mysql", "benchmarks_db/mysql", 3306, vec![
            ("MYSQL_ROOT_PASSWORD", "benchmarkdbpass"),
            ("MYSQL_DATABASE", "hello_world"),
            ("MYSQL_USER", "benchmarkdbuser"),
            ("MYSQL_PASSWORD", "benchmarkdbpass"),
        ]),
        DatabaseKind::Mongodb => ("mongodb", "benchmarks_db/mongodb", 27017, vec![]),
        DatabaseKind::Mssql => ("mssql", "benchmarks_db/mssql", 1433, vec![
            ("ACCEPT_EULA", "Y"),
            ("MSSQL_SA_PASSWORD", "Benchmark!12345"),
        ]),
        DatabaseKind::Mariadb => ("mariadb", "benchmarks_db/mariadb", 3306, vec![
            ("MARIADB_ROOT_PASSWORD", "benchmarkdbpass"),
            ("MARIADB_DATABASE", "hello_world"),
            ("MARIADB_USER", "benchmarkdbuser"),
            ("MARIADB_PASSWORD", "benchmarkdbpass"),
        ]),
    }
}


pub async fn cleanup(executor: &impl Executor, benchmark_name: &str, db_kind: &Option<DatabaseKind>, pb: &ProgressBar) -> anyhow::Result<()> {
    // Stop and remove app
    execute_with_progress(executor, &format!("docker stop {}", benchmark_name), pb).await.ok();
    execute_with_progress(executor, &format!("docker rm {}", benchmark_name), pb).await.ok();

    // Stop and remove db if exists
    if let Some(db_kind) = db_kind {
        let (image_name, _, _, _) = get_db_config(db_kind);
        execute_with_progress(executor, &format!("docker stop {}", image_name), pb).await.ok();
        execute_with_progress(executor, &format!("docker rm {}", image_name), pb).await.ok();
    }
    
    Ok(())
}

pub async fn execute_with_progress(
    executor: &impl Executor, 
    cmd: &str,    
    pb: &ProgressBar
) -> anyhow::Result<()> {
    let cmd = cmd.to_string();
    pb.set_message(cmd.clone());
    
    let last_lines = Arc::new(Mutex::new(VecDeque::with_capacity(6)));
    let stderr_log = Arc::new(Mutex::new(String::new()));
    
    let last_lines_clone = last_lines.clone();
    let pb_clone = pb.clone();
    let cmd_clone = cmd.clone();
    let on_stdout = move |line: &str| {
        let mut lines = last_lines_clone.lock().unwrap();
        if lines.len() >= 6 { lines.pop_front(); }
        lines.push_back(line.to_string());
        
        let gray_lines = lines.iter()
            .map(|l| format!("{}  {}", style("===>").black().bright(), style(l).black().bright()))
            .collect::<Vec<_>>()
            .join("\n");
            
        pb_clone.set_message(format!("{}\n{}", cmd_clone, gray_lines));
    };
    
    let last_lines_clone = last_lines.clone();
    let stderr_log_clone = stderr_log.clone();
    let pb_clone = pb.clone();
    let cmd_clone = cmd.clone();

    let on_stderr = move |line: &str| {
        let mut lines = last_lines_clone.lock().unwrap();
        if lines.len() >= 6 { lines.pop_front(); }
        lines.push_back(line.to_string());
        
        let gray_lines = lines.iter()
            .map(|l| format!("{}  {}", style("===>").black().bright(), style(l).black().bright()))
            .collect::<Vec<_>>()
            .join("\n");
            
        pb_clone.set_message(format!("{}\n{}", cmd_clone, gray_lines));
        
        let mut log = stderr_log_clone.lock().unwrap();
        log.push_str(line);
        log.push('\n');
    };

    match executor.execute(&cmd, on_stdout, on_stderr).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let stderr = stderr_log.lock().unwrap();
            if !stderr.is_empty() {
                bail!("Command failed: {}\nStderr:\n{}", e, stderr);
            } else {
                Err(e)
            }
        }
    }
}