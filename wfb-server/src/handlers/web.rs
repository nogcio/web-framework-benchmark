use askama::Template;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;
use std::fmt;
use std::sync::Arc;
use std::time::Instant;
use wfb_storage::BenchmarkTests;

#[allow(unused_imports)]
use crate::filters as filters;
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

#[derive(Template)]
#[template(path = "pages/index.html.j2")]
struct IndexTemplate {
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
    latency_p50: u64,
    latency_p90: u64,
    latency_p99: u64,
    errors: u64,
    memory_usage_bytes: u64,
    cpu_usage_percent: f64,
}

struct BenchmarkView {
    framework: String,
    framework_url: Option<String>,
    framework_version: String,
    language: String,
    language_url: Option<String>,
    language_color: String,
    rps: f64,
    rps_percent: f64,
    tps: u64,
    latency_p99: u64,
    errors: u64,
    database: Option<String>,
    repo_url: Option<String>,
}

#[derive(Deserialize)]
pub struct IndexQuery {
    run: Option<String>,
    env: Option<String>,
    test: Option<String>,
}

pub async fn index_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<IndexQuery>,
) -> impl IntoResponse {
    let render_started = Instant::now();
    let data = state.storage.data.read().unwrap();
    let runs_manifests = state.storage.runs.read().unwrap();
    
    // 1. Get Runs
    let mut runs = Vec::new();
    for (run_id, _) in data.iter() {
        let created_at = if let Some(manifest) = runs_manifests.get(run_id) {
            manifest.created_at
        } else {
            chrono::Utc::now()
        };
        runs.push(RunView {
            id: run_id.clone(),
            created_at_fmt: created_at.format("%b %d, %Y").to_string(),
        });
    }
    // Sort desc by id (which contains timestamp usually)
    runs.sort_by(|a, b| b.id.cmp(&a.id));

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
    let active_run_id = query.run.unwrap_or_else(|| runs.first().unwrap().id.clone());
    let run_data = data.get(&active_run_id);

    // 3. Get Environments for this run
    let mut environment_names = Vec::new();
    if let Some(r_data) = run_data {
        for k in r_data.keys() {
            environment_names.push(k.clone());
        }
    }
    environment_names.sort();
    // 4. Select Environment
    let active_env = query
        .env
        .unwrap_or_else(|| environment_names.first().cloned().unwrap_or_default());

    let config = state.config.read().unwrap();
    let environments: Vec<EnvironmentView> = environment_names
        .into_iter()
        .map(|name| {
            if let Some(env) = config.get_environment(&name) {
                EnvironmentView {
                    name,
                    title: env.title().to_string(),
                    icon: env.icon().unwrap_or("laptop").to_string(),
                }
            } else {
                let title = name.clone();
                EnvironmentView {
                    name,
                    title,
                    icon: "laptop".to_string(),
                }
            }
        })
        .collect();

    // 5. Get Tests
    let tests = vec![
        TestView {
            id: BenchmarkTests::PlainText.to_string(),
            name: "Plain Text".to_string(),
            icon: "zap".to_string(),
        },
        TestView {
            id: BenchmarkTests::JsonAggregate.to_string(),
            name: "JSON".to_string(),
            icon: "braces".to_string(),
        },
        TestView {
            id: BenchmarkTests::GrpcAggregate.to_string(),
            name: "gRPC".to_string(),
            icon: "server".to_string(),
        },
        TestView {
            id: BenchmarkTests::DbComplex.to_string(),
            name: "Database".to_string(),
            icon: "database".to_string(),
        },
    ];
    
    // 6. Select Test
    let active_test = query.test.unwrap_or_else(|| tests.first().unwrap().id.clone());

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
                    let language_url = language_meta.map(|lang| lang.url.clone());
                    let framework_url = config
                        .frameworks()
                        .iter()
                        .find(|fw| fw.name == *bench_name)
                        .map(|fw| fw.url.clone());

                    let database = manifest
                        .database
                        .as_ref()
                        .map(|db| db.to_string().to_uppercase());

                    benchmarks.push(BenchmarkView {
                        framework: bench_name.clone(),
                        framework_url,
                        framework_version: manifest.framework_version.clone(),
                        language: lang.clone(),
                        language_url,
                        language_color: language_color.to_string(),
                        rps: test_summary.requests_per_sec,
                        rps_percent: 0.0,
                        tps: test_summary.bytes_per_sec,
                        latency_p99: test_summary.latency_p99,
                        errors: test_summary.total_errors,
                        database,
                        repo_url: benchmark_repo_url(&manifest.path),
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
    benchmarks.sort_by(|a, b| b.rps.partial_cmp(&a.rps).unwrap_or(std::cmp::Ordering::Equal));

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

    let mut runs = Vec::new();
    for (run_id, _) in data.iter() {
        let created_at = if let Some(manifest) = runs_manifests.get(run_id) {
            manifest.created_at
        } else {
            chrono::Utc::now()
        };
        runs.push(RunView {
            id: run_id.clone(),
            created_at_fmt: created_at.format("%b %d, %Y").to_string(),
        });
    }
    runs.sort_by(|a, b| b.id.cmp(&a.id));

    let (active_run_id, run_data) = if let Some(run_id) = query.run.clone() {
        let run_data = data.get(&run_id);
        (run_id, run_data)
    } else {
        let run_id = runs.first().map(|r| r.id.clone()).unwrap_or_default();
        let run_data = data.get(&run_id);
        (run_id, run_data)
    };

    let mut environment_names = Vec::new();
    if let Some(r_data) = run_data {
        for k in r_data.keys() {
            environment_names.push(k.clone());
        }
    }
    environment_names.sort();
    let active_env = query
        .env
        .unwrap_or_else(|| environment_names.first().cloned().unwrap_or_default());

    let config = state.config.read().unwrap();
    let environments: Vec<EnvironmentView> = environment_names
        .into_iter()
        .map(|name| {
            if let Some(env) = config.get_environment(&name) {
                EnvironmentView {
                    name,
                    title: env.title().to_string(),
                    icon: env.icon().unwrap_or("laptop").to_string(),
                }
            } else {
                let title = name.clone();
                EnvironmentView {
                    name,
                    title,
                    icon: "laptop".to_string(),
                }
            }
        })
        .collect();

    let tests = vec![
        TestView {
            id: BenchmarkTests::PlainText.to_string(),
            name: "Plain Text".to_string(),
            icon: "zap".to_string(),
        },
        TestView {
            id: BenchmarkTests::JsonAggregate.to_string(),
            name: "JSON".to_string(),
            icon: "braces".to_string(),
        },
        TestView {
            id: BenchmarkTests::GrpcAggregate.to_string(),
            name: "gRPC".to_string(),
            icon: "server".to_string(),
        },
        TestView {
            id: BenchmarkTests::DbComplex.to_string(),
            name: "Database".to_string(),
            icon: "database".to_string(),
        },
    ];
    let active_test = query.test.unwrap_or_else(|| tests.first().unwrap().id.clone());

    let mut bench_detail: Option<BenchDetailView> = None;
    if let Some(r_data) = run_data
        && let Some(env_data) = r_data.get(&active_env)
    {
        let mut candidate: Option<(&String, &String, &wfb_storage::BenchmarkResult)> = None;
        for (lang, lang_data) in env_data {
            for (bench_name, bench_result) in lang_data {
                if bench_result.test_cases.get(&active_test).is_some() {
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

        if let Some((lang, bench_name, bench_result)) = candidate {
            if let Some(test_summary) = bench_result.test_cases.get(&active_test) {
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
                    latency_p50: test_summary.latency_p50,
                    latency_p90: test_summary.latency_p90,
                    latency_p99: test_summary.latency_p99,
                    errors: test_summary.total_errors,
                    memory_usage_bytes: test_summary.memory_usage_bytes,
                    cpu_usage_percent: test_summary.cpu_usage_percent,
                });
            }
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
}
