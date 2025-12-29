use dashmap::DashMap;
use hdrhistogram::Histogram;
use std::collections::HashMap;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

#[derive(Debug)]
pub struct Stats {
    pub connections: AtomicU64,
    pub total_requests: AtomicU64,
    pub total_errors: AtomicU64,
    pub total_bytes_received: AtomicU64,
    pub errors_map: DashMap<String, u64>,
    pub latency_histogram: Arc<Mutex<Histogram<u64>>>,
}

impl Stats {
    pub fn new() -> Self {
        Self {
            connections: AtomicU64::new(0),
            total_requests: AtomicU64::new(0),
            total_errors: AtomicU64::new(0),
            total_bytes_received: AtomicU64::new(0),
            errors_map: DashMap::new(),
            latency_histogram: Arc::new(Mutex::new(Histogram::new(3).unwrap_or_else(|_| {
                eprintln!("Failed to create histogram, using default");
                Histogram::new(2).expect("Failed to create fallback histogram")
            }))),
        }
    }

    pub fn inc_connections(&self) {
        self.connections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_error(&self, error: String) {
        self.total_errors.fetch_add(1, Ordering::Relaxed);
        *self.errors_map.entry(error).or_insert(0) += 1;
    }

    pub fn merge(&self, requests: u64, bytes_received: u64, errors: &HashMap<String, u64>, histogram: &Histogram<u64>) {
        self.total_requests.fetch_add(requests, Ordering::Relaxed);
        self.total_bytes_received.fetch_add(bytes_received, Ordering::Relaxed);
        
        let error_count: u64 = errors.values().sum();
        self.total_errors.fetch_add(error_count, Ordering::Relaxed);

        for (k, v) in errors {
            *self.errors_map.entry(k.clone()).or_insert(0) += v;
        }
        
        if let Ok(mut h) = self.latency_histogram.lock() {
            let _ = h.add(histogram);
        }
    }

    pub fn snapshot(&self, duration: Duration, elapsed: Duration, rps_samples: Vec<f64>) -> StatsSnapshot {
        StatsSnapshot {
            duration,
            elapsed,
            connections: self.connections.load(Ordering::Relaxed),
            total_requests: self.total_requests.load(Ordering::Relaxed),
            total_errors: self.total_errors.load(Ordering::Relaxed),
            total_bytes_received: self.total_bytes_received.load(Ordering::Relaxed),
            errors: self
                .errors_map
                .iter()
                .map(|r| (r.key().clone(), *r.value()))
                .collect(),
            latency_histogram: if let Ok(hist) = self.latency_histogram.lock() {
                hist.clone()
            } else {
                Histogram::new(3).unwrap_or_else(|_| Histogram::new(2).expect("Failed to create fallback histogram"))
            },
            rps_samples,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StatsSnapshot {
    pub duration: Duration,
    pub elapsed: Duration,
    pub connections: u64,
    pub total_requests: u64,
    pub total_errors: u64,
    pub total_bytes_received: u64,
    pub errors: HashMap<String, u64>,
    pub latency_histogram: Histogram<u64>,
    pub rps_samples: Vec<f64>,
}
