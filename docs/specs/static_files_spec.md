# Static Files Test Case (Realistic HTTP Semantics)

This test case verifies that a framework serves static binary files correctly **and** behaves like a production-ready static file handler with respect to caching and partial content.

**Why this matters:** real-world static serving depends on cache validators and range requests, not just returning bytes.

It is intentionally stricter than a “200 + correct length” check: returning a buffer from memory can pass that, while still being a poor/incorrect static implementation in real deployments.

## Requirements

### Files
The service must expose the following files under `/files`:

- `/files/15kb.bin` (exactly 15 * 1024 bytes)
- `/files/1mb.bin` (exactly 1024 * 1024 bytes)

> Note: A larger file (e.g. 10MB) may be added in a future revision of this test.

The file contents must be stable across requests within a run (serving a changing blob is not considered a valid static file).

### 1. Full GET
For each file above, the service must support a normal GET:

- **Method**: `GET`
- **URL**: `/files/<name>.bin`
- **Request Headers**: `Accept-Encoding: identity`

Response requirements:

- **Status Code**: `200 OK`
- **Headers**:
  - `Content-Length` must be present and equal to the exact file size
  - `Content-Type` must contain `application/octet-stream`
- **Body**: payload must be exactly the file bytes; length must match `Content-Length`

### 2. HEAD
The service must support HEAD for at least `/files/1mb.bin`:

- **Method**: `HEAD`
- **URL**: `/files/1mb.bin`
- **Request Headers**: `Accept-Encoding: identity`

Response requirements:

- **Status Code**: `200 OK`
- **Headers**:
  - `Content-Length` must be present and equal to `1024 * 1024`
- **Body**: empty (no payload)

### 3. Range Requests (Partial Content)
The service must support byte range reads for `/files/1mb.bin`:

- **Method**: `GET`
- **URL**: `/files/1mb.bin`
- **Request Headers**:
  - `Accept-Encoding: identity`
  - `Range: bytes=0-1023`

Response requirements:

- **Status Code**: `206 Partial Content`
- **Headers**:
  - `Content-Range` must be exactly `bytes 0-1023/1048576`
- **Body**:
  - the server must return the correct payload for the range
  - validation is performed via headers (see notes below)

### 4. Conditional GET (Cache Validation)
Real static delivery relies heavily on cache validators.

For `/files/1mb.bin`:

- If the server returns an `ETag` header on the first full `GET`, then it **must** honor a subsequent `GET` with `If-None-Match: <etag>` by returning:
  - **Status Code**: `304 Not Modified`

- Otherwise, if the server returns `Last-Modified`, then it **must** honor a subsequent `GET` with `If-Modified-Since: <last-modified>` by returning:
  - **Status Code**: `304 Not Modified`

If the server provides neither `ETag` nor `Last-Modified`, the conditional section is skipped (acceptable but less cache-friendly).

## Verification Logic
The reference test runner performs these checks:

1. For each of the files:
  - `GET` once, validate status, `Content-Length`, and `Content-Type`.
  - `GET` again and assert caching validator stability if present (`ETag` or `Last-Modified`).
2. For `/files/1mb.bin`:
   - `HEAD` and verify correct `Content-Length`.
  - `Range` request and verify `206`, `Content-Range`, and `Content-Length`.
   - If the server exposes validators (`ETag` or `Last-Modified`), verify it returns `304 Not Modified` to the corresponding conditional request.

## Notes

### Body validation

The load generator used by WFB (wrkr) exposes response bodies to Lua as UTF-8 decoded strings. For `application/octet-stream` responses, the Lua `res.body` may be empty even when the full payload is transferred.

Because of that, the WFB `static_files` verification validates static file correctness via headers and caching semantics (e.g. `Content-Length`, `Content-Range`, `ETag`/`Last-Modified`, and status codes) rather than byte-for-byte comparison of the response body.

