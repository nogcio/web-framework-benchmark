use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::IntoResponse;
use axum_extra::routing::TypedPath;
use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use std::sync::Arc;
use tracing::error;

use crate::handlers::web::helpers::get_available_tests;
use crate::public_url;
use crate::routes;
use crate::state::AppState;

pub async fn robots_txt_handler(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    let sitemap = public_url::join(public_url::public_base_url(), routes::SitemapXml::PATH);

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );

    let body = format!("User-agent: *\nAllow: /\n\nSitemap: {}\n", sitemap);

    (headers, body)
}

pub async fn sitemap_xml_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let data = state.storage.data_read();
    let tests = get_available_tests();

    let mut urls: Vec<String> = Vec::new();

    // Root.
    urls.push(public_url::page_url(routes::IndexRoot::PATH));

    // Results pages (runs/env/test).
    for (run_id, run_data) in data.iter() {
        for env_name in run_data.keys() {
            for test in &tests {
                let path = routes::IndexViewPath {
                    run: run_id.clone(),
                    env: env_name.clone(),
                    test: test.id.clone(),
                }
                .to_uri()
                .to_string();

                urls.push(public_url::page_url(&path));
            }
        }
    }

    urls.sort();
    urls.dedup();

    let xml = match build_sitemap_xml(&urls) {
        Ok(xml) => xml,
        Err(e) => {
            error!("Failed to build sitemap.xml: {e}");
            // Keep response simple; errors should be visible in logs.
            String::new()
        }
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/xml; charset=utf-8"),
    );

    (headers, xml)
}

fn build_sitemap_xml(urls: &[String]) -> anyhow::Result<String> {
    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

    let mut urlset = BytesStart::new("urlset");
    urlset.push_attribute(("xmlns", "http://www.sitemaps.org/schemas/sitemap/0.9"));
    writer.write_event(Event::Start(urlset))?;

    for url in urls {
        writer.write_event(Event::Start(BytesStart::new("url")))?;
        writer.write_event(Event::Start(BytesStart::new("loc")))?;
        writer.write_event(Event::Text(BytesText::new(url)))?;
        writer.write_event(Event::End(BytesEnd::new("loc")))?;
        writer.write_event(Event::End(BytesEnd::new("url")))?;
    }

    writer.write_event(Event::End(BytesEnd::new("urlset")))?;

    let bytes = writer.into_inner();
    Ok(String::from_utf8(bytes)?)
}
