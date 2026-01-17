use super::context::{chrome_context, empty_selection_context, select_common};
use super::github::github_stars_value_string;
use super::helpers::select_important_table_tags;
use super::render::HtmlTemplate;
use super::types::{BenchmarkView, ChromeContext, IndexQuery, Routes, SelectionContext};
use askama::Template;
use axum::extract::State;
use axum::response::IntoResponse;
use std::sync::Arc;
use std::time::Instant;
use wfb_storage::BenchmarkTests;

use crate::routes;
use crate::state::AppState;

#[allow(unused_imports)]
use crate::filters;

#[derive(Template)]
#[template(path = "pages/index.rs.j2")]
struct IndexTemplate {
    chrome: ChromeContext,
    selection: SelectionContext,
    benchmarks: Vec<BenchmarkView>,
    routes: Routes,
}

async fn render_index(state: Arc<AppState>, query: IndexQuery) -> HtmlTemplate<IndexTemplate> {
    let render_started = Instant::now();
    let github_stars = github_stars_value_string().await;
    let data = state.storage.data.read().unwrap();
    let runs_manifests = state.storage.runs.read().unwrap();
    let config = state.config.read().unwrap();

    let selection = select_common(&data, &runs_manifests, &config, &query);

    if selection.runs.is_empty() {
        return HtmlTemplate(IndexTemplate {
            chrome: chrome_context(render_started, true, github_stars),
            selection: empty_selection_context(),
            benchmarks: vec![],
            routes: Routes,
        });
    }

    let run_data = data.get(&selection.active_run_id);

    let mut benchmarks = Vec::new();
    let mut max_rps = 0.0;

    if let Some(r_data) = run_data
        && let Some(env_data) = r_data.get(&selection.active_env)
    {
        for (lang, lang_data) in env_data {
            for (bench_name, bench_result) in lang_data {
                if let Some(test_summary) = bench_result.test_cases.get(&selection.active_test) {
                    if test_summary.requests_per_sec > max_rps {
                        max_rps = test_summary.requests_per_sec;
                    }

                    let manifest = &bench_result.manifest;
                    let language_meta = config.get_lang(lang);
                    let language_color = language_meta
                        .map(|lang| lang.color.as_str())
                        .unwrap_or("#94a3b8");

                    let database = if selection.active_test == BenchmarkTests::DbComplex.to_string()
                    {
                        manifest
                            .database
                            .as_ref()
                            .map(|db| db.to_string().to_uppercase())
                    } else {
                        None
                    };

                    benchmarks.push(BenchmarkView {
                        framework: bench_name.clone(),
                        framework_version: manifest.framework_version.clone(),
                        language: lang.clone(),
                        language_color: language_color.to_string(),
                        rps: test_summary.requests_per_sec,
                        rps_percent: 0.0,
                        tps: test_summary.bytes_per_sec,
                        latency_p99: test_summary.latency_p99,
                        errors: test_summary.total_errors,
                        database,
                        tags: select_important_table_tags(&manifest.tags),
                    });
                }
            }
        }
    }

    if max_rps > 0.0 {
        for bench in &mut benchmarks {
            let percent = (bench.rps / max_rps) * 100.0;
            bench.rps_percent = percent;
        }
    }

    benchmarks.sort_by(|a, b| {
        b.rps
            .partial_cmp(&a.rps)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    HtmlTemplate(IndexTemplate {
        chrome: chrome_context(render_started, true, github_stars),
        selection,
        benchmarks,
        routes: Routes,
    })
}

pub async fn root_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    render_index(
        state,
        IndexQuery {
            run: None,
            env: None,
            test: None,
        },
    )
    .await
}

pub async fn index_path_handler(
    State(state): State<Arc<AppState>>,
    params: routes::IndexViewPath,
) -> impl IntoResponse {
    let query = IndexQuery {
        run: Some(params.run),
        env: Some(params.env),
        test: Some(params.test),
    };
    render_index(state, query).await
}

#[derive(Template)]
#[template(path = "partials/index/update.rs.j2")]
struct IndexUpdateTemplate {
    chrome: ChromeContext,
    selection: SelectionContext,
    benchmarks: Vec<BenchmarkView>,
    routes: Routes,
}

async fn render_index_update(
    state: Arc<AppState>,
    query: IndexQuery,
) -> HtmlTemplate<IndexUpdateTemplate> {
    let render_started = Instant::now();
    let github_stars = github_stars_value_string().await;
    let data = state.storage.data.read().unwrap();
    let runs_manifests = state.storage.runs.read().unwrap();
    let config = state.config.read().unwrap();

    let selection = select_common(&data, &runs_manifests, &config, &query);
    if selection.runs.is_empty() {
        return HtmlTemplate(IndexUpdateTemplate {
            chrome: chrome_context(render_started, true, github_stars),
            selection: empty_selection_context(),
            benchmarks: vec![],
            routes: Routes,
        });
    }

    let run_data = data.get(&selection.active_run_id);
    let mut benchmarks = Vec::new();
    let mut max_rps = 0.0;

    if let Some(r_data) = run_data
        && let Some(env_data) = r_data.get(&selection.active_env)
    {
        for (lang, lang_data) in env_data {
            for (bench_name, bench_result) in lang_data {
                if let Some(test_summary) = bench_result.test_cases.get(&selection.active_test) {
                    if test_summary.requests_per_sec > max_rps {
                        max_rps = test_summary.requests_per_sec;
                    }

                    let manifest = &bench_result.manifest;
                    let language_meta = config.get_lang(lang);
                    let language_color = language_meta
                        .map(|lang| lang.color.as_str())
                        .unwrap_or("#94a3b8");

                    let database = if selection.active_test == BenchmarkTests::DbComplex.to_string()
                    {
                        manifest
                            .database
                            .as_ref()
                            .map(|db| db.to_string().to_uppercase())
                    } else {
                        None
                    };

                    benchmarks.push(BenchmarkView {
                        framework: bench_name.clone(),
                        framework_version: manifest.framework_version.clone(),
                        language: lang.clone(),
                        language_color: language_color.to_string(),
                        rps: test_summary.requests_per_sec,
                        rps_percent: 0.0,
                        tps: test_summary.bytes_per_sec,
                        latency_p99: test_summary.latency_p99,
                        errors: test_summary.total_errors,
                        database,
                        tags: select_important_table_tags(&manifest.tags),
                    });
                }
            }
        }
    }

    if max_rps > 0.0 {
        for bench in &mut benchmarks {
            bench.rps_percent = (bench.rps / max_rps) * 100.0;
        }
    }

    benchmarks.sort_by(|a, b| {
        b.rps
            .partial_cmp(&a.rps)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    HtmlTemplate(IndexUpdateTemplate {
        chrome: chrome_context(render_started, true, github_stars),
        selection,
        benchmarks,
        routes: Routes,
    })
}

pub async fn index_update_path_handler(
    State(state): State<Arc<AppState>>,
    params: routes::IndexPartialsViewPath,
) -> impl IntoResponse {
    let query = IndexQuery {
        run: Some(params.run),
        env: Some(params.env),
        test: Some(params.test),
    };
    render_index_update(state, query).await
}
