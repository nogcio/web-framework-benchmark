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
        .route("/json/aggregate", post(json_aggregate))
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

#[derive(Deserialize)]
struct Order {
    status: String,
    amount: i64,
    country: String,
    items: Option<Vec<OrderItem>>,
}

#[derive(Deserialize)]
struct OrderItem {
    quantity: i32,
    category: String,
}

#[derive(Serialize)]
struct AggregateResponse {
    #[serde(rename = "processedOrders")]
    processed_orders: usize,
    results: HashMap<String, i64>,
    #[serde(rename = "categoryStats")]
    category_stats: HashMap<String, i32>,
}

async fn json_aggregate(
    Json(orders): Json<Vec<Order>>,
) -> impl IntoResponse {
    let mut processed_orders = 0;
    let mut results = HashMap::new();
    let mut category_stats = HashMap::new();

    for order in orders {
        if order.status == "completed" {
            processed_orders += 1;
            *results.entry(order.country).or_insert(0) += order.amount;
            
            if let Some(items) = &order.items {
                for item in items {
                    *category_stats.entry(item.category.clone()).or_insert(0) += item.quantity;
                }
            }
        }
    }

    Json(AggregateResponse {
        processed_orders,
        results,
        category_stats,
    })
}
