use axum::extract::State;
use axum::response::{IntoResponse, Redirect};
use std::sync::Arc;

use crate::handlers::web::types::REPOSITORY_URL;
use crate::state::AppState;

pub async fn methodology_handler(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    let docs_url = format!(
        "{}/blob/main/docs/METHODOLOGY.md",
        REPOSITORY_URL.trim_end_matches('/')
    );
    Redirect::to(&docs_url)
}
