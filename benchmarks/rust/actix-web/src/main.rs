use actix_files::Files;
use actix_web::{dev::Service as _, get, web, App, HttpResponse, HttpServer, Responder};
use mimalloc::MiMalloc;
use std::env;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[get("/")]
async fn hello_world() -> impl Responder {
    HttpResponse::Ok().body("Hello, World!")
}

#[get("/health")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = env::var("PORT").unwrap_or_else(|_| "8000".to_string());
    let port: u16 = port.parse().unwrap();
    
    let data_dir = env::var("DATA_DIR").unwrap_or_else(|_| "benchmarks_data".to_string());

    println!("Listening on 0.0.0.0:{}", port);

    HttpServer::new(move || {
        App::new()
            .wrap_fn(|req, srv| {
                let request_id = req.headers().get("x-request-id").cloned();
                let fut = srv.call(req);
                async move {
                    let mut res = fut.await?;
                    if let Some(request_id) = request_id {
                        res.headers_mut().insert(
                            actix_web::http::header::HeaderName::from_static("x-request-id"),
                            actix_web::http::header::HeaderValue::from_bytes(request_id.as_bytes()).unwrap(),
                        );
                    }
                    Ok(res)
                }
            })
            .service(hello_world)
            .service(health_check)
            .service(Files::new("/files", &data_dir))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
