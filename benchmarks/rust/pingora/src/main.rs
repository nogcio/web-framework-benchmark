use async_trait::async_trait;
use bytes::Bytes;
use pingora::apps::{HttpPersistentSettings, HttpServerApp, ReusedHttpStream};
use pingora::http::ResponseHeader;
use pingora::protocols::http::ServerSession;
use pingora::server::configuration::ServerConf;
use pingora::server::ShutdownWatch;
use pingora::services::listening::Service;
use pingora::prelude::Server;
use std::sync::Arc;
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn build_header(status: u16, len: usize) -> ResponseHeader {
    let mut header = ResponseHeader::build(status, None).unwrap();
    header.set_content_length(len).unwrap();
    header.insert_header("content-type", "text/plain").unwrap();
    header
}

pub struct HelloServer {
    hello_body: Bytes,
    health_body: Bytes,
    not_found_body: Bytes,
    hello_hdr: ResponseHeader,
    health_hdr: ResponseHeader,
    not_found_hdr: ResponseHeader,
}

impl HelloServer {
    pub fn new() -> Self {
        let hello_body = Bytes::from_static(b"Hello, World!");
        let health_body = Bytes::from_static(b"OK");
        let not_found_body = Bytes::from_static(b"Not Found");

        let hello_hdr = build_header(200, hello_body.len());
        let health_hdr = build_header(200, health_body.len());
        let not_found_hdr = build_header(404, not_found_body.len());

        Self {
            hello_body,
            health_body,
            not_found_body,
            hello_hdr,
            health_hdr,
            not_found_hdr,
        }
    }
}

#[async_trait]
impl HttpServerApp for HelloServer {
    async fn process_new_http(
        self: &Arc<Self>,
        mut http: ServerSession,
        shutdown: &ShutdownWatch,
    ) -> Option<ReusedHttpStream> {
        match http.read_request().await.ok()? {
            true => {}
            false => return None,
        }

        if *shutdown.borrow() {
            http.set_keepalive(None);
        } else {
            http.set_keepalive(Some(60));
        }

        let path = http.req_header().uri.path();
        let (header, body) = if path == "/" {
            (&self.hello_hdr, self.hello_body.clone())
        } else if path == "/health" {
            (&self.health_hdr, self.health_body.clone())
        } else {
            (&self.not_found_hdr, self.not_found_body.clone())
        };

        http.write_response_header_ref(header).await.ok()?;
        http.write_response_body(body, true).await.ok()?;

        let persistent_settings = HttpPersistentSettings::for_session(&http);
        http.finish()
            .await
            .ok()?
            .map(|stream| ReusedHttpStream::new(stream, Some(persistent_settings)))
    }
}

fn main() {
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let mut conf = ServerConf::default();
    conf.threads = threads;
    conf.listener_tasks_per_fd = 1;

    let mut my_server = Server::new_with_opt_and_conf(None, conf);
    my_server.bootstrap();

    let mut service = Service::new("pingora_http".to_string(), HelloServer::new());
    service.add_tcp("0.0.0.0:8000");
    my_server.add_service(service);
    my_server.run_forever();
}
