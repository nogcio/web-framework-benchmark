use axum::{
    Router,
    body::Body,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Json, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use tower_http::services::ServeDir;

use crate::{
    benchmark::BenchmarkTests,
    benchmark_environment::get_environment_config,
    db::{
        self,
        runs::{RunResult, RunSummary},
    },
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TestInfo {
    id: String,
    name: String,
}

#[derive(Serialize)]
struct LanguageInfo {
    name: String,
    url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FrameworkInfo {
    language: String,
    name: String,
    url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BenchmarkInfo {
    name: String,
    language: String,
    language_version: String,
    framework: String,
    framework_version: String,
    tests: Vec<String>,
    tags: HashMap<String, String>,
    path: String,
    database: String,
    disabled: bool,
    only: bool,
    arguments: Vec<String>,
    env: HashMap<String, String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvironmentInfo {
    name: String,
    display_name: String,
    spec: Option<String>,
    icon: String,
}

#[derive(Serialize)]
struct VersionInfo {
    version: String,
}

#[derive(Deserialize)]
struct TranscriptParams {
    lang: Option<String>,
}

pub fn create_router(db: db::Db) -> Router {
    let mut router = Router::new()
        .route("/api/tags", get(get_tags))
        .route("/api/environments", get(get_environments))
        .route("/api/tests", get(get_tests))
        .route("/api/languages", get(get_languages))
        .route("/api/frameworks", get(get_frameworks))
        .route("/api/benchmarks", get(get_benchmarks))
        .route("/api/runs", get(get_runs))
        .route("/api/version", get(get_version))
        .route(
            "/api/runs/{run_id}/environments/{env}/tests/{test}",
            get(get_run_results),
        )
        .route(
            "/api/runs/{run_id}/environments/{env}/tests/{test}/frameworks/{framework}/transcript",
            get(get_run_transcript),
        )
        .with_state(db);

    if std::path::Path::new("static").exists() {
        router =
            router.fallback_service(ServeDir::new("static").append_index_html_on_directories(true));
    }

    router
}

async fn get_version() -> Json<VersionInfo> {
    Json(VersionInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

async fn get_tags(State(db): State<db::Db>) -> Result<Json<Vec<String>>, StatusCode> {
    let tags = db
        .get_tag_keys()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(tags))
}

async fn get_environments(
    State(db): State<db::Db>,
) -> Result<Json<Vec<EnvironmentInfo>>, StatusCode> {
    let env_names = db
        .get_environments()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut environments = Vec::new();
    for name in env_names {
        if let Ok(config) = get_environment_config(&name) {
            environments.push(EnvironmentInfo {
                name,
                spec: config.spec,
                display_name: config.name,
                icon: config.icon.unwrap_or_else(|| "server".to_string()),
            });
        }
    }
    Ok(Json(environments))
}

async fn get_tests(State(_db): State<db::Db>) -> Result<Json<Vec<TestInfo>>, StatusCode> {
    let tests = vec![
        BenchmarkTests::HelloWorld,
        BenchmarkTests::Json,
        BenchmarkTests::DbReadOne,
        BenchmarkTests::DbReadPaging,
        BenchmarkTests::DbWrite,
        BenchmarkTests::TweetService,
        BenchmarkTests::StaticFilesSmall,
        BenchmarkTests::StaticFilesMedium,
        BenchmarkTests::StaticFilesLarge,
    ]
    .into_iter()
    .map(|t| TestInfo {
        id: t.to_string(),
        name: readable_test_name(&t),
    })
    .collect();
    Ok(Json(tests))
}

fn readable_test_name(test: &BenchmarkTests) -> String {
    match test {
        BenchmarkTests::HelloWorld => "Plain Text".to_string(),
        BenchmarkTests::Json => "JSON".to_string(),
        BenchmarkTests::DbReadOne => "DB Read One".to_string(),
        BenchmarkTests::DbReadPaging => "DB Read Paging".to_string(),
        BenchmarkTests::DbWrite => "DB Write".to_string(),
        BenchmarkTests::StaticFilesSmall => "Files Small".to_string(),
        BenchmarkTests::StaticFilesMedium => "Files Medium".to_string(),
        BenchmarkTests::StaticFilesLarge => "Files Large".to_string(),
        BenchmarkTests::TweetService => "Tweet Service".to_string(),
    }
}

async fn get_languages(State(db): State<db::Db>) -> Result<Json<Vec<LanguageInfo>>, StatusCode> {
    let languages = db
        .get_languages()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|l| LanguageInfo {
            name: l.name,
            url: l.url,
        })
        .collect();
    Ok(Json(languages))
}

async fn get_frameworks(State(db): State<db::Db>) -> Result<Json<Vec<FrameworkInfo>>, StatusCode> {
    let frameworks = db
        .get_frameworks()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|f| FrameworkInfo {
            language: f.language,
            name: f.name,
            url: f.url,
        })
        .collect();
    Ok(Json(frameworks))
}

async fn get_benchmarks(State(db): State<db::Db>) -> Result<Json<Vec<BenchmarkInfo>>, StatusCode> {
    let benchmarks = db
        .get_benchmarks()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|b| BenchmarkInfo {
            name: b.name,
            language: b.language,
            language_version: b.language_version,
            framework: b.framework,
            framework_version: b.framework_version,
            tests: b.tests.iter().map(|t| t.to_string()).collect(),
            tags: b.tags,
            path: b.path,
            database: b
                .database
                .map(|d| d.to_string())
                .unwrap_or_else(|| "none".to_string()),
            disabled: b.disabled,
            only: b.only,
            arguments: b.arguments,
            env: b.env,
        })
        .collect();
    Ok(Json(benchmarks))
}

async fn get_runs(State(db): State<db::Db>) -> Result<Json<Vec<RunSummary>>, StatusCode> {
    let runs = db
        .get_runs()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(runs))
}

async fn get_run_results(
    State(db): State<db::Db>,
    Path((run_id, env_str, test_str)): Path<(u32, String, String)>,
) -> Result<Json<Vec<RunResult>>, StatusCode> {
    let environment = env_str;
    let test = test_str
        .as_str()
        .try_into()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let results = db
        .get_run_results(run_id, environment, test)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(results))
}

async fn get_run_transcript(
    State(db): State<db::Db>,
    Path((run_id, env, test, framework)): Path<(u32, String, String, String)>,
    Query(params): Query<TranscriptParams>,
) -> Result<Response, StatusCode> {
    let transcript_path = db
        .get_transcript(run_id, &env, &test, &framework, params.lang.as_deref())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match transcript_path {
        Some(path) => {
            let file = File::open(path)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let stream = ReaderStream::new(file);
            let body = Body::from_stream(stream);

            Ok(Response::builder()
                .header("Content-Type", "text/markdown; charset=utf-8")
                .body(body)
                .unwrap())
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}
