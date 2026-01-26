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
5. Run the load test using [nogcio/wrkr](https://github.com/nogcio/wrkr) inside Docker.

> Note: `wfb-runner dev` is a developer convenience mode that starts the containers and tails logs. It does **not** execute correctness verification or load.

## Load Profile

### Warmup

- Warmup duration: **30s**.
- Warmup connections: **8**.
- Purpose: reduce bias from cold starts / JIT warmup (Java, C#, JS, Lua, etc.).

### Measurement

- Test duration (per test case): **240s**.
- Load shape: `ramping-vus` (ramp up → hold → ramp down) to a per-test max VU target.

Max VUs per test case:

- `plain_text`: `1024`
- `json_aggregate`: `512`
- `db_complex`: `512`
- `grpc_aggregate`: `1024`
- `static_files`: `128`

### Transport & Client

- The load generator is [nogcio/wrkr](https://github.com/nogcio/wrkr) running in Docker.
- The runner mounts `./scripts` into the container and executes `scripts/wfb_*.lua` scenarios.
- The runner sets `ulimit nofile=1000000:1000000` for the [nogcio/wrkr](https://github.com/nogcio/wrkr) container.

## Correctness / Validation

WFB treats correctness as a first-class requirement.

During load, [nogcio/wrkr](https://github.com/nogcio/wrkr) executes Lua scenarios that:

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

- Latency is measured **per request** in the load generator as wall-clock elapsed time.
- Storage is recorded in **microseconds** (the runner converts the load generator JSON output, which is in milliseconds).
- Aggregation uses an HDRHistogram.
- Reported percentiles include p50, p75, p90, p99, and max.

### Throughput

- `requests_per_sec` is computed over 1-second intervals from the observed request counter.
- `bytes_per_sec` (TPS) is computed from total bytes received.

### Errors

Errors are reported as failed checks from the load generator.
Some scenarios intentionally allow expected negative cases (e.g., `db_complex` includes 404s by design and does not treat them as failures).

## Reproducibility

- The entire suite is orchestrated by `wfb-runner` and runs in Docker.
- Benchmark implementations are versioned through their Dockerfiles and the YAML config files in `config/`.

## Running a Single Benchmark Locally (Load)

The `run` command executes the configured suite; for focused work on one implementation you can:

1. Start the benchmark in dev mode:

- `cargo run --release --bin wfb-runner -- dev <benchmark_name> --env local`

2. In a second terminal, run [nogcio/wrkr](https://github.com/nogcio/wrkr) directly against the app URL.

On macOS, the easiest way is to run the load generator in Docker and target `host.docker.internal`.

Docker image: [nogcio/wrkr](https://github.com/nogcio/wrkr) (`nogcio/wrkr:latest`).

```bash
docker run --rm \
	-v "$PWD/scripts:/scripts:ro" \
	-e BASE_URL="http://host.docker.internal:54320" \
	-e WFB_DURATION="60s" \
	-e WFB_MAX_VUS="128" \
	nogcio/wrkr:latest run /scripts/wfb_plaintext.lua --output json
```

If you want correctness checks only (no load), use:

- `cargo run --release --bin wfb-runner -- verify --benchmark <benchmark_name> --env local`

## Limitations / What Not To Conclude

- Results are only valid for the specific environment, versions, and workload definitions used.
- “Fastest RPS” is not the same as “best for your product”; use latency distribution and error rate as primary signals.
- Frameworks may have different “best practices” and tunables; WFB aims for a consistent baseline rather than per-framework hand-tuning.
