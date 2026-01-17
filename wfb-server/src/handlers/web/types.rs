use crate::view_models::{EnvironmentView, RunView, TestView};

pub use crate::routes::tpl::Routes;

pub const BACKEND_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const REPOSITORY_URL: &str = env!("CARGO_PKG_REPOSITORY");
pub const CONTACT_EMAIL: &str = env!("WFB_CONTACT_EMAIL");

pub struct ChromeContext {
    pub backend_version: &'static str,
    pub render_duration: super::render::RenderDuration,
    pub show_header_controls: bool,
    pub repository_url: &'static str,
    pub contact_email: &'static str,
    pub github_stars: String,
}

pub struct SelectionContext {
    pub runs: Vec<RunView>,
    pub active_run_id: String,
    pub environments: Vec<EnvironmentView>,
    pub active_env: String,
    pub tests: Vec<TestView>,
    pub active_test: String,
}

pub struct BenchDetailView {
    pub run_id: String,
    pub env: String,
    pub test: String,
    pub framework: String,
    pub language: String,
    pub framework_version: String,
    pub language_version: String,
    pub database: Option<String>,
    pub repo_url: Option<String>,
    pub path: String,
    pub tags: Vec<(String, String)>,
    pub rps: f64,
    pub tps: u64,
    pub latency_p99: u64,
    pub errors: u64,
}

pub struct BenchmarkView {
    pub framework: String,
    pub framework_version: String,
    pub language: String,
    pub language_color: String,
    pub rps: f64,
    pub rps_percent: f64,
    pub tps: u64,
    pub latency_p99: u64,
    pub errors: u64,
    pub database: Option<String>,
    pub tags: Vec<(String, String)>,
}

#[derive(serde::Deserialize)]
pub struct IndexQuery {
    pub run: Option<String>,
    pub env: Option<String>,
    pub test: Option<String>,
}
