use crate::handlers::common;
use crate::view_models::{EnvironmentView, RunView, TestView};
use std::collections::HashMap;
use std::sync::RwLockReadGuard;
use wfb_storage::StorageData;

use super::types::REPOSITORY_URL;

pub fn select_important_table_tags(tags: &HashMap<String, String>) -> Vec<(String, String)> {
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

pub fn benchmark_repo_url(path: &str) -> Option<String> {
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

pub fn get_runs(
    data: &RwLockReadGuard<'_, StorageData>,
    runs_manifests: &RwLockReadGuard<'_, HashMap<String, wfb_storage::RunManifest>>,
) -> Vec<RunView> {
    common::get_all_runs(data, runs_manifests)
        .into_iter()
        .map(|r| RunView {
            id: r.id,
            created_at: r.created_at,
        })
        .collect()
}

pub fn get_environment_views(
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

pub fn get_available_tests() -> Vec<TestView> {
    common::get_available_tests()
        .into_iter()
        .map(|t| TestView {
            id: t.id.unwrap_or_default(),
            name: t.name,
            icon: t.icon,
        })
        .collect()
}
