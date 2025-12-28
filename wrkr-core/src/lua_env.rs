use mlua::{Lua, Result, UserData, UserDataMethods, UserDataRef, Value, Table, UserDataFields, LuaSerdeExt};
use reqwest::{Client, Method};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use crate::{response::Response, stats::Stats};
use std::time::Duration;
use tokio::time::{sleep, Instant};
use hdrhistogram::Histogram;
use std::collections::HashMap;

struct LocalStats {
    requests: AtomicU64,
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
    errors: Mutex<HashMap<String, u64>>,
    histogram: Mutex<Histogram<u64>>,
}

impl LocalStats {
    fn new() -> Self {
        Self {
            requests: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            errors: Mutex::new(HashMap::new()),
            histogram: Mutex::new(Histogram::new(3).unwrap()),
        }
    }
}

#[derive(Clone)]
pub struct BenchmarkContext {
    vu_id: u64,
    client: Client,
    base_url: String,
    stats: Arc<Stats>,
    local_stats: Arc<LocalStats>,
}

impl BenchmarkContext {
    pub fn new(client: Client, base_url: String, stats: Arc<Stats>, vu_id: u64) -> Self {
        Self {
            client,
            base_url,
            stats,
            vu_id,
            local_stats: Arc::new(LocalStats::new()),
        }
    }

    pub fn flush_stats(&self) {
        let requests = self.local_stats.requests.swap(0, Ordering::Relaxed);
        let bytes_sent = self.local_stats.bytes_sent.swap(0, Ordering::Relaxed);
        let bytes_received = self.local_stats.bytes_received.swap(0, Ordering::Relaxed);
        
        let mut errors_guard = self.local_stats.errors.lock().unwrap();
        let mut histogram_guard = self.local_stats.histogram.lock().unwrap();
        
        if requests > 0 || bytes_sent > 0 || bytes_received > 0 || !errors_guard.is_empty() || !histogram_guard.is_empty() {
             self.stats.merge(
                requests,
                bytes_sent,
                bytes_received,
                &errors_guard,
                &histogram_guard
            );
            
            errors_guard.clear();
            histogram_guard.reset();
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
        methods.add_method("vu", |_, this, ()| {
            Ok(this.vu_id)
        });

        methods.add_async_method("pace", |_, _, ms: u64| {
            async move {
                sleep(Duration::from_millis(ms)).await;
                Ok(())
            }
        });

        methods.add_method("assert", |_, this, (condition, msg): (bool, String)| {
            if !condition {
                let mut errors = this.local_stats.errors.lock().unwrap();
                *errors.entry(msg.clone()).or_insert(0) += 1;
                // We could throw a Lua error to stop execution of the current iteration
                return Err(mlua::Error::RuntimeError(format!("Assertion failed: {}", msg)));
            }
            Ok(())
        });

        methods.add_async_method("http", |lua: Lua, this: UserDataRef<BenchmarkContext>, options: Table| {
            let this = this.clone();
            
            let prepare_result = (|| {
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

                let mut req_size = method.as_str().len() + url.len();
                
                if let Ok(h) = options.get::<Table>("headers") {
                    for pair in h.pairs::<mlua::String, mlua::String>() {
                        let (k, v) = pair?;
                        let k_str = k.to_str()?;
                        let v_str = v.to_str()?;
                        req_size += k_str.len() + v_str.len() + 4; // key + value + ": " + "\r\n"
                        req = req.header(k_str.to_string(), v_str.to_string());
                    }
                }

                let body: Option<Value> = options.get("body").ok();
                if let Some(b) = body {
                    match b {
                        Value::String(s) => {
                            let s_str = s.to_str()?;
                            req_size += s_str.len();
                            req = req.body(s_str.to_string());
                        },
                        Value::Table(t) => {
                            let json_val: serde_json::Value = lua.from_value(Value::Table(t))?;
                            let json_str = json_val.to_string();
                            req_size += json_str.len();
                            req = req.json(&json_val);
                        },
                        _ => {}
                    }
                }
                Ok((req, req_size))
            })();

            async move {
                let (req, req_size) = match prepare_result {
                    Ok(v) => v,
                    Err(e) => return Err(e),
                };

                this.local_stats.bytes_sent.fetch_add(req_size as u64, Ordering::Relaxed);

                let start = Instant::now();
                let resp = req.send().await;

                match resp {
                    Ok(r) => {
                        match Response::new(r).await {
                            Ok(response) => {
                                let resp_size = response.body_len() + response.headers().iter().map(|(k, v)| k.as_str().len() + v.len() + 4).sum::<usize>() + 12; // status line approx
                                
                                this.local_stats.requests.fetch_add(1, Ordering::Relaxed);
                                this.local_stats.bytes_received.fetch_add(resp_size as u64, Ordering::Relaxed);
                                let duration = start.elapsed();
                                {
                                    let mut hist = this.local_stats.histogram.lock().unwrap();
                                    let _ = hist.record(duration.as_micros() as u64);
                                }
                                
                                let status = response.status();
                                if !(200..400).contains(&status) {
                                    let mut errors = this.local_stats.errors.lock().unwrap();
                                    *errors.entry("Non 2xx and non 3xx status code".to_owned()).or_insert(0) += 1;
                                }
                                Ok(response)
                            },
                            Err(e) => {
                                this.local_stats.requests.fetch_add(1, Ordering::Relaxed);
                                let mut errors = this.local_stats.errors.lock().unwrap();
                                *errors.entry(format!("Response processing error: {}", e)).or_insert(0) += 1;
                                Err(mlua::Error::external(e))
                            }
                        }
                    },
                    Err(e) => {
                        this.local_stats.requests.fetch_add(1, Ordering::Relaxed);
                        let mut errors = this.local_stats.errors.lock().unwrap();
                        if e.is_timeout() {
                            *errors.entry("Request timeout".to_owned()).or_insert(0) += 1;
                        } else {
                            *errors.entry(format!("Request error: {}", e)).or_insert(0) += 1;
                        }
                        Err(mlua::Error::external(e))
                    }
                }
            }
        });
    }
}


pub fn create_lua_env(client: Client, base_url: String, stats: Arc<Stats>, vu_id: u64) -> Result<(Lua, BenchmarkContext)> {
    let lua = Lua::new();
    let ctx = BenchmarkContext::new(client, base_url, stats, vu_id);
    
    Ok((lua, ctx))
}
