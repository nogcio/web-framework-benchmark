use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::routing::get;
use axum_extra::routing::TypedPath;
use serde::Deserialize;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::services::ServeDir;

use crate::handlers::{api, web};
use crate::middleware;
use crate::state::AppState;

macro_rules! define_routes {
    (
        $(
            $vis:vis $name:ident => $path:literal $( { $( $field:ident : $fty:ty ),* $(,)? } )? $( [tpl $tpl_name:ident] )? ;
        )*
    ) => {
        $(
            define_routes!(@struct $vis $name => $path $( { $( $field : $fty ),* } )? );
        )*

        // --- Template routing helper (Askama) ---
        //
        // Askama templates can't construct Rust structs directly, so we expose a small
        // helper object in the template context.
        //
        // This module is generated from the same route list as the TypedPath structs,
        // so fields are defined exactly once.
        pub mod tpl {
            use axum_extra::routing::TypedPath;
            use super::*;

            #[derive(Debug, Clone, Copy, Default)]
            pub struct Routes;

            impl Routes {
                $(
                    define_routes!(@tpl $( [tpl $tpl_name] )? $name $( { $( $field : $fty ),* } )? );
                )*
            }
        }
    };

    (@struct $vis:vis $name:ident => $path:literal) => {
        #[derive(Debug, Clone, TypedPath)]
        #[typed_path($path)]
        $vis struct $name;
    };

    (@struct $vis:vis $name:ident => $path:literal { $( $field:ident : $fty:ty ),* $(,)? }) => {
        #[derive(Debug, Clone, Deserialize, TypedPath)]
        #[typed_path($path)]
        $vis struct $name {
            $(
                $vis $field: $fty,
            )*
        }
    };

    // Template helper generation: no tpl marker => emit nothing.
    (@tpl $name:ident) => {};
    (@tpl $name:ident { $( $field:ident : $fty:ty ),* $(,)? }) => {};

    // Template helper generation: unit route.
    (@tpl [tpl $tpl_name:ident] $name:ident) => {
        pub fn $tpl_name(&self) -> String {
            $name.to_uri().to_string()
        }
    };

    // Template helper generation: route with fields. We intentionally use &str
    // for template ergonomics and convert into owned Strings.
    (@tpl [tpl $tpl_name:ident] $name:ident { $( $field:ident : $fty:ty ),* $(,)? }) => {
        pub fn $tpl_name(&self, $( $field: &str ),* ) -> String {
            $name {
                $( $field: $field.to_string(), )*
            }
            .to_uri()
            .to_string()
        }
    };
}

define_routes! {
    // --- Web routes ---
    pub IndexRoot => "/";

    pub GithubStarsPartials => "/partials/github/stars" [tpl github_stars_url];

    pub IndexViewPath => "/runs/{run}/env/{env}/test/{test}" {
        run: String,
        env: String,
        test: String,
    } [tpl index_url];

    pub IndexPartialsViewPath => "/partials/runs/{run}/env/{env}/test/{test}" {
        run: String,
        env: String,
        test: String,
    } [tpl index_partials_url];

    pub BenchViewPath => "/runs/{run}/env/{env}/test/{test}/bench/{framework}" {
        run: String,
        env: String,
        test: String,
        framework: String,
    } [tpl bench_url];

    pub BenchChartsPartialsViewPath => "/partials/runs/{run}/env/{env}/test/{test}/bench/{framework}/charts" {
        run: String,
        env: String,
        test: String,
        framework: String,
    } [tpl bench_charts_partials_url];

    // --- API routes ---
    pub ApiVersion => "/api/version";
    pub ApiTags => "/api/tags";
    pub ApiEnvironments => "/api/environments";
    pub ApiTests => "/api/tests";
    pub ApiLanguages => "/api/languages";
    pub ApiFrameworks => "/api/frameworks";
    pub ApiBenchmarks => "/api/benchmarks";
    pub ApiRuns => "/api/runs";

    pub ApiRunResultsPath => "/api/runs/{run_id}/environments/{env}/tests/{test}" {
        run_id: String,
        env: String,
        test: String,
    };

    pub ApiRunRawPath => "/api/runs/{run_id}/environments/{env}/tests/{test}/frameworks/{framework}/raw" {
        run_id: String,
        env: String,
        test: String,
        framework: String,
    };
}

/// Build the full Axum app (routes + middleware + static assets fallback).
///
/// Centralizes route registration to keep URL patterns and reverse routing consistent.
pub fn build_app(state: Arc<AppState>, assets_dir: PathBuf) -> Router {
    let cors_layer = cors_layer_from_env();
    let app = Router::new()
        // Web
        .route(IndexRoot::PATH, get(web::root_handler))
        .route(IndexViewPath::PATH, get(web::index_path_handler))
        .route(
            IndexPartialsViewPath::PATH,
            get(web::index_update_path_handler),
        )
        .route(
            GithubStarsPartials::PATH,
            get(web::github_stars_partials_handler),
        )
        .route(BenchViewPath::PATH, get(web::bench_path_handler))
        .route(
            BenchChartsPartialsViewPath::PATH,
            get(web::bench_charts_partials_path_handler),
        )
        // API
        .route(ApiTags::PATH, get(api::get_tags))
        .route(ApiEnvironments::PATH, get(api::get_environments))
        .route(ApiTests::PATH, get(api::get_tests))
        .route(ApiLanguages::PATH, get(api::get_languages))
        .route(ApiFrameworks::PATH, get(api::get_frameworks))
        .route(ApiBenchmarks::PATH, get(api::get_benchmarks))
        .route(ApiRuns::PATH, get(api::get_runs))
        .route(ApiVersion::PATH, get(api::get_version))
        .route(ApiRunResultsPath::PATH, get(api::get_run_results))
        .route(ApiRunRawPath::PATH, get(api::get_run_raw_data))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(axum::middleware::from_fn(middleware::security_headers))
                .layer(CompressionLayer::new())
                .layer(cors_layer),
        );

    let static_service = ServiceBuilder::new()
        .layer(axum::middleware::from_fn(middleware::security_headers))
        .layer(CompressionLayer::new())
        .layer(axum::middleware::from_fn(middleware::static_cache_control))
        .service(ServeDir::new(assets_dir).append_index_html_on_directories(false));

    app.fallback_service(static_service)
}

fn cors_layer_from_env() -> CorsLayer {
    // For a public site, permissive CORS is typically not desired.
    //
    // Configure explicitly via:
    // - WFB_CORS_ALLOW_ORIGINS="https://example.com,https://other.com"
    // - WFB_CORS_ALLOW_ORIGINS="*" (not recommended for public prod)
    let raw = std::env::var("WFB_CORS_ALLOW_ORIGINS").unwrap_or_default();
    let raw = raw.trim();
    if raw.is_empty() {
        // Default: do not emit CORS headers.
        return CorsLayer::new();
    }

    if raw == "*" {
        return CorsLayer::new().allow_origin(Any);
    }

    {
        let mut origins: Vec<axum::http::HeaderValue> = Vec::new();
        for part in raw.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            if let Ok(hv) = axum::http::HeaderValue::from_str(part) {
                origins.push(hv);
            }
        }

        if origins.is_empty() {
            return CorsLayer::new();
        }

        CorsLayer::new().allow_origin(AllowOrigin::list(origins))
    }
}
