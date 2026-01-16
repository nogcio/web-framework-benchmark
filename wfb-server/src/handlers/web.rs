use askama::Template;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLockReadGuard};
use std::time::Instant;
use wfb_storage::{BenchmarkTests, StorageData};

#[allow(unused_imports)]
use crate::filters;
use crate::handlers::common;
use crate::state::AppState;
use crate::view_models::{EnvironmentView, RunView, TestView};

const BACKEND_VERSION: &str = env!("CARGO_PKG_VERSION");
const REPOSITORY_URL: &str = env!("CARGO_PKG_REPOSITORY");
struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {}", err),
            )
                .into_response(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct RenderDuration {
    start: Instant,
}

impl RenderDuration {
    pub fn new(start: Instant) -> Self {
        Self { start }
    }
}

impl fmt::Display for RenderDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ms = self.start.elapsed().as_secs_f64() * 1000.0;
        if ms >= 1.0 {
            write!(f, "{:.0}", ms.round())
        } else {
            write!(f, "{:.2}", ms)
        }
    }
}

fn select_important_table_tags(tags: &HashMap<String, String>) -> Vec<(String, String)> {
    let mut selected = Vec::new();

    if let Some(v) = tags.get("type") {
        selected.push(("type".to_string(), v.clone()));
    }
    if let Some(v) = tags.get("runtime") {
        selected.push(("runtime".to_string(), v.clone()));
    }

    // Variant C: arch is usually too noisy, so show only when it's not a common baseline.
    if let Some(v) = tags.get("arch") {
        let baseline = matches!(v.as_str(), "async" | "event-loop" | "coroutine");
        if !baseline {
            selected.push(("arch".to_string(), v.clone()));
        }
    }

    // ORM only matters for DB-ish implementations.
    if let Some(v) = tags.get("orm") {
        selected.push(("orm".to_string(), v.clone()));
    }

    selected
}

fn benchmark_repo_url(path: &str) -> Option<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return None;
    }
    let path = trimmed.trim_start_matches("./");
    if path.is_empty() {
        return None;
    }
    Some(format!(
        "{}/tree/main/{}",
        REPOSITORY_URL.trim_end_matches('/'),
        path
    ))
}

fn get_runs(
    data: &RwLockReadGuard<'_, StorageData>,
    runs_manifests: &RwLockReadGuard<
        '_,
        std::collections::HashMap<String, wfb_storage::RunManifest>,
    >,
) -> Vec<RunView> {
    common::get_all_runs(data, runs_manifests)
        .into_iter()
        .map(|r| RunView {
            id: r.id,
            created_at_fmt: r.created_at.format("%b %d, %Y").to_string(),
        })
        .collect()
}

fn get_environment_views(
    run_id: &str,
    data: &RwLockReadGuard<'_, StorageData>,
    config: &wfb_storage::Config,
) -> Vec<EnvironmentView> {
    common::get_run_environments(run_id, data, config)
        .into_iter()
        .map(|e| EnvironmentView {
            name: e.name,
            title: e.display_name,
            icon: e.icon,
            spec: e.spec,
        })
        .collect()
}

fn get_available_tests() -> Vec<TestView> {
    common::get_available_tests()
        .into_iter()
        .map(|t| TestView {
            id: t.id.unwrap_or_default(),
            name: t.name,
            icon: t.icon,
        })
        .collect()
}

#[derive(Template)]
#[template(path = "pages/index.html.j2")]
struct IndexTemplate {
    page: PageContext,
}

#[derive(Template)]
#[template(path = "pages/methodology.html.j2")]
struct MethodologyTemplate {
    page: PageContext,
}

struct PageContext {
    runs: Vec<RunView>,
    active_run_id: String,
    environments: Vec<EnvironmentView>,
    active_env: String,
    tests: Vec<TestView>,
    active_test: String,
    benchmarks: Vec<BenchmarkView>,
    backend_version: &'static str,
    render_duration: RenderDuration,
    show_header_controls: bool,
}

