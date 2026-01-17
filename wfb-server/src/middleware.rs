use axum::http::header::HeaderName;
use axum::http::{HeaderValue, Request, Uri, header};
use axum::middleware::Next;

pub async fn security_headers(
    req: Request<axum::body::Body>,
    next: Next,
) -> axum::response::Response {
    let mut resp = next.run(req).await;

    // Set a baseline of safe headers. (Avoid CSP/HSTS here; those need deployment-specific tuning.)
    let headers = resp.headers_mut();

    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static(
            "accelerometer=(), autoplay=(), camera=(), clipboard-read=(), clipboard-write=(), geolocation=(), gyroscope=(), microphone=(), payment=(), usb=()",
        ),
    );
    headers.insert(
        HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin"),
    );

    resp
}

pub async fn static_cache_control(
    req: Request<axum::body::Body>,
    next: Next,
) -> axum::response::Response {
    let uri: Uri = req.uri().clone();
    let mut resp = next.run(req).await;

    if resp.status().is_success() {
        let value = if uri.path() == "/images/logo.svg" || uri.path() == "/images/preview.png" {
            // Keep this URL stable for OG/meta caches; allow it to be cached.
            "public, max-age=604800"
        } else if is_versioned_asset(&uri) {
            "public, max-age=31536000, immutable"
        } else {
            // Strict default: the app should only reference fingerprinted assets.
            "no-store"
        };

        resp.headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static(value));
    }

    resp
}

fn is_versioned_asset(uri: &Uri) -> bool {
    if let Some(q) = uri.query() {
        // Matches current "?q=..." cache buster as well as any future query-based versioning.
        if q.split('&').any(|kv| kv.starts_with("q=")) {
            return true;
        }
    }

    let path = uri.path();
    let Some(file) = path.rsplit('/').next() else {
        return false;
    };

    // Treat "name.<hexhash>.ext" as versioned.
    // Example: app.a1b2c3d4e5.css
    let parts = file.split('.').collect::<Vec<_>>();
    if parts.len() < 3 {
        return false;
    }

    let hash = parts
        .get(parts.len().saturating_sub(2))
        .copied()
        .unwrap_or("");
    hash.len() >= 8 && hash.chars().all(|c| c.is_ascii_hexdigit())
}
