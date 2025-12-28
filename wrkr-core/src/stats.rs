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
    pub total_bytes_sent: AtomicU64,
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
            total_bytes_sent: AtomicU64::new(0),
            total_bytes_received: AtomicU64::new(0),
            errors_map: DashMap::new(),
            latency_histogram: Arc::new(Mutex::new(Histogram::new(3).unwrap())),
        }
    }

    pub fn inc_connections(&self) {
        self.connections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_requests(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_errors(&self) {
        self.total_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_latency(&self, latency: Duration) {
        if let Ok(mut hist) = self.latency_histogram.lock() {
            let _ = hist.record(latency.as_micros() as u64);
        }
    }

    pub fn record_error(&self, error: String) {
        self.inc_errors();
        *self.errors_map.entry(error).or_insert(0) += 1;
    }

    pub fn add_bytes_sent(&self, bytes: u64) {
        self.total_bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn add_bytes_received(&self, bytes: u64) {
        self.total_bytes_received
            .fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn snapshot(&self, duration: Duration, elapsed: Duration) -> StatsSnapshot {
        StatsSnapshot {
            duration,
            elapsed,
            connections: self.connections.load(Ordering::Relaxed),
            total_requests: self.total_requests.load(Ordering::Relaxed),
            total_errors: self.total_errors.load(Ordering::Relaxed),
            total_bytes_sent: self.total_bytes_sent.load(Ordering::Relaxed),
            total_bytes_received: self.total_bytes_received.load(Ordering::Relaxed),
            errors: self
                .errors_map
                .iter()
                .map(|r| (r.key().clone(), *r.value()))
                .collect(),
            latency_histogram: if let Ok(hist) = self.latency_histogram.lock() {
                hist.clone()
            } else {
                Histogram::new(3).unwrap()
            },
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
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub errors: HashMap<String, u64>,
    pub latency_histogram: Histogram<u64>,
}
