# Copilot Instructions (Web Framework Benchmark)

This repository is a Rust workspace that benchmarks many web frameworks (multi-language) in a reproducible, Docker-driven way.

Use these instructions as the default rules when proposing changes.

## Where you are in the monorepo

- Editing `benchmarks/**` → see [copilot/benchmarks.md](./copilot/benchmarks.md)
- Editing Rust crates (`wfb-*`, `wrkr*`) → see [copilot/rust-workspace.md](./copilot/rust-workspace.md)
- Editing UI/templates/assets (`wfb-server/templates/**`, `wfb-server/assets/**`) → see [copilot/ui-templates.md](./copilot/ui-templates.md)
- Editing `config/**` or `docs/**` → see [copilot/config-docs.md](./copilot/config-docs.md)
- How to verify changes → see [copilot/verification.md](./copilot/verification.md)
- PR handoff / ready-to-commit checklist → see [copilot/pr-handoff.md](./copilot/pr-handoff.md)

When in doubt: search by route/symbol and follow existing patterns.

## Quick decision tree

- Changing only `benchmarks/**` (framework app / Docker context) → read [copilot/benchmarks.md](./copilot/benchmarks.md), verify via Docker loop in [copilot/verification.md](./copilot/verification.md)
- Changing Rust crates (`wfb-*`, `wrkr*`) → read [copilot/rust-workspace.md](./copilot/rust-workspace.md), verify via [copilot/verification.md](./copilot/verification.md)
- Changing UI/templates/assets → read [copilot/ui-templates.md](./copilot/ui-templates.md), verify via `cargo check -p wfb-server` (see [copilot/verification.md](./copilot/verification.md))
- Changing config/specs/docs → read [copilot/config-docs.md](./copilot/config-docs.md), call out breaking changes; verify the affected benchmark/spec via [copilot/verification.md](./copilot/verification.md) when relevant

## Global rules (apply everywhere)

When starting a task that affects behavior, architecture, or benchmarks:

- Read `README.md` for project goals and user-facing commands.
- Read `docs/METHODOLOGY.md` if the change impacts measurement, fairness, or validation.
- Read `docs/GUIDE_ADDING_BENCHMARKS.md` if the change touches `benchmarks/` or `config/`.
- Identify the relevant crate(s) from the workspace `Cargo.toml`.

If unsure where something lives, search by symbol/route name and confirm with the existing patterns instead of inventing new ones.

### Repository map (high level)

- `wfb-runner/`: CLI orchestrator. Builds/runs benchmark containers, manages DB services, runs verification and load tests.
- `wfb-server/`: Web server + dashboard/API. Also contains templates/assets for the UI.
- `wfb-storage/`: Shared library for configuration, storage, and data models used by runner/server.
- `wrkr/`: Load generator binary (Rust) used to drive traffic and validate responses.
- `wrkr-core/`: Core load-testing logic (library + benches).
- `wrkr-api/`: API/structs used by the load generator and runner.

Non-Rust runtime assets:

- `benchmarks/<language>/<framework>/`: One benchmark implementation per folder (Docker build context).
- `config/`: YAML registry of languages/frameworks/benchmarks and environment configs.
- `docs/specs/`: Endpoint and protocol specs used by verification.
- `benchmarks_db/`: DB container images/init scripts.
- `benchmarks_data/`: Static data files used by some benchmarks.
- `data/`: On-disk benchmark run outputs (see “Benchmark results” below).

### Benchmark results (on-disk data layout)

Benchmark outputs are stored on disk under `data/<run_id>/` (where `run_id` is the argument passed to `wfb-runner run <run_id> ...`). Both `wfb-runner` and `wfb-server` load results from this folder via `wfb-storage`.

Directory structure:

- `data/<run_id>/manifest.yaml`: run-level manifest (e.g. `created_at`).
- `data/<run_id>/<environment>/<language>/<benchmark>/`: per-benchmark results for that environment.
  - `<environment>` is the environment config name (e.g. `local`, `dell_r640`).
  - `<language>` is the configured language name (case-sensitive; e.g. `Rust`, `Go`, `JavaScript`).
  - `<benchmark>` is the configured benchmark/framework name (e.g. `axum`, `fastify`, `hyperf-swoole-pg`).

Per-benchmark folder contents:

- `manifest.yaml`: benchmark manifest/metadata.
- `<test>.yaml`: summary for a single test case (e.g. `plaintext.yaml`, `json_aggregate.yaml`, `db_complex.yaml`, `grpc_aggregate.yaml`).
- `<test>_raw.jsonl`: raw per-iteration/per-sample records for that test case (JSON Lines).

### Change discipline (how to work in this repo)

- Prefer small, focused diffs; avoid unrelated refactors.
- Prefer simple, readable, DRY code over “quick hacks”.
  - Don’t duplicate logic; extract helpers/modules when there are 2+ call sites or the logic is non-trivial.
  - Don’t cram everything into one file; keep responsibilities separated and follow the existing folder/module structure.
  - Keep names explicit; optimize for clarity first, performance only when it matters.

### Performance is a priority

- Treat performance as a first-class requirement: avoid introducing latency/allocations in hot paths (request handlers, middleware, rendering, critical loops).
- If a change adds work per request (parsing, env/config reads, extra I/O, heavy allocations), refactor to do it once at startup or cache it (e.g. in app state or via `std::sync::OnceLock`).
- Respect user edits and reversions:
  - If the user manually changes files or undoes/reverts changes during the session, treat the current workspace state as the source of truth.
  - Do NOT re-apply the same changes again just because they were previously suggested or attempted.
  - If a previously-applied change was reverted and still seems necessary, ask for confirmation before re-introducing it.
- No shortcuts:
  - Don’t ship TODO-only changes, stubs, or placeholder implementations.
  - Don’t “make it pass” by weakening correctness/validation or bypassing existing contracts.
  - Prefer completing the change end-to-end in one coherent PR (code + callers/usages + minimal verification).

## Rust error-handling rule (AI)

- Do not use `unwrap()` or `expect()` in production code (including `wfb-server`).
  - Prefer `?`, `match`, `if let`, and explicit fallbacks.
  - If a lock can be poisoned, recover deliberately (e.g. via `PoisonError::into_inner()`) and log.
  - `unwrap/expect` are acceptable only in tests/benches/examples when the panic is intentionally part of the test.

## Area-specific essentials (quick summary)

### Benchmarks (`benchmarks/**`)

- Treat benchmark implementations as contract-driven; don’t add “extra safety” in the hot path.
- Must listen on `8080`, include `HEALTHCHECK`, and implement endpoints required by `docs/specs/*.md`.

### Rust crates (`wfb-*`, `wrkr*`)

- Never read env/config per-request; parse once at startup or cache via `OnceLock`.
- If you change an API/contract, update all call sites in the same change.

### UI/templates/assets (`wfb-server/templates/**`, `wfb-server/assets/**`)

- Keep Askama templates and handler payloads in sync.
- Don’t hardcode repository host URLs; use the repo URL single source of truth and `url_join`.
- Tailwind/esbuild runs via the Rust build pipeline; don’t use `npx tailwindcss` as normal workflow.

### Config/docs (`config/**`, `docs/**`)

- Preserve naming conventions (benchmark slugs/tests).
- Backward compatibility is not required unless requested; call out breaking changes.

## Verification and PR handoff

See [copilot/verification.md](./copilot/verification.md) and [copilot/pr-handoff.md](./copilot/pr-handoff.md).
