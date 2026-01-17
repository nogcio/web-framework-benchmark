use super::context::select_common;
use super::github::github_stars_value_string;
use super::helpers::benchmark_repo_url;
use super::render::HtmlTemplate;
use super::types::{BenchDetailView, ChromeContext, Routes, SelectionContext};
use askama::Template;
use axum::extract::State;
use axum::response::{IntoResponse, Redirect};
use axum_extra::routing::TypedPath;
use std::sync::Arc;
use std::time::Instant;

use crate::handlers::web::context::chrome_context;
use crate::routes;
use crate::state::AppState;

#[allow(unused_imports)]
use crate::filters;

#[derive(Template)]
#[template(path = "pages/bench.rs.j2")]
struct BenchTemplate {
    chrome: ChromeContext,
    selection: SelectionContext,
    bench: Option<BenchDetailView>,
    routes: Routes,
}

pub async fn bench_path_handler(
    State(state): State<Arc<AppState>>,
    params: routes::BenchViewPath,
) -> impl IntoResponse {
    bench_render(state, params).await
}

async fn bench_render(
    state: Arc<AppState>,
    params: routes::BenchViewPath,
) -> axum::response::Response {
    let page_path = params.to_uri().to_string();
    let render_started = Instant::now();
    let github_stars = github_stars_value_string().await;
    let data = state.storage.data_read();
    let runs_manifests = state.storage.runs_read();
    let config = state.config_read();

    let selection_query = super::types::IndexQuery {
        run: Some(params.run.clone()),
        env: Some(params.env.clone()),
        test: Some(params.test.clone()),
    };
    let selection = select_common(&data, &runs_manifests, &config, &selection_query);

    if selection.runs.is_empty() {
        return Redirect::to(routes::IndexRoot::PATH).into_response();
    }

    let run_data = data.get(&selection.active_run_id);

    let mut bench_detail: Option<BenchDetailView> = None;
    if let Some(r_data) = run_data
        && let Some(env_data) = r_data.get(&selection.active_env)
    {
        let mut candidate: Option<(&String, &String, &wfb_storage::BenchmarkResult)> = None;
        for (lang, lang_data) in env_data {
            for (bench_name, bench_result) in lang_data {
                if bench_result.test_cases.contains_key(&selection.active_test)
                    && bench_name == &params.framework
                {
                    candidate = Some((lang, bench_name, bench_result));
                    break;
                }
            }
            if candidate.is_some() {
                break;
            }
        }

        if let Some((lang, bench_name, bench_result)) = candidate
            && let Some(test_summary) = bench_result.test_cases.get(&selection.active_test)
        {
            let manifest = &bench_result.manifest;
            let database = manifest
                .database
                .as_ref()
                .map(|db| db.to_string().to_uppercase());

            let mut tags: Vec<(String, String)> = manifest
                .tags
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            tags.sort_by(|a, b| a.0.cmp(&b.0));

            bench_detail = Some(BenchDetailView {
                run_id: selection.active_run_id.clone(),
                env: selection.active_env.clone(),
                test: selection.active_test.clone(),
                framework: bench_name.clone(),
                language: lang.clone(),
                framework_version: manifest.framework_version.clone(),
                language_version: manifest.language_version.clone(),
                database,
                repo_url: benchmark_repo_url(&manifest.path),
                path: manifest.path.clone(),
                tags,
                rps: test_summary.requests_per_sec,
                tps: test_summary.bytes_per_sec,
                latency_p99: test_summary.latency_p99,
                errors: test_summary.total_errors,
            });
        }
    }

    HtmlTemplate(BenchTemplate {
        chrome: chrome_context(render_started, false, github_stars, &page_path),
        selection,
        bench: bench_detail,
        routes: Routes,
    })
    .into_response()
}
