use actix_files::Files;
use actix_web::{dev::Service as _, get, post, web, App, HttpResponse, HttpServer, Responder};
use mimalloc::MiMalloc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[get("/")]
async fn hello_world() -> impl Responder {
    HttpResponse::Ok().content_type("text/plain").body("Hello, World!")
}

#[get("/plaintext")]
async fn plaintext() -> impl Responder {
    HttpResponse::Ok().content_type("text/plain").body("Hello, World!")
}

#[get("/health")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

#[derive(Deserialize)]
struct OrderItem {
    quantity: i32,
    category: String,
}

#[derive(Deserialize)]
struct Order {
    status: String,
    amount: i64,
    country: String,
    items: Vec<OrderItem>,
}

#[derive(Serialize)]
struct AggregateResponse {
    #[serde(rename = "processedOrders")]
    processed_orders: usize,
    results: HashMap<String, i64>,
    #[serde(rename = "categoryStats")]
    category_stats: HashMap<String, i64>,
}

#[post("/json/aggregate")]
async fn json_aggregate(orders: web::Json<Vec<Order>>) -> impl Responder {
    let mut processed_orders = 0;
    let mut results = HashMap::new();
    let mut category_stats = HashMap::new();

    for order in orders.iter() {
        if order.status == "completed" {
            processed_orders += 1;
            *results.entry(order.country.clone()).or_insert(0) += order.amount;
            
            for item in &order.items {
                *category_stats.entry(item.category.clone()).or_insert(0) += item.quantity as i64;
            }
        }
    }

    HttpResponse::Ok().json(AggregateResponse {
        processed_orders,
        results,
        category_stats,
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let port: u16 = port.parse().unwrap();
    
    let data_dir = env::var("DATA_DIR").unwrap_or_else(|_| "benchmarks_data".to_string());

    println!("Listening on 0.0.0.0:{}", port);

    HttpServer::new(move || {
        App::new()
            .service(hello_world)
            .service(plaintext)
            .service(health_check)
            .service(json_aggregate)
            .service(Files::new("/files", &data_dir))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