#[derive(Template)]
#[template(path = "pages/bench.html.j2")]
struct BenchTemplate {
    page: PageContext,
    bench: Option<BenchDetailView>,
}

struct BenchDetailView {
    run_id: String,
    env: String,
    test: String,
    framework: String,
    language: String,
    framework_version: String,
    language_version: String,
    database: Option<String>,
    repo_url: Option<String>,
    path: String,
    tags: Vec<(String, String)>,
    rps: f64,
    tps: u64,
    latency_p99: u64,
    errors: u64,
}

struct BenchmarkView {
    framework: String,
    framework_version: String,
    language: String,
    language_color: String,
    rps: f64,
    rps_percent: f64,
    tps: u64,
    latency_p99: u64,
    errors: u64,
    database: Option<String>,
    tags: Vec<(String, String)>,
}

#[derive(Deserialize)]
pub struct IndexQuery {
    run: Option<String>,
    env: Option<String>,
    test: Option<String>,
}

pub async fn methodology_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<IndexQuery>,
) -> impl IntoResponse {
    let render_started = Instant::now();
    let data = state.storage.data.read().unwrap();
    let runs_manifests = state.storage.runs.read().unwrap();

    let runs = get_runs(&data, &runs_manifests);

    // If there are no runs yet, still render the page.
    let active_run_id = query
        .run
        .or_else(|| runs.first().map(|r| r.id.clone()))
        .unwrap_or_default();

    let config = state.config.read().unwrap();
    let environments = if !active_run_id.is_empty() {
        get_environment_views(&active_run_id, &data, &config)
    } else {
        vec![]
    };

    let active_env = query.env.unwrap_or_else(|| {
        environments
            .first()
            .map(|e| e.name.clone())
            .unwrap_or_default()
    });

    let tests = get_available_tests();
    let active_test = query
        .test
        .unwrap_or_else(|| tests.first().map(|t| t.id.clone()).unwrap_or_default());

    HtmlTemplate(MethodologyTemplate {
        page: PageContext {
            runs,
            active_run_id,
            environments,
            active_env,
            tests,
            active_test,
            benchmarks: vec![],
            backend_version: BACKEND_VERSION,
            render_duration: RenderDuration::new(render_started),
            show_header_controls: false,
        },
    })
}

