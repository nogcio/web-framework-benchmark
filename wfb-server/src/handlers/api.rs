use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use std::time::Duration;

use crate::api_models::*;
use crate::handlers::common;
use crate::routes;
use crate::state::AppState;

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
    Json(common::get_available_tests())
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
    Json(common::get_all_runs(&data, &runs_manifests))
}

pub async fn get_run_results(
    State(state): State<Arc<AppState>>,
    params: routes::ApiRunResultsPath,
) -> Json<Vec<RunResult>> {
    let routes::ApiRunResultsPath { run_id, env, test } = params;
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
                    });
                }
            }
        }
    }
    Json(results)
}

pub async fn get_run_raw_data(
    State(state): State<Arc<AppState>>,
    params: routes::ApiRunRawPath,
    Query(query_params): Query<TranscriptParams>,
) -> Result<Json<Vec<TestCaseRawApi>>, StatusCode> {
    let routes::ApiRunRawPath {
        run_id,
        env,
        test,
        framework,
    } = params;
    let lang = if let Some(l) = query_params.lang {
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
