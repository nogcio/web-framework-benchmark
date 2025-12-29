use actix_files::Files;
use actix_web::{dev::Service as _, get, web, App, HttpResponse, HttpServer, Responder};
use mimalloc::MiMalloc;
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
            .service(Files::new("/files", &data_dir))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
