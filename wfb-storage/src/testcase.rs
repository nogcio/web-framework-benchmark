use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestCaseRaw {
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
    pub latency_distribution: Vec<(u8, u64)>,

    pub errors: HashMap<String, u64>,

    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: f64,

    pub req_per_sec_avg: f64,
    pub req_per_sec_stdev: f64,
    pub req_per_sec_max: f64,
    pub req_per_sec_stdev_pct: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestCaseSummary {
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
    pub latency_distribution: Vec<(u8, u64)>,

    pub errors: HashMap<String, u64>,
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: f64,

    pub req_per_sec_avg: f64,
    pub req_per_sec_stdev: f64,
    pub req_per_sec_max: f64,
    pub req_per_sec_stdev_pct: f64,
}
