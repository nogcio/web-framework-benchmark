use mimalloc::MiMalloc;
use std::collections::HashMap;
use rustc_hash::FxHashMap;
use tonic::{transport::Server, Request, Response, Status};

pub mod analytics {
    tonic::include_proto!("_");
}

use analytics::analytics_service_server::{AnalyticsService, AnalyticsServiceServer};
use analytics::{AggregateResult, OrderStatus, AnalyticsRequest};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Debug, Default)]
pub struct HelperService;

#[tonic::async_trait]
impl AnalyticsService for HelperService {
    async fn aggregate_orders(
        &self,
        request: Request<AnalyticsRequest>,
    ) -> Result<Response<AggregateResult>, Status> {
        let client_id = match request.metadata().get("x-client-id") {
            Some(v) => v.to_str().unwrap_or("").to_string(),
            None => {
                // println!("Error: x-client-id header missing. Headers: {:?}", request.metadata());
                "MISSING_HEADER".to_string()
            }
        };

        let req = request.into_inner();

        let mut processed_orders = 0;
        let mut amount_by_country: FxHashMap<String, i64> = FxHashMap::default();
        let mut quantity_by_category: FxHashMap<String, i32> = FxHashMap::default();

        for order in req.orders {
            if order.status == OrderStatus::Completed as i32 {
                processed_orders += 1;
                
                let mut order_amount = 0;
                for item in order.items {
                     order_amount += item.price_cents * item.quantity as i64;
                     
                     *quantity_by_category.entry(item.category).or_insert(0) += item.quantity;
                }
                
                *amount_by_country.entry(order.country).or_insert(0) += order_amount;
            }
        }

        let reply = AggregateResult {
            processed_orders,
            amount_by_country: amount_by_country.into_iter().collect(),
            quantity_by_category: quantity_by_category.into_iter().collect(),
            echoed_client_id: client_id,
        };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port).parse()?;
    let service = HelperService::default();
    
    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<AnalyticsServiceServer<HelperService>>()
        .await;

    println!("AnalyticsService listening on {}", addr);

    Server::builder()
        .initial_stream_window_size(Some(1 * 1024 * 1024)) // 1MB
        .initial_connection_window_size(Some(10 * 1024 * 1024)) // 10MB
        .http2_keepalive_interval(None)
        .concurrency_limit_per_connection(256)
        .add_service(health_service)
        .add_service(
            AnalyticsServiceServer::new(service),
        )
        .serve(addr)
        .await?;

    Ok(())
}
