use crate::stats::StatsSnapshot;
use crate::{BenchmarkConfig, WrkConfig, error::*, stats::Stats, lua_env::create_lua_env};
use std::sync::Arc;
use reqwest::redirect::Policy;
use tokio::time::{sleep, Instant, Duration};
use tokio::task::JoinSet;
use mlua::Function;
use std::sync::atomic::{AtomicU64, Ordering};
use reqwest::Client;

enum ExecutionMode {
    Duration(Instant),
    Once,
}

fn build_client() -> Result<Client> {
    Client::builder()
        // One connection per VU: single idle slot and no cross-VU pooling
        .pool_max_idle_per_host(1)
        .http1_only()
        .tcp_nodelay(true)
        .no_brotli()
        .no_deflate()
        .no_gzip()
        .no_zstd()
        .redirect(Policy::none())
        .timeout(Duration::from_millis(2000))
        .tcp_keepalive(Some(Duration::from_secs(120)))
        .no_proxy()
        .build()
        .map_err(|e| Error::Other(e.to_string()))
}

pub async fn run_benchmark<F>(config: BenchmarkConfig, mut on_progress: Option<F>) -> Result<StatsSnapshot> 
where F: FnMut(StatsSnapshot) + Send + 'static
{
    let stats = Arc::new(Stats::new());
    let mut set = JoinSet::new();

    let start_time = Instant::now();
    let end_time = start_time + config.duration;
    let mut current_connections = 0;
    let vu_counter = Arc::new(AtomicU64::new(1));
    let mut rps_samples = Vec::new();
    let mut last_requests = 0;
    let mut last_sample_time = start_time;
    let mut first_progress_sent = false;
    
    // Main loop
    loop {
        let now = Instant::now();
        if now >= end_time {
            break;
        }
        let now = Instant::now();
        let elapsed = now.duration_since(start_time);
        
        // Calculate RPS sample
        {   
            let current_requests = stats.total_requests.load(Ordering::Relaxed);
            let sample_elapsed = now.duration_since(last_sample_time).as_secs_f64();
            if sample_elapsed >= 1.0 {
                let requests_diff = current_requests - last_requests;
                let rps = requests_diff as f64 / sample_elapsed;
                rps_samples.push(rps);
                last_requests = current_requests;
                last_sample_time = now;
            }
        }

        let ramp_up_secs = if let Some(ramp_up) = config.ramp_up {
            ramp_up.as_secs_f64()
        } else {
            (config.duration.as_secs_f64() - 1f64).max(1.0)
        };

        let progress = elapsed.as_secs_f64() / ramp_up_secs;
        let progress = progress.min(1.0);
        
        let target_connections = if let (Some(steps), Some(step_duration)) = (&config.step_connections, config.step_duration) {
            if steps.is_empty() {
                config.connections as f64
            } else if steps.len() == 1 {
                steps[0] as f64
            } else {
                let step_secs = step_duration.as_secs_f64();
                let total_secs = config.duration.as_secs_f64();
                let total_hold_time = steps.len() as f64 * step_secs;
                
                // Calculate available time for ramping
                let total_ramp_time = (total_secs - total_hold_time).max(0.0);
                let num_ramps = (steps.len() - 1) as f64;
                let ramp_secs = if num_ramps > 0.0 { total_ramp_time / num_ramps } else { 0.0 };
                
                let elapsed_secs = elapsed.as_secs_f64();
                let mut current_target = *steps.last().unwrap() as f64;
                
                // Find which cycle we are in
                for i in 0..steps.len() - 1 {
                    let cycle_start = i as f64 * (step_secs + ramp_secs);
                    let hold_end = cycle_start + step_secs;
                    let ramp_end = hold_end + ramp_secs;
                    
                    if elapsed_secs < hold_end {
                        current_target = steps[i] as f64;
                        break;
                    } else if elapsed_secs < ramp_end {
                        let progress = (elapsed_secs - hold_end) / ramp_secs;
                        let start_val = steps[i] as f64;
                        let end_val = steps[i+1] as f64;
                        current_target = start_val + (end_val - start_val) * progress;
                        break;
                    }
                }
                current_target
            }
        } else if config.connections >= config.start_connections {
             config.start_connections as f64 + (config.connections - config.start_connections) as f64 * progress
        } else {
             config.start_connections as f64
        };
        
        let target_connections = target_connections as u64;
        
        if current_connections < target_connections {
            let to_spawn = target_connections - current_connections;
            for _ in 0..to_spawn {
                let stats = stats.clone();
                let wrk_config = config.wrk.clone();
                let vu_id = vu_counter.fetch_add(1, Ordering::Relaxed);                
                set.spawn(async move {
                    stats.inc_connections();
                    let client = build_client().unwrap();
                    run_vu(wrk_config, stats, ExecutionMode::Duration(end_time), vu_id, client).await.unwrap();
                });
            }
            current_connections += to_spawn;
        }

        // Wait before sending first progress report to avoid zero stats
        if !first_progress_sent {
            sleep(Duration::from_secs(1)).await;
            first_progress_sent = true;
        }

        if let Some(ref mut cb) = on_progress {
            let snapshot = stats.snapshot(config.duration, Instant::now().duration_since(start_time), rps_samples.clone());
            cb(snapshot);
        }

        sleep(Duration::from_secs(1)).await;
    }
    
    while let Some(res) = set.join_next().await {
        match res {
            Ok(_) => {},
            Err(e) => {
                eprintln!("VU task failed: {}", e);
            }
        }
    }
    
    let snapshot = stats.snapshot(config.duration, config.duration, rps_samples);
    Ok(snapshot)
}

