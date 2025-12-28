use reqwest::header::HeaderMap;
use mlua::{LuaSerdeExt, UserData, UserDataFields, UserDataRef};


pub struct Response {
    status: u16,
    headers: HeaderMap,
    body: bytes::Bytes,
    body_len: usize,
    headers_size: usize,
}

impl Response {
    pub async fn new(res: reqwest::Response) -> Result<Self, reqwest::Error> {
        let status = res.status().as_u16();
        let headers = res.headers().clone();
        // Calculate headers size once during construction
        let headers_size = headers.iter().map(|(k, v)| k.as_str().len() + v.len() + 4).sum::<usize>() + 12;
        let b = res.bytes().await.unwrap_or_default();
        let len = b.len();
        Ok(Self { status, headers, body: b, body_len: len, headers_size })
    }

    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn body(&self) -> &bytes::Bytes {
        &self.body
    }

    pub fn body_len(&self) -> usize {
        self.body_len
    }
    
    /// Returns total response size (body + headers + status line approximation)
    pub fn total_size(&self) -> usize {
        self.body_len + self.headers_size
    }
}

impl UserData for Response {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("status", |_, this| Ok(this.status));
        fields.add_field_method_get("bytes", |_, this| Ok(this.body.to_vec()));
        
        fields.add_field_method_get("headers", |lua, this| {
            let t = lua.create_table()?;
            for (k, v) in this.headers.iter() {
                if let Ok(val_str) = v.to_str() {
                    t.set(k.as_str(), val_str)?;
                }
            }
            Ok(t)
        });
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("header", |_, this, name: String| {
            if let Some(Ok(s)) = this.headers.get(&name).map(|val| val.to_str()) {
                return Ok(Some(s.to_string()));
            }
            Ok(None)
        });

        methods.add_method("text", |_, this, ()| {
            let s = String::from_utf8_lossy(&this.body).to_string();
            Ok(s)
        });

        methods.add_method("json", |lua, this, ()| {
            let v: serde_json::Value = serde_json::from_slice(&this.body)
                .map_err(|e| mlua::Error::RuntimeError(format!("JSON decode error: {}", e)))?;
            lua.to_value(&v)
        });

        methods.add_method("check_body", |_, this, val: mlua::String| {
            Ok(val.as_bytes() == this.body.as_ref())
        });

        methods.add_method("check_body_resp", |_, this, other: UserDataRef<Response>| {
            Ok(this.body == other.body)
        });

        methods.add_method("check_body_resp_prefix", |_, this, (other, len): (UserDataRef<Response>, usize)| {
            if len > other.body.len() {
                return Ok(false);
            }
            Ok(this.body.as_ref() == &other.body[..len])
        });
    }
}