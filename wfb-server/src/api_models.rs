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
