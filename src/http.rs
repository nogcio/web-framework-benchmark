use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::Serialize;
use std::collections::HashMap;
use clap::ValueEnum;

use crate::{
    db::{self, runs::RunResult},
    BenchmarkEnvironmentType, benchmark::BenchmarkTests,
};

#[derive(Serialize)]
struct TagsResponse {
    tags: Vec<String>,
}

#[derive(Serialize)]
struct EnvironmentsResponse {
    environments: Vec<String>,
}

#[derive(Serialize)]
struct TestsResponse {
    tests: Vec<TestInfo>,
}

#[derive(Serialize)]
struct TestInfo {
    id: String,
    name: String,
}

#[derive(Serialize)]
struct LanguagesResponse {
    languages: Vec<LanguageInfo>,
}

#[derive(Serialize)]
struct LanguageInfo {
    name: String,
    url: String,
}

#[derive(Serialize)]
struct FrameworksResponse {
    frameworks: Vec<FrameworkInfo>,
}

#[derive(Serialize)]
struct FrameworkInfo {
    language: String,
    name: String,
    url: String,
    tags: HashMap<String, String>,
}

#[derive(Serialize)]
struct RunsResponse {
    runs: Vec<RunInfo>,
}

#[derive(Serialize)]
struct RunInfo {
    id: u32,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
struct RunResultsResponse {
    results: Vec<RunResult>,
}

pub fn create_router(db: db::Db) -> Router {
    Router::new()
        .route("/tags", get(get_tags))
        .route("/environments", get(get_environments))
        .route("/tests", get(get_tests))
        .route("/languages", get(get_languages))
        .route("/frameworks", get(get_frameworks))
        .route("/runs", get(get_runs))
        .route(
            "/runs/{run_id}/environments/{env}/tests/{test}",
            get(get_run_results),
        )
        .with_state(db)
}

async fn get_tags(State(db): State<db::Db>) -> Result<Json<TagsResponse>, StatusCode> {
    let tags = db.get_tag_keys().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(TagsResponse { tags }))
}

async fn get_environments(State(db): State<db::Db>) -> Result<Json<EnvironmentsResponse>, StatusCode> {
    let environments = db
        .get_environments()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|e| e.to_string())
        .collect();
    Ok(Json(EnvironmentsResponse { environments }))
}

async fn get_tests(State(_db): State<db::Db>) -> Result<Json<TestsResponse>, StatusCode> {
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
    Ok(Json(TestsResponse { tests }))
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

async fn get_languages(State(db): State<db::Db>) -> Result<Json<LanguagesResponse>, StatusCode> {
    let languages = db
        .get_languages()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|l| LanguageInfo {
            name: l.name,
            url: l.url,
        })
        .collect();
    Ok(Json(LanguagesResponse { languages }))
}

async fn get_frameworks(State(db): State<db::Db>) -> Result<Json<FrameworksResponse>, StatusCode> {
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
    Ok(Json(FrameworksResponse { frameworks }))
}

async fn get_runs(State(db): State<db::Db>) -> Result<Json<RunsResponse>, StatusCode> {
    let runs = db
        .get_runs()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|r| RunInfo {
            id: r.id,
            created_at: r.manifest.created_at,
        })
        .collect();
    Ok(Json(RunsResponse { runs }))
}

async fn get_run_results(
    State(db): State<db::Db>,
    Path((run_id, env_str, test_str)): Path<(u32, String, String)>,
) -> Result<Json<RunResultsResponse>, StatusCode> {
    let environment = BenchmarkEnvironmentType::from_str(&env_str, true)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let test = test_str
        .as_str()
        .try_into()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let results = db
        .get_run_results(run_id, environment, test)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(RunResultsResponse { results }))
}