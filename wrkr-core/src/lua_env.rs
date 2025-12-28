use mlua::{Lua, Result, UserData, UserDataMethods, UserDataRef, Value, Table, UserDataFields, LuaSerdeExt};
use reqwest::{Client, Method};
use std::sync::Arc;
use crate::{response::Response, stats::Stats};
use std::time::Duration;
use tokio::time::{sleep, Instant};

#[derive(Clone)]
pub struct BenchmarkContext {
    vu_id: u64,
    client: Client,
    base_url: String,
    stats: Arc<Stats>,
}

impl BenchmarkContext {
    pub fn new(client: Client, base_url: String, stats: Arc<Stats>, vu_id: u64) -> Self {
        Self {
            client,
            base_url,
            stats,
            vu_id
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
                this.stats.record_error(msg.clone());
                // We could throw a Lua error to stop execution of the current iteration
                return Err(mlua::Error::RuntimeError(format!("Assertion failed: {}", msg)));
            }
            Ok(())
        });

        methods.add_async_method("http", |lua: Lua, this: UserDataRef<BenchmarkContext>, options: Table| {
            let this = this.clone();
            async move {
                let method_str: String = options.get("method").unwrap_or("GET".to_string());
                let url_str: String = options.get("url").unwrap_or("/".to_string());
                let headers: Option<Table> = options.get("headers").ok();
                let body: Option<Value> = options.get("body").ok();

                let url = if url_str.starts_with("http") {
                    url_str.clone()
                } else {
                    format!("{}{}", this.base_url, url_str)
                };

                let method = match method_str.to_uppercase().as_str() {
                    "GET" => Method::GET,
                    "POST" => Method::POST,
                    "PUT" => Method::PUT,
                    "DELETE" => Method::DELETE,
                    "PATCH" => Method::PATCH,
                    "HEAD" => Method::HEAD,
                    _ => Method::GET,
                };

                let mut req = this.client.request(method, &url);

                let mut req_size = method_str.len() + url_str.len();
                if let Some(h) = headers {
                    for pair in h.pairs::<String, String>() {
                        let (k, v) = pair?;
                        req_size += k.len() + v.len() + 4; // key + value + ": " + "\r\n"
                        req = req.header(k, v);
                    }
                }

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
                
                this.stats.add_bytes_sent(req_size as u64);

                let start = Instant::now();
                let resp = req.send().await;
                this.stats.inc_requests();

                match resp {
                    Ok(r) => {
                        match Response::new(r).await {
                            Ok(response) => {
                                let resp_size = response.body().len() + response.headers().iter().map(|(k, v)| k.as_str().len() + v.len() + 4).sum::<usize>() + 12; // status line approx
                                this.stats.add_bytes_received(resp_size as u64);
                                let duration = start.elapsed();
                                this.stats.record_latency(duration);
                                
                                let status = response.status();
                                if !(200..400).contains(&status) {
                                    this.stats.record_error("Non 2xx and non 3xx status code".to_owned());
                                }
                                Ok(response)
                            },
                            Err(e) => {
                                this.stats.record_error(format!("Response processing error: {}", e));
                                Err(mlua::Error::external(e))
                            }
                        }
                    },
                    Err(e) => {
                        if e.is_timeout() {
                            this.stats.record_error("Request timeout".to_owned());
                        } else {
                            this.stats.record_error(format!("Request error: {}", e));
                        }
                        Err(mlua::Error::external(e))
                    }
                }
            }
        });
    }
}


pub fn create_lua_env(base_url: String, stats: Arc<Stats>, vu_id: u64) -> Result<(Lua, BenchmarkContext)> {
    let lua = Lua::new();
    let client = Client::new();
    let ctx = BenchmarkContext::new(client, base_url, stats, vu_id);
    
    Ok((lua, ctx))
}
