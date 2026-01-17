use super::context::select_common;
use super::github::github_stars_value_string;
use super::render::HtmlTemplate;
use super::types::{ChromeContext, IndexQuery, Routes, SelectionContext};
use askama::Template;
use axum::extract::State;
use axum::response::IntoResponse;
use std::sync::Arc;
use std::time::Instant;

use crate::handlers::web::context::chrome_context;
use crate::state::AppState;

#[allow(unused_imports)]
use crate::filters;

#[derive(Template)]
#[template(path = "pages/methodology.rs.j2")]
struct MethodologyTemplate {
    chrome: ChromeContext,
    selection: SelectionContext,
    routes: Routes,
}

pub async fn methodology_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let render_started = Instant::now();
    let github_stars = github_stars_value_string().await;
    let data = state.storage.data.read().unwrap();
    let runs_manifests = state.storage.runs.read().unwrap();
    let config = state.config.read().unwrap();

    let query = IndexQuery {
        run: None,
        env: None,
        test: None,
    };
    let selection = select_common(&data, &runs_manifests, &config, &query);

    HtmlTemplate(MethodologyTemplate {
        chrome: chrome_context(render_started, false, github_stars),
        selection,
        routes: Routes,
    })
}
