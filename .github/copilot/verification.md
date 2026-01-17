# Verification loops

Pick commands based on what you changed.

## Rust crates

Use the narrowest check first:

- `cargo check -p wfb-server`
- `cargo check -p wfb-runner`
- `cargo test -p wfb-server` or `cargo test -p wfb-runner`

Then repo-wide style/lint when appropriate:

- `cargo fmt --all`
- `cargo clippy --all-targets -- -D warnings`

## Benchmarks

Important: when you are **only** changing a benchmark implementation under `benchmarks/`, you generally **should not** run `cargo check` / `cargo test` as “verification”. Those commands validate the Rust workspace.

For developing/checking a single benchmark implementation, always run it as a Docker container:

- Preferred dev loop:
  - `cargo run --release --bin wfb-runner -- dev <benchmark_name> --env local`
- Contract/spec verification:
  - `cargo run --release --bin wfb-runner -- verify --benchmark <benchmark_slug> --env local`

Do not claim a benchmark “works” unless verification passes.
