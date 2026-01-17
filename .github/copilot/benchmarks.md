# Benchmarks (contract-driven)

Applies when editing benchmark implementations (non-Rust apps, Docker contexts, framework code) under `benchmarks/`.

## Rules for benchmark implementations

- The app MUST listen on port `8080`.
- A `HEALTHCHECK` MUST be present in the `Dockerfile`.
- Implement `/health` and any endpoints required by `docs/specs/*.md` for the tests listed in config.

## Performance & contract

- Benchmark implementations are contract-driven: assume requests match the spec (method/path/headers/body) and optimize for throughput/latency.
- Avoid “extra safety” in the hot path (redundant validation, sanitization, schema checking, defensive copies, verbose error handling).
- Prefer fail-fast at startup (config/env/DB wiring) over per-request guards.
- Keep per-request work minimal: no per-request env reads, no extra allocations/logging, no expensive parsing beyond what the spec requires.

## Environment & local iteration

- Respect runner-injected env vars (see `docs/GUIDE_ADDING_BENCHMARKS.md`), especially DB vars.
- For local iteration on a specific implementation, prefer `wfb-runner dev` so the benchmark is built/run through Docker with the same wiring as real runs.
- It’s also OK to locally build a benchmark image with `docker build` for quick feedback, but ensure you use the correct build context (the runner normally injects `benchmarks_data` into the context).

## Adding a new benchmark

- Create `benchmarks/<language>/<framework>/` with a `Dockerfile`.
- Register it in:
  - `config/benchmarks/<language>.yaml`
  - `config/frameworks.yaml`
  - `config/languages.yaml` (only if a new language)
