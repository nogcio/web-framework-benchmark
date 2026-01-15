use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Json, Response},
};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::state::AppState;
use crate::api_models::*;
use wfb_storage::BenchmarkTests;

pub async fn get_version() -> Json<VersionInfo> {
    Json(VersionInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

pub async fn get_tags(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    let config = state.config.read().unwrap();
    let mut tags = std::collections::HashSet::new();
    for b in config.benchmarks() {
        for key in b.tags.keys() {
            tags.insert(key.clone());
        }
    }
    let mut tags_vec: Vec<String> = tags.into_iter().collect();
    tags_vec.sort();
    Json(tags_vec)
}

pub async fn get_environments(State(state): State<Arc<AppState>>) -> Json<Vec<EnvironmentInfo>> {
    let data = state.storage.data.read().unwrap();
    let mut used_envs = std::collections::HashSet::new();
    for run_data in data.values() {
        for env_name in run_data.keys() {
            used_envs.insert(env_name.clone());
        }
    }

    let config = state.config.read().unwrap();
    let envs = config
        .environments()
        .iter()
        .filter(|e| used_envs.contains(e.name()))
        .map(|e| EnvironmentInfo {
            name: e.name().to_string(),
            display_name: e.title().to_string(),
            spec: e.spec().map(|s| s.to_string()),
            icon: e.icon().unwrap_or("laptop").to_string(),
        })
        .collect();
    Json(envs)
}

pub async fn get_tests() -> Json<Vec<TestInfo>> {
    let tests = vec![
        TestInfo {
            id: Some(BenchmarkTests::PlainText.to_string()),
            name: "Plain Text".to_string(),
            icon: "zap".to_string(),
            children: vec![],
        },
        TestInfo {
            id: Some(BenchmarkTests::JsonAggregate.to_string()),
            name: "JSON".to_string(),
            icon: "braces".to_string(),
            children: vec![],
        },
        TestInfo {
            id: Some(BenchmarkTests::GrpcAggregate.to_string()),
            name: "gRPC".to_string(),
            icon: "server".to_string(), // best guess for icon
            children: vec![],
        },
        TestInfo {
            id: Some(BenchmarkTests::DbComplex.to_string()),
            name: "Database".to_string(),
            icon: "database".to_string(),
            children: vec![],
        },
    ];
    Json(tests)
}

pub async fn get_languages(State(state): State<Arc<AppState>>) -> Json<Vec<LanguageInfo>> {
    let config = state.config.read().unwrap();
    let langs = config
        .languages()
        .iter()
        .map(|l| LanguageInfo {
            name: l.name.clone(),
            url: l.url.clone(),
            color: l.color.clone(),
        })
        .collect();
    Json(langs)
}

pub async fn get_frameworks(State(state): State<Arc<AppState>>) -> Json<Vec<FrameworkInfo>> {
    let config = state.config.read().unwrap();
    let frameworks = config
        .frameworks()
        .iter()
        .map(|f| FrameworkInfo {
            language: f.language.clone(),
            name: f.name.clone(),
            url: f.url.clone(),
        })
        .collect();
    Json(frameworks)
}

pub async fn get_benchmarks(State(state): State<Arc<AppState>>) -> Json<Vec<BenchmarkInfo>> {
    let config = state.config.read().unwrap();
    let benchmarks = config
        .benchmarks()
        .iter()
        .map(|b| BenchmarkInfo {
            name: b.name.clone(),
            language: b.language.clone(),
            language_version: b.language_version.clone(),
            framework: b.framework.clone(),
            framework_version: b.framework_version.clone(),
            tests: b.tests.iter().map(|t| t.to_string()).collect(),
            tags: b.tags.clone(),
            path: b.path.clone(),
            database: b
                .database
                .map(|d| format!("{:?}", d).to_lowercase())
                .unwrap_or_else(|| "none".to_string()),
            disabled: b.disabled,
            only: b.only,
            arguments: b.arguments.clone(),
            env: b.env.clone(),
        })
        .collect();
    Json(benchmarks)
}

pub async fn get_runs(State(state): State<Arc<AppState>>) -> Json<Vec<RunSummary>> {
    let data = state.storage.data.read().unwrap();
    let runs_manifests = state.storage.runs.read().unwrap();
    let mut runs = Vec::new();
    for (run_id, _) in data.iter() {
        let created_at = if let Some(manifest) = runs_manifests.get(run_id) {
            manifest.created_at
        } else {
            chrono::Utc::now()
        };
        runs.push(RunSummary {
            id: run_id.clone(),
            created_at,
        });
    }
    runs.sort_by(|a, b| b.id.cmp(&a.id));
    Json(runs)
}

pub async fn get_run_results(
    State(state): State<Arc<AppState>>,
    Path((run_id, env, test)): Path<(String, String, String)>,
) -> Json<Vec<RunResult>> {
    let data = state.storage.data.read().unwrap();
    let mut results = Vec::new();
    if let Some(env_data) = data.get(&run_id).and_then(|run_data| run_data.get(&env)) {
        for (lang, lang_data) in env_data {
            for (bench_name, bench_result) in lang_data {
                if let Some(test_summary) = bench_result.test_cases.get(&test) {
                    results.push(RunResult {
                        name: bench_name.clone(),
                        language: lang.clone(),
                        language_version: bench_result.manifest.language_version.clone(),
                        framework: bench_name.clone(),
                        framework_version: bench_result.manifest.framework_version.clone(),
                        database: bench_result
                            .manifest
                            .database
                            .as_ref()
                            .map(|d| format!("{:?}", d).to_lowercase()),
                        path: Some(bench_result.manifest.path.clone()),
                        rps: test_summary.requests_per_sec,
                        tps: test_summary.bytes_per_sec,
                        latency_avg: Duration::from_secs_f64(
                            test_summary.latency_mean / 1_000_000.0,
                        ),
                        latency_stdev: Duration::from_secs_f64(
                            test_summary.latency_stdev / 1_000_000.0,
                        ),
                        latency_max: Duration::from_micros(test_summary.latency_max),
                        latency50: Duration::from_micros(test_summary.latency_p50),
                        latency75: Duration::from_micros(test_summary.latency_p75),
                        latency90: Duration::from_micros(test_summary.latency_p90),
                        latency99: Duration::from_micros(test_summary.latency_p99),
                        latency_stdev_pct: test_summary.latency_stdev_pct,
                        latency_distribution: test_summary
                            .latency_distribution
                            .iter()
                            .map(|(p, l)| (*p, Duration::from_micros(*l)))
                            .collect(),
                        req_per_sec_avg: test_summary.req_per_sec_avg,
                        req_per_sec_stdev: test_summary.req_per_sec_stdev,
                        req_per_sec_max: test_summary.req_per_sec_max,
                        req_per_sec_stdev_pct: test_summary.req_per_sec_stdev_pct,
                        errors: test_summary.total_errors,
                        memory_usage: test_summary.memory_usage_bytes,
                        tags: bench_result.manifest.tags.clone(),
                        has_transcript: false,
                    });
                }
            }
        }
    }
    Json(results)
}

pub async fn get_run_transcript(
    State(state): State<Arc<AppState>>,
    Path((run_id, env, _test, framework)): Path<(String, String, String, String)>,
    Query(params): Query<TranscriptParams>,
) -> Result<Response, StatusCode> {
    let lang = if let Some(l) = params.lang {
        l
    } else {
        // Try to find language
        let data = state.storage.data.read().unwrap();
        let mut found_lang = None;
        if let Some(env_data) = data.get(&run_id).and_then(|run_data| run_data.get(&env)) {
            for (l, lang_data) in env_data {
                if lang_data.contains_key(&framework) {
                    found_lang = Some(l.clone());
                    break;
                }
            }
        }
        found_lang.ok_or(StatusCode::NOT_FOUND)?
    };

    // Construct path: base_path/run_id/env/lang/framework/transcript.md
    // Note: framework here is the benchmark name
    let path = state
        .storage
        .base_path
        .join(&run_id)
        .join(&env)
        .join(&lang)
        .join(&framework)
        .join("transcript.md"); // Assuming filename

    if path.exists() {
        let file = File::open(path)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let stream = ReaderStream::new(file);
        let body = Body::from_stream(stream);

        Ok(Response::builder()
            .header("Content-Type", "text/markdown; charset=utf-8")
            .body(body)
            .unwrap())
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub async fn get_run_raw_data(
    State(state): State<Arc<AppState>>,
    Path((run_id, env, test, framework)): Path<(String, String, String, String)>,
    Query(params): Query<TranscriptParams>,
) -> Result<Json<Vec<TestCaseRawApi>>, StatusCode> {
    let lang = if let Some(l) = params.lang {
        l
    } else {
        // Try to find language
        let data = state.storage.data.read().unwrap();
        let mut found_lang = None;
        if let Some(env_data) = data.get(&run_id).and_then(|run_data| run_data.get(&env)) {
            for (l, lang_data) in env_data {
                if lang_data.contains_key(&framework) {
                    found_lang = Some(l.clone());
                    break;
                }
            }
        }
        found_lang.ok_or(StatusCode::NOT_FOUND)?
    };

    let raw_data = state
        .storage
        .get_raw_data(&run_id, &env, &lang, &framework, &test)
        .unwrap_or_default();

    let api_data: Vec<TestCaseRawApi> = raw_data.into_iter().map(Into::into).collect();

    Ok(Json(api_data))
}
