use bytes::Bytes;
use flate2::Compression;
use flate2::write::GzEncoder;
use mlua::{Lua, Result, UserData, UserDataMethods};
use prost::encoding::{WireType, encode_key, encode_varint};
use std::io::Write;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// --- Protobuf Builder for Lua ---

#[derive(Default, Clone)]
pub struct PbBuilder {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl PbBuilder {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::with_capacity(256))),
        }
    }
}

impl UserData for PbBuilder {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // Factory
        // methods.add_function("new", |_, ()| Ok(PbBuilder::new()));
        // Note: We'll export a helper function to create this, or expose constructor differently

        methods.add_method("bool", |_, this, (tag, val): (u32, bool)| {
            if val {
                let mut buf = this.buffer.lock().unwrap_or_else(|err| err.into_inner());
                encode_key(tag, WireType::Varint, &mut *buf);
                encode_varint(val as u64, &mut *buf);
            }
            Ok(this.clone())
        });

        methods.add_method("float", |_, this, (tag, val): (u32, f32)| {
            if val != 0.0 {
                let mut buf = this.buffer.lock().unwrap_or_else(|err| err.into_inner());
                encode_key(tag, WireType::ThirtyTwoBit, &mut *buf);
                buf.extend_from_slice(&val.to_le_bytes());
            }
            Ok(this.clone())
        });

        methods.add_method("double", |_, this, (tag, val): (u32, f64)| {
            if val != 0.0 {
                let mut buf = this.buffer.lock().unwrap_or_else(|err| err.into_inner());
                encode_key(tag, WireType::SixtyFourBit, &mut *buf);
                buf.extend_from_slice(&val.to_le_bytes());
            }
            Ok(this.clone())
        });

        methods.add_method("sint32", |_, this, (tag, val): (u32, i32)| {
            if val != 0 {
                let mut buf = this.buffer.lock().unwrap_or_else(|err| err.into_inner());
                encode_key(tag, WireType::Varint, &mut *buf);
                let encoded = (val as u32) << 1 ^ (val >> 31) as u32; // ZigZag
                encode_varint(encoded as u64, &mut *buf);
            }
            Ok(this.clone())
        });

        methods.add_method("sint64", |_, this, (tag, val): (u32, i64)| {
            if val != 0 {
                let mut buf = this.buffer.lock().unwrap_or_else(|err| err.into_inner());
                encode_key(tag, WireType::Varint, &mut *buf);
                let encoded = (val as u64) << 1 ^ (val >> 63) as u64; // ZigZag
                encode_varint(encoded, &mut *buf);
            }
            Ok(this.clone())
        });

        methods.add_method("int32", |_, this, (tag, val): (u32, i32)| {
            if val != 0 {
                let mut buf = this.buffer.lock().unwrap_or_else(|err| err.into_inner());
                encode_key(tag, WireType::Varint, &mut *buf);
                encode_varint(val as u64, &mut *buf);
            }
            Ok(this.clone())
        });

        methods.add_method("int64", |_, this, (tag, val): (u32, i64)| {
            if val != 0 {
                let mut buf = this.buffer.lock().unwrap_or_else(|err| err.into_inner());
                encode_key(tag, WireType::Varint, &mut *buf);
                encode_varint(val as u64, &mut *buf);
            }
            Ok(this.clone())
        });

        methods.add_method("string", |_, this, (tag, val): (u32, String)| {
            if !val.is_empty() {
                let mut buf = this.buffer.lock().unwrap_or_else(|err| err.into_inner());
                encode_key(tag, WireType::LengthDelimited, &mut *buf);
                encode_varint(val.len() as u64, &mut *buf);
                buf.extend_from_slice(val.as_bytes());
            }
            Ok(this.clone())
        });

        methods.add_method("bytes", |_, this, (tag, val): (u32, mlua::String)| {
            if !val.as_bytes().is_empty() {
                let mut buf = this.buffer.lock().unwrap_or_else(|err| err.into_inner());
                let bytes = val.as_bytes();
                encode_key(tag, WireType::LengthDelimited, &mut *buf);
                encode_varint(bytes.len() as u64, &mut *buf);
                buf.extend_from_slice(&bytes);
            }
            Ok(this.clone())
        });

        methods.add_method("raw_bytes", |_, this, val: mlua::String| {
            let mut buf = this.buffer.lock().unwrap_or_else(|err| err.into_inner());
            buf.extend_from_slice(&val.as_bytes());
            Ok(this.clone())
        });

        // Nested message support: we expect the user to pass encoded bytes of the nested message
        methods.add_method("message", |_, this, (tag, val): (u32, mlua::String)| {
            // For message, we generally encode even if empty if explicitly passed?
            // But following proto3 "default values are not encoded", empty message?
            // If byte string is empty, it's length 0.
            // Let's stick strictly to "if bytes not empty".
            // If user wants empty message, they pass empty string.
            // Proto3 does encode empty messages if they are present in repeated fields, or if we want to show presence? No, only fields.
            // If we encode a message field, and it has no content, it is just tag + len(0).
            // Let's allow empty messages.

            let mut buf = this.buffer.lock().unwrap_or_else(|err| err.into_inner());
            let bytes = val.as_bytes();
            encode_key(tag, WireType::LengthDelimited, &mut *buf);
            encode_varint(bytes.len() as u64, &mut *buf);
            buf.extend_from_slice(&bytes);
            Ok(this.clone())
        });

