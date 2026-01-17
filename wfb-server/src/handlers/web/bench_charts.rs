use super::render::HtmlTemplate;
use askama::Template;
use axum::extract::Extension;
use axum::extract::State;
use axum::response::IntoResponse;
use serde::Serialize;
use std::sync::Arc;

use crate::middleware::CspNonce;
use crate::routes;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct BenchChartClientData {
    pub x: Vec<f64>,
    pub rps: Vec<f64>,
    pub p99_ms: Vec<f64>,
    pub mem_mb: Vec<f64>,
}

#[derive(Template)]
#[template(path = "partials/bench/charts.rs.j2")]
struct BenchChartsPartialTemplate {
    chart_data: BenchChartClientData,
    csp_nonce: String,
}

pub async fn bench_charts_partials_path_handler(
    State(state): State<Arc<AppState>>,
    Extension(CspNonce(csp_nonce)): Extension<CspNonce>,
    params: routes::BenchChartsPartialsViewPath,
) -> impl IntoResponse {
    let lang = find_language_for_framework(
        &state,
        &params.run,
        &params.env,
        &params.test,
        &params.framework,
    );

    let raw = lang
        .as_deref()
        .and_then(|lang| {
            state.storage.get_raw_data(
                &params.run,
                &params.env,
                lang,
                &params.framework,
                &params.test,
            )
        })
        .unwrap_or_default();

    HtmlTemplate(BenchChartsPartialTemplate {
        chart_data: build_chart_data(&raw, 240),
        csp_nonce,
    })
}

fn to_ms(nanos: u64) -> f64 {
    // Raw latency values are microseconds.
    // Convert to milliseconds.
    nanos as f64 / 1_000.0
}

fn to_mb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

fn build_chart_data(raw: &[wfb_storage::TestCaseRaw], max_points: usize) -> BenchChartClientData {
    if raw.is_empty() {
        return BenchChartClientData {
            x: Vec::new(),
            rps: Vec::new(),
            p99_ms: Vec::new(),
            mem_mb: Vec::new(),
        };
    }

    let mut data: Vec<&wfb_storage::TestCaseRaw> = raw.iter().collect();
    data.sort_by_key(|r| r.elapsed_secs);

    let step = if max_points == 0 {
        1
    } else {
        (data.len() as f64 / max_points as f64).ceil() as usize
    }
    .max(1);

    let mut x = Vec::with_capacity(data.len().min(max_points));
    let mut rps = Vec::with_capacity(data.len().min(max_points));
    let mut p99_ms = Vec::with_capacity(data.len().min(max_points));
    let mut mem_mb = Vec::with_capacity(data.len().min(max_points));

    for r in data.into_iter().step_by(step) {
        x.push(r.elapsed_secs as f64);
        rps.push(r.requests_per_sec);
        p99_ms.push(to_ms(r.latency_p99));
        mem_mb.push(to_mb(r.memory_usage_bytes));
    }

    BenchChartClientData {
        x,
        rps,
        p99_ms,
        mem_mb,
    }
}

fn find_language_for_framework(
    state: &AppState,
    run: &str,
    env: &str,
    test: &str,
    framework: &str,
) -> Option<String> {
    let data = state.storage.data_read();
    let run_data = data.get(run)?;
    let env_data = run_data.get(env)?;

    for (lang, lang_data) in env_data {
        if let Some(bench_result) = lang_data.get(framework)
            && bench_result.test_cases.contains_key(test)
        {
            return Some(lang.clone());
        }
    }

    None
}
