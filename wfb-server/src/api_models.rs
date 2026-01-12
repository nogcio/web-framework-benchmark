use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestInfo {
    pub id: Option<String>,
    pub name: String,
    pub icon: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<TestInfo>,
}

#[derive(Serialize)]
pub struct LanguageInfo {
    pub name: String,
    pub url: String,
    pub color: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameworkInfo {
    pub language: String,
    pub name: String,
    pub url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkInfo {
    pub name: String,
    pub language: String,
    pub language_version: String,
    pub framework: String,
    pub framework_version: String,
    pub tests: Vec<String>,
    pub tags: HashMap<String, String>,
    pub path: String,
    pub database: String,
    pub disabled: bool,
    pub only: bool,
    pub arguments: Vec<String>,
    pub env: HashMap<String, String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentInfo {
    pub name: String,
    pub display_name: String,
    pub spec: Option<String>,
    pub icon: String,
}

#[derive(Serialize)]
pub struct VersionInfo {
    pub version: String,
}

#[derive(Deserialize)]
pub struct TranscriptParams {
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    pub id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

fn serialize_duration_as_nanos<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u128(duration.as_nanos())
}

fn serialize_latency_distribution<S>(
    dist: &Vec<(u8, Duration)>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeSeq;
    let mut seq = serializer.serialize_seq(Some(dist.len()))?;
    for (percent, duration) in dist {
        seq.serialize_element(&(percent, duration.as_nanos()))?;
    }
    seq.end()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunResult {
    pub name: String,
    pub language: String,
    pub language_version: String,
    pub framework: String,
    pub framework_version: String,
    pub database: Option<String>,
    pub path: Option<String>,
    pub rps: f64,
    pub tps: u64,
    #[serde(serialize_with = "serialize_duration_as_nanos")]
    pub latency_avg: Duration,
    #[serde(serialize_with = "serialize_duration_as_nanos")]
    pub latency_stdev: Duration,
    #[serde(serialize_with = "serialize_duration_as_nanos")]
    pub latency_max: Duration,
    #[serde(serialize_with = "serialize_duration_as_nanos")]
    pub latency50: Duration,
    #[serde(serialize_with = "serialize_duration_as_nanos")]
    pub latency75: Duration,
    #[serde(serialize_with = "serialize_duration_as_nanos")]
    pub latency90: Duration,
    #[serde(serialize_with = "serialize_duration_as_nanos")]
    pub latency99: Duration,
    pub latency_stdev_pct: f64,
    #[serde(serialize_with = "serialize_latency_distribution")]
    pub latency_distribution: Vec<(u8, Duration)>,
    pub req_per_sec_avg: f64,
    pub req_per_sec_stdev: f64,
    pub req_per_sec_max: f64,
    pub req_per_sec_stdev_pct: f64,
    pub errors: u64,
    pub memory_usage: u64,
    pub tags: HashMap<String, String>,
    pub has_transcript: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestCaseRawApi {
    pub elapsed_secs: u64,
    pub connections: u64,
    pub requests_per_sec: f64,
    pub bytes_per_sec: u64,
    pub total_requests: u64,
    pub total_bytes: u64,
    pub total_errors: u64,
    pub latency_mean: f64,
    pub latency_stdev: f64,
    pub latency_max: u64,
    pub latency_p50: u64,
    pub latency_p75: u64,
    pub latency_p90: u64,
    pub latency_p99: u64,
    pub latency_stdev_pct: f64,
    pub errors: HashMap<String, u64>,
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: f64,
    pub req_per_sec_avg: f64,
    pub req_per_sec_stdev: f64,
    pub req_per_sec_max: f64,
    pub req_per_sec_stdev_pct: f64,
}

impl From<wfb_storage::TestCaseRaw> for TestCaseRawApi {
    fn from(r: wfb_storage::TestCaseRaw) -> Self {
        Self {
            elapsed_secs: r.elapsed_secs,
            connections: r.connections,
            requests_per_sec: r.requests_per_sec,
            bytes_per_sec: r.bytes_per_sec,
            total_requests: r.total_requests,
            total_bytes: r.total_bytes,
            total_errors: r.total_errors,
            latency_mean: r.latency_mean,
            latency_stdev: r.latency_stdev,
            latency_max: r.latency_max,
            latency_p50: r.latency_p50,
            latency_p75: r.latency_p75,
            latency_p90: r.latency_p90,
            latency_p99: r.latency_p99,
            latency_stdev_pct: r.latency_stdev_pct,
            errors: r.errors,
            memory_usage_bytes: r.memory_usage_bytes,
            cpu_usage_percent: r.cpu_usage_percent,
            req_per_sec_avg: r.req_per_sec_avg,
            req_per_sec_stdev: r.req_per_sec_stdev,
            req_per_sec_max: r.req_per_sec_max,
            req_per_sec_stdev_pct: r.req_per_sec_stdev_pct,
        }
    }
}