        methods.add_method("as_bytes", |lua, this, ()| {
            let buf = this.buffer.lock().unwrap_or_else(|err| err.into_inner());
            lua.create_string(&*buf)
        });

        methods.add_method("as_grpc_frame", |lua, this, compressed: Option<bool>| {
            let buf = this.buffer.lock().unwrap_or_else(|err| err.into_inner());

            let (flag, payload_bytes) = if compressed.unwrap_or(false) {
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&buf).map_err(mlua::Error::external)?;
                let payload = encoder.finish().map_err(mlua::Error::external)?;
                (1u8, payload)
            } else {
                (0u8, buf.clone())
            };

            let len = payload_bytes.len() as u32;
            let mut out = Vec::with_capacity(5 + payload_bytes.len());
            out.push(flag); // compressed
            out.extend_from_slice(&len.to_be_bytes());
            out.extend_from_slice(&payload_bytes);
            lua.create_string(&out)
        });
    }
}

// --- Protobuf Scanner for Lua ---
// A simple iterator-like object to traverse fields

#[derive(Clone)]
pub struct PbScanner {
    data: Bytes,
    pos: usize,
}

impl PbScanner {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data: Bytes::from(data),
            pos: 0,
        }
    }

    pub fn new_from_bytes(data: Bytes, pos: usize) -> Self {
        Self { data, pos }
    }
}

impl UserData for PbScanner {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("next", |lua, this, ()| {
            if this.pos >= this.data.len() {
                return Ok(None);
            }

            let mut slice = &this.data[this.pos..];
            if slice.is_empty() {
                return Ok(None);
            }

            let start_len = slice.len();
            let (tag, wire_type) = match prost::encoding::decode_key(&mut slice) {
                Ok(k) => k,
                Err(_) => return Ok(None), // End or Error
            };

            // Advance pos
            let key_len = start_len - slice.len();
            this.pos += key_len;

            // Capture value
            let val_slice = &this.data[this.pos..];

            let content_bytes = match wire_type {
                WireType::Varint => {
                    // Check length of varint
                    let mut temp = val_slice;
                    let _ =
                        prost::encoding::decode_varint(&mut temp).map_err(mlua::Error::external)?;
                    let len = val_slice.len() - temp.len();
                    let res = val_slice[..len].to_vec();
                    this.pos += len;
                    res
                }
                WireType::SixtyFourBit => {
                    if val_slice.len() < 8 {
                        return Ok(None);
                    }
                    let res = val_slice[..8].to_vec();
                    this.pos += 8;
                    res
                }
                WireType::LengthDelimited => {
                    let mut temp = val_slice;
                    let len =
                        prost::encoding::decode_varint(&mut temp).map_err(mlua::Error::external)?;
                    let header_len = val_slice.len() - temp.len();

                    if temp.len() < len as usize {
                        return Ok(None);
                    }
                    let res = temp[..len as usize].to_vec(); // Just content!
                    this.pos += header_len + len as usize;
                    res
                }
                WireType::ThirtyTwoBit => {
                    if val_slice.len() < 4 {
                        return Ok(None);
                    }
                    let res = val_slice[..4].to_vec();
                    this.pos += 4;
                    res
                }
                _ => return Ok(None),
            };

            // Return raw table instead of tuple, or use Lua::create_table to return multiple values cleanly?
            // "IntoLuaMulti" supports tuples, but only if elements support IntoLua.
            // Vec<u8> does NOT implement IntoLua directly in mlua 0.9+ sometimes without feature or wrapper?
            // Actually mlua `Vec<u8>` converts to UserData or needs bytes wrapper.
            // But we can return LuaString.

            let b = lua.create_string(&content_bytes)?;
            let t = lua.create_table()?;
            t.set(1, tag)?;
            t.set(2, wire_type as u8)?;
            t.set(3, b)?;
            Ok(Some(t))
        });

        // Helpers to parse values from bytes returned by next()

        methods.add_method("parse_bool", |_, _, bytes: mlua::String| {
            let b = bytes.as_bytes();
            let mut s = &b[..];
            let v = prost::encoding::decode_varint(&mut s).map_err(mlua::Error::external)?;
            Ok(v != 0)
        });

        methods.add_method("parse_float", |_, _, bytes: mlua::String| {
            let b = bytes.as_bytes();
            if b.len() < 4 {
                return Err(mlua::Error::RuntimeError(
                    "Not enough bytes for float".into(),
                ));
            }
            let arr: [u8; 4] = b[0..4]
                .try_into()
                .map_err(|_| mlua::Error::RuntimeError("Invalid float bytes".into()))?;
            Ok(f32::from_le_bytes(arr))
        });

