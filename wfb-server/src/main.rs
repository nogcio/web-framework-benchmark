#![deny(clippy::expect_used, clippy::unwrap_used)]

use anyhow::bail;
use axum::Router;
use clap::Parser;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracing::info;

use wfb_storage::{Config, Storage};

mod api_models;
mod assets_manifest;
mod file_watcher;
mod filters;
mod handlers;
mod middleware;
mod public_url;
mod routes;
mod state;
mod view_models;

use file_watcher::{FileChangeEvent, FileWatcherService};
use routes::build_app;
use state::AppState;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Host to bind to
    #[arg(long, env = "HOST", default_value = "127.0.0.1")]
    host: String,

    /// Port to bind to
    #[arg(long, env = "PORT", default_value_t = 8080)]
    port: u16,

    /// Directory that contains static assets (CSS, JS, images)
    #[arg(long, env = "ASSETS_DIR")]
    assets_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .with_env_filter(
            match tracing_subscriber::EnvFilter::try_from_default_env() {
                Ok(filter) => filter,
                Err(_) => tracing_subscriber::EnvFilter::new("info"),
            },
        )
        .init();
    let args = Args::parse();

    let data_path = std::path::PathBuf::from("./data");
    let config_path = std::path::PathBuf::from("./config");

    let storage = Arc::new(Storage::new(&data_path)?);
    let config = Arc::new(RwLock::new(Config::load(&config_path)?));

    let state = Arc::new(AppState {
        storage: storage.clone(),
        config: config.clone(),
    });

    let assets_dir = resolve_assets_dir(args.assets_dir.clone())?;
    info!("Serving assets from {}", assets_dir.display());

    let (mut watcher, mut rx) = FileWatcherService::new(&config_path, &data_path)?;
    watcher.watch(&config_path)?;
    watcher.watch(&data_path)?;

    let storage_clone = storage.clone();
    let config_clone = config.clone();
    let config_path_clone = config_path.clone();
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                FileChangeEvent::ConfigChanged => {
                    tracing::info!("Config changed, reloading...");
                    tokio::time::sleep(Duration::from_millis(500)).await;

                    let mut config_guard =
                        config_clone.write().unwrap_or_else(|err| err.into_inner());
                    if let Err(e) = config_guard.reload(&config_path_clone) {
                        tracing::error!("Failed to reload config: {}", e);
                    } else {
                        tracing::info!("Config reloaded successfully");
                    }
                }
                FileChangeEvent::DataChanged => {
                    tracing::info!("Data changed, reloading...");
                    tokio::time::sleep(Duration::from_millis(500)).await;

                    if let Err(e) = storage_clone.reload() {
                        tracing::error!("Failed to reload data: {}", e);
                    } else {
                        tracing::info!("Data reloaded successfully");
                    }
                }
            }
        }
    });

    let app: Router = build_app(state, assets_dir);

    let addr = format!("{}:{}", args.host, args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

fn resolve_assets_dir(explicit: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(path) = explicit {
        if path.is_dir() {
            return Ok(path);
        } else {
            bail!("Assets directory {:?} does not exist", path);
        }
    }

    let mut candidates: Vec<PathBuf> = Vec::new();

    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/dist"));

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("assets/dist"));
        candidates.push(cwd.join("assets"));
    }

    if let Ok(exe_path) = std::env::current_exe()
        && let Some(exe_dir) = exe_path.parent()
    {
        candidates.push(exe_dir.join("assets/dist"));
        candidates.push(exe_dir.join("assets"));
    }

    for candidate in candidates {
        if candidate.is_dir() {
            return Ok(candidate);
        }
    }

    bail!(
        "Unable to locate assets directory. Provide --assets-dir CLI flag or set ASSETS_DIR env variable."
    );
}
