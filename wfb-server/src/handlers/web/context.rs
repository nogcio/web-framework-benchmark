use super::helpers::{get_available_tests, get_environment_views, get_runs};
use super::types::{
    BACKEND_VERSION, CONTACT_EMAIL, ChromeContext, IndexQuery, REPOSITORY_URL, SelectionContext,
};
use std::collections::HashMap;
use std::sync::RwLockReadGuard;
use std::time::Instant;
use wfb_storage::StorageData;

pub fn chrome_context(
    render_started: Instant,
    show_header_controls: bool,
    github_stars: String,
    page_path: &str,
) -> ChromeContext {
    let page_url = crate::public_url::page_url(page_path);

    ChromeContext {
        backend_version: BACKEND_VERSION,
        render_duration: super::render::RenderDuration::new(render_started),
        show_header_controls,
        repository_url: REPOSITORY_URL,
        page_url,
        contact_email: CONTACT_EMAIL,
        github_stars,
    }
}

pub fn empty_selection_context() -> SelectionContext {
    SelectionContext {
        runs: vec![],
        active_run_id: String::new(),
        environments: vec![],
        active_env: String::new(),
        tests: vec![],
        active_test: String::new(),
    }
}

pub fn select_common(
    data: &RwLockReadGuard<'_, StorageData>,
    runs_manifests: &RwLockReadGuard<'_, HashMap<String, wfb_storage::RunManifest>>,
    config: &wfb_storage::Config,
    query: &IndexQuery,
) -> SelectionContext {
    let runs = get_runs(data, runs_manifests);

    // If there are no runs, keep everything empty.
    if runs.is_empty() {
        return SelectionContext {
            runs,
            active_run_id: String::new(),
            environments: vec![],
            active_env: String::new(),
            tests: vec![],
            active_test: String::new(),
        };
    }

    let active_run_id = query
        .run
        .clone()
        .unwrap_or_else(|| runs.first().map(|r| r.id.clone()).unwrap_or_default());

    let environments = if !active_run_id.is_empty() {
        get_environment_views(&active_run_id, data, config)
    } else {
        vec![]
    };

    let active_env = query.env.clone().unwrap_or_else(|| {
        environments
            .first()
            .map(|e| e.name.clone())
            .unwrap_or_default()
    });

    let tests = get_available_tests();
    let active_test = query
        .test
        .clone()
        .unwrap_or_else(|| tests.first().map(|t| t.id.clone()).unwrap_or_default());

    SelectionContext {
        runs,
        active_run_id,
        environments,
        active_env,
        tests,
        active_test,
    }
}
