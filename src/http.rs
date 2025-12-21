use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::get,
};
use serde::Serialize;
use std::collections::HashMap;

use crate::{
    benchmark::BenchmarkTests,
    benchmark_environment::get_environment_config,
    db::{self, runs::RunResult},
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
    tags: HashMap<String, String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunInfo {
    id: u32,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvironmentInfo {
    name: String,
    display_name: String,
    icon: String,
}

pub fn create_router(db: db::Db) -> Router {
    Router::new()
        .route("/api/tags", get(get_tags))
        .route("/api/environments", get(get_environments))
        .route("/api/tests", get(get_tests))
        .route("/api/languages", get(get_languages))
        .route("/api/frameworks", get(get_frameworks))
        .route("/api/runs", get(get_runs))
        .route(
            "/api/runs/{run_id}/environments/{env}/tests/{test}",
            get(get_run_results),
        )
        .with_state(db)
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
        BenchmarkTests::StaticFiles,
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
        BenchmarkTests::HelloWorld => "Hello World".to_string(),
        BenchmarkTests::Json => "JSON".to_string(),
        BenchmarkTests::DbReadOne => "Database Read One".to_string(),
        BenchmarkTests::DbReadPaging => "Database Read Paging".to_string(),
        BenchmarkTests::DbWrite => "Database Write".to_string(),
        BenchmarkTests::StaticFiles => "Static Files".to_string(),
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
            name: f.framework.name,
            url: f.framework.url,
            tags: f.framework.tags,
        })
        .collect();
    Ok(Json(frameworks))
}

async fn get_runs(State(db): State<db::Db>) -> Result<Json<Vec<RunInfo>>, StatusCode> {
    let runs = db
        .get_runs()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|r| RunInfo {
            id: r.id,
            created_at: r.manifest.created_at,
        })
        .collect();
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
