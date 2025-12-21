use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::{env, net::SocketAddr, path::PathBuf};
use tokio::fs;

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    data_dir: String,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
struct HelloWorld {
    id: i32,
    name: String,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
}

#[derive(Deserialize)]
struct JsonPayload {
    #[serde(rename = "servlet-name")]
    servlet_name: Option<String>,
    #[serde(flatten)]
    extra: std::collections::HashMap<String, serde_json::Value>,
}

#[tokio::main]
async fn main() {
    let db_host = env::var("DB_HOST").unwrap_or_else(|_| "db".to_string());
    let db_port = env::var("DB_PORT").unwrap_or_else(|_| "5432".to_string());
    let db_user = env::var("DB_USER").unwrap_or_else(|_| "benchmark".to_string());
    let db_pass = env::var("DB_PASSWORD").unwrap_or_else(|_| "benchmark".to_string());
    let db_name = env::var("DB_NAME").unwrap_or_else(|_| "benchmark".to_string());
    let db_url = format!("postgres://{}:{}@{}:{}/{}", db_user, db_pass, db_host, db_port, db_name);

    let pool = PgPoolOptions::new()
        .max_connections(128)
        .connect(&db_url)
        .await
        .expect("Failed to connect to DB");

    let data_dir = env::var("DATA_DIR").unwrap_or_else(|_| "benchmarks_data".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8000".to_string());

    let state = AppState { pool, data_dir };

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/info", get(info))
        .route("/json/{from}/{to}", post(json_handler))
        .route("/db/read/one", get(db_read_one))
        .route("/db/read/many", get(db_read_many))
        .route("/db/write/insert", post(db_write_insert))
        .route("/files/*filename", get(static_files))
        .layer(axum::middleware::from_fn(request_id_middleware))
        .with_state(state);

    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    println!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn request_id_middleware(
    headers: HeaderMap,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let req_id = headers.get("x-request-id").cloned();
    let mut response = next.run(request).await;
    if let Some(req_id) = req_id {
        response.headers_mut().insert("x-request-id", req_id);
    }
    response
}

async fn root() -> &'static str {
    "Hello, World!"
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query("SELECT 1").execute(&state.pool).await {
        Ok(_) => (StatusCode::OK, "OK"),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "Service Unavailable"),
    }
}

async fn info() -> &'static str {
    "0.8.8,hello_world,json,db_read_one,db_read_paging,db_write,static_files"
}

async fn json_handler(
    Path((from, to)): Path<(String, String)>,
    Json(mut body): Json<serde_json::Value>,
) -> impl IntoResponse {
    traverse(&mut body, &from, &to);
    Json(body)
}

fn traverse(value: &mut serde_json::Value, from: &str, to: &str) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::String(s)) = map.get("servlet-name") {
                if s == from {
                    map.insert("servlet-name".to_string(), serde_json::Value::String(to.to_string()));
                }
            }
            for v in map.values_mut() {
                traverse(v, from, to);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                traverse(v, from, to);
            }
        }
        _ => {}
    }
}

#[derive(Deserialize)]
struct IdQuery {
    id: i32,
}

async fn db_read_one(
    State(state): State<AppState>,
    Query(query): Query<IdQuery>,
) -> impl IntoResponse {
    let row = sqlx::query_as::<_, HelloWorld>("SELECT * FROM hello_world WHERE id = $1")
        .bind(query.id)
        .fetch_optional(&state.pool)
        .await;

    match row {
        Ok(Some(row)) => Ok(Json(row)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(Deserialize)]
struct PagingQuery {
    offset: i32,
    limit: Option<i32>,
}

async fn db_read_many(
    State(state): State<AppState>,
    Query(query): Query<PagingQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(50);
    let rows = sqlx::query_as::<_, HelloWorld>("SELECT * FROM hello_world ORDER BY id LIMIT $1 OFFSET $2")
        .bind(limit)
        .bind(query.offset)
        .fetch_all(&state.pool)
        .await;

    match rows {
        Ok(rows) => Ok(Json(rows)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(Deserialize)]
struct InsertBody {
    name: String,
}

#[derive(Deserialize)]
struct InsertQuery {
    name: Option<String>,
}

async fn db_write_insert(
    State(state): State<AppState>,
    Query(query): Query<InsertQuery>,
    body: Option<Json<InsertBody>>,
) -> impl IntoResponse {
    let name = if let Some(n) = query.name {
        Some(n)
    } else if let Some(Json(b)) = body {
        Some(b.name)
    } else {
        None
    };

    let name = match name {
        Some(n) => n,
        None => return Err((StatusCode::BAD_REQUEST, "Missing name")),
    };

    let row = sqlx::query_as::<_, HelloWorld>(
        "INSERT INTO hello_world (name, created_at, updated_at) VALUES ($1, NOW(), NOW()) RETURNING *"
    )
    .bind(name)
    .fetch_one(&state.pool)
    .await;

    match row {
        Ok(row) => Ok(Json(row)),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "DB Error")),
    }
}

async fn static_files(
    State(state): State<AppState>,
    Path(filename): Path<String>,
) -> impl IntoResponse {
    // Security check
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err(StatusCode::FORBIDDEN);
    }

    let path = PathBuf::from(&state.data_dir).join(filename);
    
    match fs::read(path).await {
        Ok(content) => {
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));
            Ok((headers, content))
        },
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}