        methods.add_method("parse_double", |_, _, bytes: mlua::String| {
            let b = bytes.as_bytes();
            if b.len() < 8 {
                return Err(mlua::Error::RuntimeError(
                    "Not enough bytes for double".into(),
                ));
            }
            let arr: [u8; 8] = b[0..8]
                .try_into()
                .map_err(|_| mlua::Error::RuntimeError("Invalid double bytes".into()))?;
            Ok(f64::from_le_bytes(arr))
        });

        methods.add_method("parse_uint", |_, _, bytes: mlua::String| {
            let b = bytes.as_bytes();
            let mut s = &b[..];
            let v = prost::encoding::decode_varint(&mut s).map_err(mlua::Error::external)?;
            Ok(v) // u64 directly maps to Lua (might overflow if huge? mlua handles u64 mostly or floats)
        });

        methods.add_method("parse_sint32", |_, _, bytes: mlua::String| {
            let b = bytes.as_bytes();
            let mut s = &b[..];
            let v = prost::encoding::decode_varint(&mut s).map_err(mlua::Error::external)?;
            // ZigZag decode
            let n = v as u32;
            let res = (n >> 1) as i32 ^ -((n & 1) as i32);
            Ok(res)
        });

        methods.add_method("parse_sint64", |_, _, bytes: mlua::String| {
            let b = bytes.as_bytes();
            let mut s = &b[..];
            let v = prost::encoding::decode_varint(&mut s).map_err(mlua::Error::external)?;
            // ZigZag decode
            let n = v;
            let res = (n >> 1) as i64 ^ -((n & 1) as i64);
            Ok(res)
        });

        methods.add_method("parse_int", |_, _, bytes: mlua::String| {
            let b = bytes.as_bytes();
            let mut s = &b[..]; // Explicitly convert BorrowedBytes to slice
            let v = prost::encoding::decode_varint(&mut s).map_err(mlua::Error::external)?;
            Ok(v as i64)
        });

        methods.add_method("parse_string", |_, _, bytes: mlua::String| {
            // Since we returned just content for LengthDelimited, we can just fetch string
            Ok(String::from_utf8_lossy(&bytes.as_bytes()).to_string())
        });
    }
}

// Helper to register these
pub fn register_utils(lua: &Lua) -> Result<()> {
    let pb_table = lua.create_table()?;

    pb_table.set(
        "Builder",
        lua.create_function(|_, ()| Ok(PbBuilder::new()))?,
    )?;

    pb_table.set(
        "Scanner",
        lua.create_function(
            |_, (data, start, len): (mlua::String, Option<usize>, Option<usize>)| {
                let bytes = data.as_bytes();
                let start_index = if let Some(s) = start {
                    s.saturating_sub(1) // Lua 1-based to Rust 0-based
                } else {
                    0
                };

                let slice = if let Some(l) = len {
                    if start_index + l > bytes.len() {
                        return Err(mlua::Error::RuntimeError("Index out of bounds".into()));
                    }
                    &bytes[start_index..start_index + l]
                } else {
                    if start_index > bytes.len() {
                        return Err(mlua::Error::RuntimeError("Index out of bounds".into()));
                    }
                    &bytes[start_index..]
                };

                Ok(PbScanner::new(slice.to_vec()))
            },
        )?,
    )?;

    // Helper for gRPC framing
    pb_table.set(
        "pack_grpc_frame",
        lua.create_function(|lua, data: mlua::String| {
            let bytes = data.as_bytes();
            let len = bytes.len() as u32;
            let mut buf = Vec::with_capacity(5 + bytes.len());
            buf.push(0); // non-compressed
            buf.extend_from_slice(&len.to_be_bytes());
            buf.extend_from_slice(&bytes);
            lua.create_string(&buf)
        })?,
    )?;

    pb_table.set(
        "unpack_grpc_frame",
        lua.create_function(|lua, data: mlua::String| {
            let bytes = data.as_bytes();
            if bytes.len() < 5 {
                return Ok((false, lua.create_string([])?));
            }
            let compressed = bytes[0] == 1;
            let mut len_bytes = [0u8; 4];
            len_bytes.copy_from_slice(&bytes[1..5]);
            let len = u32::from_be_bytes(len_bytes) as usize;

            if bytes.len() < 5 + len {
                // Incomplete frame or empty payload if len is 0?
                // If len is simply larger than available bytes, it's incomplete.
                // If len is 0, we return empty string.
                return Ok((compressed, lua.create_string([])?));
            }

            let payload = &bytes[5..5 + len];
            Ok((compressed, lua.create_string(payload)?))
        })?,
    )?;

    lua.globals().set("Pb", pb_table.clone())?;

    // User requested UUID to be separate from Pb
    lua.globals().set(
        "uuid_v4",
        lua.create_function(|_, ()| Ok(Uuid::new_v4().to_string()))?,
    )?;

    Ok(())
}
