use crate::{response::Response, stats::Stats};
use hdrhistogram::Histogram;
use mlua::{
    Lua, LuaSerdeExt, Result, Table, UserData, UserDataFields, UserDataMethods, UserDataRef, Value,
};
use reqwest::{Client, Method};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::{Instant, sleep};

struct LocalStats {
    requests: AtomicU64,
    bytes_received: AtomicU64,
    errors: Mutex<HashMap<String, u64>>,
    histogram: Mutex<Histogram<u64>>,
}

impl LocalStats {
    fn new() -> Option<Self> {
        Some(Self {
            requests: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            errors: Mutex::new(HashMap::new()),
            histogram: Mutex::new(Histogram::new(3).ok()?),
        })
    }
}

#[derive(Clone)]
pub struct BenchmarkContext {
    vu_id: u64,
    client: Client,
    base_url: String,
    stats: Arc<Stats>,
    local_stats: Arc<LocalStats>,
    track_status_codes: Arc<AtomicBool>,
}

impl BenchmarkContext {
    pub fn new(client: Client, base_url: String, stats: Arc<Stats>, vu_id: u64) -> Option<Self> {
        Some(Self {
            client,
            base_url,
            stats,
            vu_id,
            local_stats: Arc::new(LocalStats::new()?),
            track_status_codes: Arc::new(AtomicBool::new(true)),
        })
    }

    pub fn vu_id(&self) -> u64 {
        self.vu_id
    }

    pub fn stats(&self) -> &Arc<Stats> {
        &self.stats
    }

    pub fn flush_stats(&self) {
        let requests = self.local_stats.requests.swap(0, Ordering::Relaxed);
        let bytes_received = self.local_stats.bytes_received.swap(0, Ordering::Relaxed);

        let mut errors_guard = match self.local_stats.errors.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        let mut histogram_guard = match self.local_stats.histogram.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };

        if requests > 0
            || bytes_received > 0
            || !errors_guard.is_empty()
            || !histogram_guard.is_empty()
        {
            self.stats
                .merge(requests, bytes_received, &errors_guard, &histogram_guard);

            errors_guard.clear();
            histogram_guard.reset();
        }
    }

    async fn process_response(
        &self,
        resp: reqwest::Result<reqwest::Response>,
        start: Instant,
    ) -> Result<Response> {
        match resp {
            Ok(r) => match Response::new(r).await {
                Ok(response) => {
                    let resp_size = response.total_size();

                    self.local_stats.requests.fetch_add(1, Ordering::Relaxed);
                    self.local_stats
                        .bytes_received
                        .fetch_add(resp_size as u64, Ordering::Relaxed);
                    let duration = start.elapsed();
                    {
                        let mut hist = self
                            .local_stats
                            .histogram
                            .lock()
                            .unwrap_or_else(|e| e.into_inner());
                        let _ = hist.record(duration.as_micros() as u64);
                    }

                    let status = response.status();
                    if self.track_status_codes.load(Ordering::Relaxed)
                        && !(200..400).contains(&status)
                    {
                        let mut errors = self
                            .local_stats
                            .errors
                            .lock()
                            .unwrap_or_else(|e| e.into_inner());
                        *errors
                            .entry("Non 2xx and non 3xx status code".to_owned())
                            .or_insert(0) += 1;
                    }
                    Ok(response)
                }
                Err(e) => {
                    self.local_stats.requests.fetch_add(1, Ordering::Relaxed);
                    let mut errors = self
                        .local_stats
                        .errors
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    *errors
                        .entry(format!("Response processing error: {}", e))
                        .or_insert(0) += 1;
                    Err(mlua::Error::external(e))
                }
            },
            Err(e) => {
                self.local_stats.requests.fetch_add(1, Ordering::Relaxed);
                let mut errors = self
                    .local_stats
                    .errors
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                if e.is_timeout() {
                    *errors.entry("Request timeout".to_owned()).or_insert(0) += 1;
                } else {
                    *errors.entry(format!("Request error: {}", e)).or_insert(0) += 1;
                }
                Err(mlua::Error::external(e))
            }
        }
    }
}

impl UserData for BenchmarkContext {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("vars", |lua, _| {
            lua.named_registry_value::<Value>("ctx_vars")
        });

