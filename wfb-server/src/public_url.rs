use std::sync::OnceLock;

/// Returns the public base URL used for canonical links and sitemap generation.
///
/// Priority:
/// 1) `WFB_PUBLIC_BASE_URL` (recommended for prod), e.g. `https://wfb.nogc.io`
/// 2) `PUBLIC_BASE_URL`
/// 3) local fallback from `HOST`/`PORT`
///
/// The returned value never ends with `/`.
pub fn public_base_url() -> &'static str {
    static BASE: OnceLock<String> = OnceLock::new();
    BASE.get_or_init(|| {
        let raw = std::env::var("WFB_PUBLIC_BASE_URL")
            .or_else(|_| std::env::var("PUBLIC_BASE_URL"))
            .unwrap_or_else(|_| {
                let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
                let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
                format!("http://{}:{}", host, port)
            });

        raw.trim().trim_end_matches('/').to_string()
    })
    .as_str()
}

/// Joins the public base URL with an absolute ("/path") or relative ("path") page path.
pub fn page_url(page_path: &str) -> String {
    join(public_base_url(), page_path)
}

pub fn join(site_base_url: &str, page_path: &str) -> String {
    if site_base_url.is_empty() {
        return page_path.to_string();
    }

    if page_path.starts_with('/') {
        format!("{}{}", site_base_url, page_path)
    } else {
        format!("{}/{}", site_base_url, page_path)
    }
}
