use axum::{
    body::Body,
    extract::{Path, Request},
    http::{HeaderName, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, net::SocketAddr, sync::OnceLock};
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

static DATA_DIR: OnceLock<String> = OnceLock::new();

#[tokio::main]
async fn main() {
    let port = env::var("PORT").unwrap_or_else(|_| "8000".to_string());
    let addr = format!("0.0.0.0:{}", port).parse::<SocketAddr>().unwrap();

    DATA_DIR.get_or_init(|| env::var("DATA_DIR").unwrap_or_else(|_| "benchmarks_data".to_string()));

    let app = Router::new()
        .route("/", get(hello_world))
        .route("/health", get(health_check))
        .route("/json/{from}/{to}", post(json_handler))
        .route("/files/{*filename}", get(file_handler))
        .layer(middleware::from_fn(x_request_id_middleware));

    println!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn file_handler(Path(filename): Path<String>) -> impl IntoResponse {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return StatusCode::FORBIDDEN.into_response();
    }

    let path = std::path::Path::new(DATA_DIR.get().unwrap()).join(&filename);

    match tokio::fs::File::open(path).await {
        Ok(file) => {
            let size = match file.metadata().await {
                Ok(metadata) => metadata.len(),
                Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            };

            let stream = tokio_util::io::ReaderStream::with_capacity(file, 1024 * 128);
            let body = Body::from_stream(stream);

            let mut response = body.into_response();
            response.headers_mut().insert(
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static("application/octet-stream"),
            );
            response.headers_mut().insert(
                axum::http::header::CONTENT_LENGTH,
                axum::http::HeaderValue::from_str(&size.to_string()).unwrap(),
            );
            response
        }
        Err(err) => {
            if err.kind() == std::io::ErrorKind::NotFound {
                StatusCode::NOT_FOUND.into_response()
            } else {
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

async fn x_request_id_middleware(req: Request, next: Next) -> Response {
    let request_id = req.headers().get("x-request-id").cloned();
    let mut response = next.run(req).await;

    if let Some(request_id) = request_id {
        response.headers_mut().insert("x-request-id", request_id);
    }

    response
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