pub async fn run_once(config: WrkConfig) -> Result<StatsSnapshot> {
    let stats = Arc::new(Stats::new());
    let start = Instant::now();

    let client = build_client()?;

    run_vu(config.clone(), stats.clone(), ExecutionMode::Once, 1, client).await?;
    
    let elapsed = start.elapsed();
    Ok(stats.snapshot(elapsed, elapsed, Vec::new()))
}

async fn run_vu(config: WrkConfig, stats: Arc<Stats>, mode: ExecutionMode, vu_id: u64, client: Client) -> Result<()> {
    let (lua, ctx_ud) = create_lua_env(client, config.host_url.clone(), stats.clone(), vu_id)?;
    lua.load(&config.script_content).exec_async().await?;
    
    lua.set_named_registry_value("ctx_vars", lua.create_table()?)?;

    if let Ok(setup) = lua.globals().get::<Function>("setup") {
        setup.call_async::<()>(ctx_ud.clone()).await?;
    }
    
    let scenario_func: Function = match lua.globals().get("scenario") {
        Ok(f) => f,
        Err(_) => {
            return Err(Error::Other("Scenario function must be declared".to_owned()));
        }
    };

    let mut last_flush = Instant::now();

    loop {
        match mode {
            ExecutionMode::Duration(end_time) => {
                if Instant::now() >= end_time {
                    break;
                }
            },
            ExecutionMode::Once => {}
        }

        if last_flush.elapsed() > Duration::from_secs(1) {
            ctx_ud.flush_stats();
            last_flush = Instant::now();
        }

        match scenario_func.call_async::<()>(ctx_ud.clone()).await {
            Ok(_) => {},
            Err(e) => {
                let msg = e.to_string();
                let clean_msg = if let mlua::Error::CallbackError { cause, .. } = &e {
                    cause.to_string()
                } else if let mlua::Error::ExternalError(cause) = &e {
                    cause.to_string()
                } else {
                    msg.lines().next().unwrap_or(&msg).to_string()
                };
                
                let clean_msg = clean_msg.trim_start_matches("runtime error: ").to_string();
                stats.record_error(clean_msg);
            }
        }

        if let ExecutionMode::Once = mode {
            break;
        }
    }
    
    ctx_ud.flush_stats();

    // Run teardown(ctx)
    if let Ok(teardown) = lua.globals().get::<Function>("teardown") {
        teardown.call_async::<()>(ctx_ud.clone()).await?;
    }
    Ok(())
}
