use crate::stats::StatsSnapshot;
use crate::{BenchmarkConfig, WrkConfig, error::*, stats::Stats, lua_env::create_lua_env};
use std::sync::Arc;
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
    
    // Main loop
    while Instant::now() < end_time {
        let now = Instant::now();
        let elapsed = now.duration_since(start_time);
        
        let ramp_up_secs = if let Some(ramp_up) = config.ramp_up {
            ramp_up.as_secs_f64()
        } else {
            (config.duration.as_secs_f64() - 1f64).max(1.0)
        };

        let progress = elapsed.as_secs_f64() / ramp_up_secs;
        let progress = progress.min(1.0);
        
        let target_connections = if config.connections >= config.start_connections {
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
                let client = build_client()?;
                                               
                set.spawn(async move {
                    stats.inc_connections();
                    run_vu(wrk_config, stats, ExecutionMode::Duration(end_time), vu_id, client).await.unwrap();
                });
            }
            current_connections += to_spawn;
        }

        if let Some(ref mut cb) = on_progress {
            let snapshot = stats.snapshot(config.duration, Instant::now().duration_since(start_time));
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
    
    let snapshot = stats.snapshot(config.duration, config.duration);
    
    Ok(snapshot)
}

pub async fn run_once(config: WrkConfig) -> Result<StatsSnapshot> {
    let stats = Arc::new(Stats::new());
    let start = Instant::now();

    let client = build_client()?;

    run_vu(config.clone(), stats.clone(), ExecutionMode::Once, 1, client).await?;
    
    let elapsed = start.elapsed();
    Ok(stats.snapshot(elapsed, elapsed))
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
