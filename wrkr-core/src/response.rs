use mlua::{LuaSerdeExt, UserData, UserDataFields, UserDataRef};
use reqwest::header::HeaderMap;

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
        let headers_size = headers
            .iter()
            .map(|(k, v)| k.as_str().len() + v.len() + 4)
            .sum::<usize>()
            + 12;
        let b = res.bytes().await.unwrap_or_default();
        let len = b.len();
        Ok(Self {
            status,
            headers,
            body: b,
            body_len: len,
            headers_size,
        })
    }

    // Helper to get body as Bytes
    #[allow(dead_code)]
    pub fn get_body(&self) -> bytes::Bytes {
        self.body.clone()
    }

    pub fn status(&self) -> u16 {
        self.status
    }

    /// Returns total response size (body + headers + status line approximation)
    pub fn total_size(&self) -> usize {
        self.body_len + self.headers_size
    }
}

impl UserData for Response {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("bytes", |lua, this| lua.create_string(&this.body));

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
        methods.add_method("status", |_, this, ()| Ok(this.status));

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

        methods.add_method(
            "check_body_resp",
            |_, this, other: UserDataRef<Response>| Ok(this.body == other.body),
        );

        methods.add_method(
            "check_body_resp_prefix",
            |_, this, (other, len): (UserDataRef<Response>, usize)| {
                if len > other.body.len() {
                    return Ok(false);
                }
                Ok(this.body.as_ref() == &other.body[..len])
            },
        );

        methods.add_method("grpc_scanner", |_, this, ()| {
            let bytes = &this.body;
            if bytes.len() < 5 {
                return Ok(None);
            }
            let compressed = bytes[0] == 1;

            let mut len_bytes = [0u8; 4];
            len_bytes.copy_from_slice(&bytes[1..5]);
            let len = u32::from_be_bytes(len_bytes) as usize;

            if bytes.len() < 5 + len {
                return Ok(None);
            }

            // Slice exact payload
            let slice = this.body.slice(5..5 + len);

            if compressed {
                use flate2::read::GzDecoder;
                use std::io::Read;

                let mut decoder = GzDecoder::new(&slice[..]);
                let mut decompressed = Vec::new();
                if decoder.read_to_end(&mut decompressed).is_ok() {
                    return Ok(Some(crate::pb_utils::PbScanner::new(decompressed)));
                }
                // If decompression fails, we return None or try parsing as is?
                // Returning None is safer as it indicates invalid frame
                return Ok(None);
            }

            Ok(Some(crate::pb_utils::PbScanner::new_from_bytes(slice, 0)))
        });
    }
}