        fields.add_field_method_set("vars", |lua, _, val: Value| {
            lua.set_named_registry_value("ctx_vars", val)
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("vu", |_, this, ()| Ok(this.vu_id));

        methods.add_method("track_status_codes", |_, this, enable: bool| {
            this.track_status_codes.store(enable, Ordering::Relaxed);
            Ok(())
        });

        methods.add_async_method("pace", |_, _, ms: u64| async move {
            sleep(Duration::from_millis(ms)).await;
            Ok(())
        });

        methods.add_method("assert", |_, _this, (condition, msg): (bool, String)| {
            if !condition {
                // We could throw a Lua error to stop execution of the current iteration
                return Err(mlua::Error::RuntimeError(format!(
                    "Assertion failed: {}",
                    msg
                )));
            }
            Ok(())
        });

        // Optimized GET method - no Table parsing overhead
        methods.add_async_method("get", |_, this, path: mlua::String| {
            let this = this.clone();
            let path_str = path.to_str().map(|s| s.to_string());

            async move {
                let path = path_str.map_err(mlua::Error::external)?;
                let url = if path.starts_with("http") {
                    path
                } else {
                    format!("{}{}", this.base_url, path)
                };

                let start = Instant::now();
                let resp = this.client.get(&url).send().await;

                this.process_response(resp, start).await
            }
        });

        // Optimized POST method - path + body (string or table->json), minimal parsing
        methods.add_async_method(
            "post",
            |lua: Lua, this, (path, body): (mlua::String, Value)| {
                let this = this.clone();
                let path_str = path.to_str().map(|s| s.to_string());

                async move {
                    let path = path_str.map_err(mlua::Error::external)?;
                    let url = if path.starts_with("http") {
                        path
                    } else {
                        format!("{}{}", this.base_url, path)
                    };

                    let mut req = this.client.post(&url);
                    match body {
                        Value::Nil => {}
                        Value::String(s) => {
                            let body = s.to_str().map_err(mlua::Error::external)?;
                            req = req.body(body.to_string());
                        }
                        Value::Table(t) => {
                            let json_val: serde_json::Value = lua.from_value(Value::Table(t))?;
                            req = req.json(&json_val);
                        }
                        _ => {}
                    }

                    let start = Instant::now();
                    let resp = req.send().await;

                    this.process_response(resp, start).await
                }
            },
        );

        methods.add_async_method(
            "http",
            |lua: Lua, this: UserDataRef<BenchmarkContext>, options: Table| {
                let this = this.clone();

                async move {
                    let method = if let Ok(m) = options.get::<mlua::String>("method") {
                        let s = m.to_str()?;
                        match s.to_uppercase().as_str() {
                            "GET" => Method::GET,
                            "POST" => Method::POST,
                            "PUT" => Method::PUT,
                            "DELETE" => Method::DELETE,
                            "PATCH" => Method::PATCH,
                            "HEAD" => Method::HEAD,
                            _ => Method::GET,
                        }
                    } else {
                        Method::GET
                    };

                    let url_str = if let Ok(u) = options.get::<mlua::String>("url") {
                        u.to_str()?.to_string()
                    } else {
                        "/".to_string()
                    };

                    let url = if url_str.starts_with("http") {
                        url_str
                    } else {
                        format!("{}{}", this.base_url, url_str)
                    };

                    let mut req = this.client.request(method.clone(), &url);

                    if let Ok(h) = options.get::<Table>("headers") {
                        for pair in h.pairs::<mlua::String, mlua::String>() {
                            let (k, v) = pair?;
                            let k_str = k.to_str()?;
                            let v_str = v.to_str()?;
                            req = req.header(k_str.to_string(), v_str.to_string());
                        }
                    }

                    let body: Option<Value> = options.get("body").ok();
                    if let Some(b) = body {
                        match b {
                            Value::String(s) => {
                                // Use as_bytes to support binary data (like Protobuf)
                                let bytes = s.as_bytes().to_vec();
                                req = req.body(bytes);
                            }
                            Value::Table(t) => {
                                let json_val: serde_json::Value =
                                    lua.from_value(Value::Table(t))?;
                                req = req.json(&json_val);
                            }
                            _ => {}
                        }
                    }

                    let start = Instant::now();
                    let resp = req.send().await;

                    this.process_response(resp, start).await
                }
            },
        );
    }
}

pub fn create_lua_env(
    client: Client,
    base_url: String,
    stats: Arc<Stats>,
    vu_id: u64,
) -> Result<(Lua, BenchmarkContext)> {
    let lua = Lua::new();
    crate::pb_utils::register_utils(&lua)?;
    let ctx = BenchmarkContext::new(client, base_url, stats, vu_id).ok_or_else(|| {
        mlua::Error::RuntimeError("Failed to create benchmark context (out of memory?)".to_string())
    })?;

    Ok((lua, ctx))
}
