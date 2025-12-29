use axum::{
    extract::{Path, Request},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use tower_http::services::ServeDir;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, net::SocketAddr, sync::OnceLock};
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

static DATA_DIR: OnceLock<String> = OnceLock::new();

#[tokio::main]
async fn main() {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port).parse::<SocketAddr>().unwrap();

    DATA_DIR.get_or_init(|| env::var("DATA_DIR").unwrap_or_else(|_| "benchmarks_data".to_string()));

    let app = Router::new()
        .route("/", get(hello_world))
        .route("/plaintext", get(hello_world))
        .route("/health", get(health_check))
        .route("/json/{from}/{to}", post(json_handler))
        .nest_service("/files", ServeDir::new(DATA_DIR.get().unwrap()));

    println!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}



async fn hello_world() -> &'static str {
    "Hello, World!"
}

async fn health_check() -> &'static str {
    "OK"
}

#[derive(Deserialize, Serialize)]
struct Payload {
    #[serde(rename = "web-app")]
    web_app: WebApp,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize, Serialize)]
struct WebApp {
    #[serde(rename = "servlet")]
    servlets: Vec<Servlet>,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize, Serialize)]
struct Servlet {
    #[serde(rename = "servlet-name")]
    servlet_name: String,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

async fn json_handler(
    Path((from, to)): Path<(String, String)>,
    Json(mut payload): Json<Payload>,
) -> impl IntoResponse {
    for servlet in &mut payload.web_app.servlets {
        if servlet.servlet_name == from {
            servlet.servlet_name = to.clone();
        }
    }
    Json(payload)
}
