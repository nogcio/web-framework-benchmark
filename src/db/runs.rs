use std::collections::HashMap;

use crate::{BenchmarkEnvironmentType, benchmark::BenchmarkTests};

use super::languages;

fn serialize_duration_as_nanos<S>(duration: &std::time::Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u128(duration.as_nanos())
}

fn serialize_latency_distribution<S>(dist: &Vec<(u8, std::time::Duration)>, serializer: S) -> Result<S::Ok, S::Error>
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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RunManifest {
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct FrameworkManifest {
    pub version: String,
    pub tags: HashMap<String, String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RunData {
    pub requests_per_sec: f64,
    pub transfer_per_sec: u64,
    pub latency_avg: std::time::Duration,
    pub latency_stdev: std::time::Duration,
    pub latency_max: std::time::Duration,
    pub latency_stdev_pct: f64,
    pub latency_distribution: Vec<(u8, std::time::Duration)>,
    pub req_per_sec_avg: f64,
    pub req_per_sec_stdev: f64,
    pub req_per_sec_max: f64,
    pub req_per_sec_stdev_pct: f64,
    pub errors: i64,
    pub memory_usage: u64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Run {
    pub id: u32,
    pub manifest: RunManifest,
    pub frameworks: Vec<FrameworkRun>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FrameworkRun {
    pub environment: BenchmarkEnvironmentType,
    pub language: String,
    pub framework: String,
    pub manifest: FrameworkManifest,
    pub results: HashMap<BenchmarkTests, RunData>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FrameworkWithLanguage {
    pub language: String,
    pub framework: languages::Framework,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct RunResult {
    pub language: String,
    pub framework: String,
    pub version: String,
    pub rps: f64,
    pub tps: u64,
    #[serde(serialize_with = "serialize_duration_as_nanos")]
    pub latency_avg: std::time::Duration,
    #[serde(serialize_with = "serialize_duration_as_nanos")]
    pub latency_stdev: std::time::Duration,
    #[serde(serialize_with = "serialize_duration_as_nanos")]
    pub latency_max: std::time::Duration,
    #[serde(serialize_with = "serialize_duration_as_nanos")]
    pub latency50: std::time::Duration,
    #[serde(serialize_with = "serialize_duration_as_nanos")]
    pub latency75: std::time::Duration,
    #[serde(serialize_with = "serialize_duration_as_nanos")]
    pub latency90: std::time::Duration,
    #[serde(serialize_with = "serialize_duration_as_nanos")]
    pub latency99: std::time::Duration,
    pub latency_stdev_pct: f64,
    #[serde(serialize_with = "serialize_latency_distribution")]
    pub latency_distribution: Vec<(u8, std::time::Duration)>,
    pub req_per_sec_avg: f64,
    pub req_per_sec_stdev: f64,
    pub req_per_sec_max: f64,
    pub req_per_sec_stdev_pct: f64,
    pub errors: i64,
    pub memory_usage: u64,
    pub tags: HashMap<String, String>,
}