# Methodology

This document describes how Web Framework Benchmark (WFB) executes benchmarks and how results should be interpreted.

## Goals

- Measure performance under production-like workloads (not just “Hello World”).
- Enforce correctness: a fast but wrong response is treated as a failure.
- Keep the process reproducible and transparent.

## High-level Flow

For each benchmark implementation and each test case:

1. Build the benchmark image.
2. If the test needs a database, start and wait for the database.
3. Start the application container and wait for `/health`.
4. Run a correctness verification against the spec.
5. Run the load test using `wrkr` inside Docker.

> Note: `wfb-runner dev` is a developer convenience mode that starts the containers and tails logs. It does **not** execute correctness verification or load.

## Load Profile

### Warmup

- Warmup duration: **30s**.
- Warmup connections: **8**.
- Purpose: reduce bias from cold starts / JIT warmup (Java, C#, JS, Lua, etc.).

### Measurement

- Test duration (per test case): **240s**.
- Step duration: **20s**.
- Stepping: the load is applied as discrete connection (VU) steps to show scaling behaviour.

Connection steps per test case:

- `plain_text`: `32,64,128,256,512,1024`
- `json_aggregate`: `32,64,128,256,512`
- `static_files`: `16,32,64,128,256`
- `db_complex`: `32,64,128,256,512`
- `grpc_aggregate`: `64,128,256,512,1024`

### Transport & Client

- The load generator is `wrkr` (async Rust, Tokio + Reqwest).
- HTTP/2 is enabled for gRPC (`--http2`).
- The runner sets `ulimit nofile=1000000:1000000` for the `wrkr` container.

## Correctness / Validation

WFB treats correctness as a first-class requirement.

During load, `wrkr` executes Lua scenarios that:

- send requests to the target endpoint,
- parse responses,
- validate response structure and values,
- fail the iteration on mismatches.

Examples of what is validated:

- Plaintext: status 200, correct `Content-Type`, exact body.
- JSON analytics: the expected aggregates are computed client-side and compared on every request.
- DB complex: required fields, array sizes, and sorting conditions are checked; negative cases are expected.
- Static files: correct `Content-Length`, stable bytes across requests, `HEAD`, `Range`, and conditional `304` when validators are present.
- gRPC: `grpc-status` must be `0`, request metadata must be echoed, aggregates must match expected values.

## Metrics

### Latency

- Latency is measured **per request** in the load generator as wall-clock elapsed time and recorded in **microseconds**.
- Aggregation uses an HDRHistogram.
- Reported percentiles include p50, p75, p90, p99, and max.

### Throughput

- `requests_per_sec` is computed over 1-second intervals from the observed request counter.
- `bytes_per_sec` (TPS) is computed from total bytes received.

### Errors

By default, `wrkr` counts any HTTP status outside `[200..399]` as an error.
Some scenarios intentionally disable status-code tracking to allow expected negative cases (e.g., the DB test includes 404s by design).

## Reproducibility

- The entire suite is orchestrated by `wfb-runner` and runs in Docker.
- Benchmark implementations are versioned through their Dockerfiles and the YAML config files in `config/`.

## Running a Single Benchmark Locally (Load)

The `run` command executes the configured suite; for focused work on one implementation you can:

1. Start the benchmark in dev mode:

- `cargo run --release --bin wfb-runner -- dev <benchmark_name> --env local`

2. In a second terminal, run `wrkr` directly against the app URL (local runner publishes the app on `http://localhost:54320` by default):

- `cargo run --release --bin wrkr -- -s scripts/wrkr_plaintext.lua --url http://localhost:54320 --duration 60 --connections 128`

For step load (matches the runner’s shape):

- `cargo run --release --bin wrkr -- -s scripts/wrkr_json_aggregate.lua --url http://localhost:54320 --duration 240 --step-connections 32,64,128,256,512 --step-duration 20 --output json`

If you want correctness checks only (no load), use:

- `cargo run --release --bin wfb-runner -- verify --benchmark <benchmark_name> --env local`

## Limitations / What Not To Conclude

- Results are only valid for the specific environment, versions, and workload definitions used.
- “Fastest RPS” is not the same as “best for your product”; use latency distribution and error rate as primary signals.
- Frameworks may have different “best practices” and tunables; WFB aims for a consistent baseline rather than per-framework hand-tuning.
