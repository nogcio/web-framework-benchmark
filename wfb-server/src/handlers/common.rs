use std::collections::HashMap;
use wfb_storage::{BenchmarkTests, Config, RunManifest, StorageData};

use crate::api_models::{EnvironmentInfo, RunSummary, TestInfo};

pub fn get_all_runs(
    data: &StorageData,
    runs_manifests: &HashMap<String, RunManifest>,
) -> Vec<RunSummary> {
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
    runs
}

pub fn get_available_tests() -> Vec<TestInfo> {
    vec![
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
            icon: "server".to_string(),
            children: vec![],
        },
        TestInfo {
            id: Some(BenchmarkTests::DbComplex.to_string()),
            name: "Database".to_string(),
            icon: "database".to_string(),
            children: vec![],
        },
    ]
}

pub fn get_run_environments(
    run_id: &str,
    data: &StorageData,
    config: &Config,
) -> Vec<EnvironmentInfo> {
    let run_data = data.get(run_id);
    let mut environment_names = Vec::new();
    if let Some(r_data) = run_data {
        for k in r_data.keys() {
            environment_names.push(k.clone());
        }
    }
    environment_names.sort();

    environment_names
        .into_iter()
        .map(|name| {
            if let Some(env) = config.get_environment(&name) {
                EnvironmentInfo {
                    name: name.clone(),
                    display_name: env.title().to_string(),
                    spec: env.spec().map(|s| s.to_string()),
                    icon: env.icon().unwrap_or("laptop").to_string(),
                }
            } else {
                EnvironmentInfo {
                    name: name.clone(),
                    display_name: name.clone(),
                    spec: None,
                    icon: "laptop".to_string(),
                }
            }
        })
        .collect()
}
