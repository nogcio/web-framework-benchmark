# Architecture

This project is a Rust workspace plus a large set of Dockerized benchmark implementations.

## Workspace crates

- `wfb-runner`: CLI orchestrator.
  - Builds and runs benchmark containers.
  - Starts/stops supporting DB containers.
  - Runs verification against the specs in `docs/specs/`.
  - Runs load tests via [nogcio/wrkr](https://github.com/nogcio/wrkr) (Docker) and aggregates results.

- `wfb-server`: Dashboard/API server.
  - Serves the UI (templates/assets) and exposes benchmark data.

- `wfb-storage`: Shared library.
  - Loads YAML config from `config/`.
  - Defines shared types/models used by runner and server.

## Load generator

- [nogcio/wrkr](https://github.com/nogcio/wrkr) (Docker image): executes the load tests.
- `scripts/`: Lua scenarios used by the runner (mounted into the [nogcio/wrkr](https://github.com/nogcio/wrkr) container).

## Non-Rust layout

- `benchmarks/`: All benchmark implementations.
  - Structure: `benchmarks/<language_slug>/<framework_slug>/`.
  - Each benchmark must provide a `Dockerfile` and implement endpoints per `docs/specs/*`.

- `config/`: Registry and configuration.
  - `config/frameworks.yaml`: Framework metadata.
  - `config/languages.yaml`: Language metadata.
  - `config/benchmarks/*.yaml`: Benchmark definitions (tests, versions, paths, tags).
  - `config/environments/`: Environment definitions (e.g., local).

- `docs/specs/`: Canonical endpoint/protocol specs.

- `benchmarks_db/`: DB images and init scripts used by DB-related benchmarks.

- `benchmarks_data/`: Static files used by certain benchmarks (e.g., static-file suites).

## Common workflows

Build/check:

- `cargo check -p wfb-runner`
- `cargo check -p wfb-server`

Format/lint:

- `cargo fmt --all`
- `cargo clippy --all-targets -- -D warnings`

Run server:

- `cargo run --release --bin wfb-server`

Run a single benchmark (development):

- `cargo run --release --bin wfb-runner -- dev <benchmark_slug> --env local`

Verify a benchmark against specs:

- `cargo run --release --bin wfb-runner -- verify --benchmark <benchmark_slug> --env local`