pub async fn index_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<IndexQuery>,
) -> impl IntoResponse {
    let render_started = Instant::now();
    let data = state.storage.data.read().unwrap();
    let runs_manifests = state.storage.runs.read().unwrap();

    // 1. Get Runs
    let runs = get_runs(&data, &runs_manifests);

    if runs.is_empty() {
        return HtmlTemplate(IndexTemplate {
            page: PageContext {
                runs: vec![],
                active_run_id: "".to_string(),
                environments: vec![],
                active_env: "".to_string(),
                tests: vec![],
                active_test: "".to_string(),
                benchmarks: vec![],
                backend_version: BACKEND_VERSION,
                render_duration: RenderDuration::new(render_started),
                show_header_controls: true,
            },
        });
    }

    // 2. Select Run
    let active_run_id = query
        .run
        .unwrap_or_else(|| runs.first().unwrap().id.clone());
    let run_data = data.get(&active_run_id);

    // 3. Get Environments for this run
    let config = state.config.read().unwrap();
    let environments = get_environment_views(&active_run_id, &data, &config);

    // 4. Select Environment
    let active_env = query.env.unwrap_or_else(|| {
        environments
            .first()
            .map(|e| e.name.clone())
            .unwrap_or_default()
    });

    // 5. Get Tests
    let tests = get_available_tests();

    // 6. Select Test
    let active_test = query
        .test
        .unwrap_or_else(|| tests.first().unwrap().id.clone());

    // 7. Get Benchmarks
    let mut benchmarks = Vec::new();
    let mut max_rps = 0.0;

    if let Some(r_data) = run_data
        && let Some(env_data) = r_data.get(&active_env)
    {
        for (lang, lang_data) in env_data {
            for (bench_name, bench_result) in lang_data {
                if let Some(test_summary) = bench_result.test_cases.get(&active_test) {
                    if test_summary.requests_per_sec > max_rps {
                        max_rps = test_summary.requests_per_sec;
                    }

                    let manifest = &bench_result.manifest;
                    let language_meta = config.get_lang(lang);
                    let language_color = language_meta
                        .map(|lang| lang.color.as_str())
                        .unwrap_or("#94a3b8");

                    let database = if active_test == BenchmarkTests::DbComplex.to_string() {
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

    // Sort by RPS desc
    benchmarks.sort_by(|a, b| {
        b.rps
            .partial_cmp(&a.rps)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    HtmlTemplate(IndexTemplate {
        page: PageContext {
            runs,
            active_run_id,
            environments,
            active_env,
            tests,
            active_test,
            benchmarks,
            backend_version: BACKEND_VERSION,
            render_duration: RenderDuration::new(render_started),
            show_header_controls: true,
        },
    })
}

#[derive(Deserialize)]
pub struct BenchQuery {
    run: Option<String>,
    env: Option<String>,
    test: Option<String>,
    framework: Option<String>,
}

pub async fn bench_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<BenchQuery>,
) -> impl IntoResponse {
    let render_started = Instant::now();
    let data = state.storage.data.read().unwrap();
    let runs_manifests = state.storage.runs.read().unwrap();

    let runs = get_runs(&data, &runs_manifests);
    if runs.is_empty() {
        // Fallback for empty state (reusing index logic slightly or just returning 404/redirect)
        // For now, let's just return a basic error or redirect to home.
        return axum::response::Redirect::to("/").into_response();
    }

    let (active_run_id, run_data) = if let Some(run_id) = query.run.clone() {
        let run_data = data.get(&run_id);
        (run_id, run_data)
    } else {
        let run_id = runs.first().map(|r| r.id.clone()).unwrap_or_default();
        let run_data = data.get(&run_id);
        (run_id, run_data)
    };

    let config = state.config.read().unwrap();
    let environments = get_environment_views(&active_run_id, &data, &config);
    let active_env = query.env.unwrap_or_else(|| {
        environments
            .first()
            .map(|e| e.name.clone())
            .unwrap_or_default()
    });

    let tests = get_available_tests();
    let active_test = query
        .test
        .unwrap_or_else(|| tests.first().unwrap().id.clone());

    let mut bench_detail: Option<BenchDetailView> = None;
    if let Some(r_data) = run_data
        && let Some(env_data) = r_data.get(&active_env)
    {
        let mut candidate: Option<(&String, &String, &wfb_storage::BenchmarkResult)> = None;
        for (lang, lang_data) in env_data {
            for (bench_name, bench_result) in lang_data {
                if bench_result.test_cases.contains_key(&active_test) {
                    if let Some(ref framework) = query.framework {
                        if bench_name == framework {
                            candidate = Some((lang, bench_name, bench_result));
                            break;
                        }
                    } else if candidate.is_none() {
                        candidate = Some((lang, bench_name, bench_result));
                    }
                }
            }
            if candidate.is_some() && query.framework.is_some() {
                break;
            }
        }

        if let Some((lang, bench_name, bench_result)) = candidate
            && let Some(test_summary) = bench_result.test_cases.get(&active_test)
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
                run_id: active_run_id.clone(),
                env: active_env.clone(),
                test: active_test.clone(),
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
        page: PageContext {
            runs,
            active_run_id,
            environments,
            active_env,
            tests,
            active_test,
            benchmarks: vec![],
            backend_version: BACKEND_VERSION,
            render_duration: RenderDuration::new(render_started),
            show_header_controls: false,
        },
        bench: bench_detail,
    })
    .into_response()
}
