use axum::http::header::HeaderName;
use axum::http::{HeaderValue, Request, Uri, header};
use axum::middleware::Next;
use rand::RngCore;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct CspNonce(pub String);

pub async fn security_headers(
    mut req: Request<axum::body::Body>,
    next: Next,
) -> axum::response::Response {
    let is_https = request_is_https(&req);
    let nonce = generate_nonce_hex_16();
    req.extensions_mut().insert(CspNonce(nonce.clone()));

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

    // For public deployments, enable stricter web security. Keep this opt-in to avoid breaking local
    // development workflows.
    //
    // Env flags:
    // - WFB_PUBLIC=1: enable CSP (enforced), HSTS, and CORP
    // - WFB_CSP_REPORT_ONLY=1: use Content-Security-Policy-Report-Only instead of enforcement
    let mode = security_mode();
    if mode.public {
        let report_only = mode.csp_report_only;

        let upgrade_insecure = if is_https {
            "; upgrade-insecure-requests"
        } else {
            ""
        };

        // NOTE: We intentionally allow inline styles because templates rely on style attributes
        // (e.g. chart masks, table row gradients). Script inline is nonce-gated.
        let csp = format!(
            "default-src 'self'; base-uri 'none'; object-src 'none'; frame-ancestors 'none'; form-action 'self'; img-src 'self' data:; font-src 'self' data:; connect-src 'self'; script-src 'self' 'nonce-{}'; style-src 'self' 'unsafe-inline'{}",
            nonce, upgrade_insecure
        );

        let csp_header = if report_only {
            HeaderName::from_static("content-security-policy-report-only")
        } else {
            HeaderName::from_static("content-security-policy")
        };
        if let Ok(value) = HeaderValue::from_str(&csp) {
            headers.insert(csp_header, value);
        }

        // Only meaningful behind HTTPS. When running behind a reverse proxy, this will be set
        // if it forwards `X-Forwarded-Proto: https` or `Forwarded: proto=https`.
        if is_https {
            headers.insert(
                HeaderName::from_static("strict-transport-security"),
                HeaderValue::from_static("max-age=31536000; includeSubDomains"),
            );
        }

        // Prevent other origins from embedding our resources.
        headers.insert(
            HeaderName::from_static("cross-origin-resource-policy"),
            HeaderValue::from_static("same-origin"),
        );
    }

    resp
}

#[derive(Debug, Clone, Copy)]
struct SecurityMode {
    public: bool,
    csp_report_only: bool,
}

impl SecurityMode {
    fn from_env() -> Self {
        Self {
            public: std::env::var("WFB_PUBLIC").as_deref() == Ok("1"),
            csp_report_only: std::env::var("WFB_CSP_REPORT_ONLY").as_deref() == Ok("1"),
        }
    }
}

fn security_mode() -> &'static SecurityMode {
    static MODE: OnceLock<SecurityMode> = OnceLock::new();
    MODE.get_or_init(SecurityMode::from_env)
}

fn generate_nonce_hex_16() -> String {
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let mut out = String::with_capacity(32);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(&mut out, "{:02x}", b);
    }
    out
}

fn request_is_https<B>(req: &Request<B>) -> bool {
    if req.uri().scheme_str() == Some("https") {
        return true;
    }

    // Common reverse proxy header.
    if let Some(v) = req.headers().get("x-forwarded-proto") {
        if let Ok(s) = v.to_str() {
            // Can be a comma-separated list (client, proxy1, proxy2).
            if s.split(',').any(|p| p.trim().eq_ignore_ascii_case("https")) {
                return true;
            }
        }
    }

    // RFC 7239 Forwarded: for=...;proto=https;host=...
    if let Some(v) = req.headers().get("forwarded") {
        if let Ok(s) = v.to_str() {
            if s.split(';')
                .map(|part| part.trim())
                .any(|part| part.eq_ignore_ascii_case("proto=https"))
            {
                return true;
            }
        }
    }

    false
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
