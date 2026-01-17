# Copilot Instructions (Web Framework Benchmark)

This repository is a Rust workspace that benchmarks many web frameworks (multi-language) in a reproducible, Docker-driven way.

Use these instructions as the default rules when proposing changes.

## 1) Context bootstrap (do this before large changes)

When starting a task that affects behavior, architecture, or benchmarks:

- Read `README.md` for project goals and user-facing commands.
- Read `docs/METHODOLOGY.md` if the change impacts measurement, fairness, or validation.
- Read `docs/GUIDE_ADDING_BENCHMARKS.md` if the change touches `benchmarks/` or `config/`.
- Identify the relevant crate(s) from the workspace `Cargo.toml`.

If unsure where something lives, search by symbol/route name and confirm with the existing patterns instead of inventing new ones.

## 2) Repository map (high level)

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

## 3) Change discipline (how to work in this repo)

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

### Performance rule: avoid per-request env reads

- Do **not** call `std::env::var` (or parse env/config) inside per-request hot paths such as Axum handlers, middleware, or template rendering.
- Read/parse env once at startup and store it in application state, or cache it via `std::sync::OnceLock` if it truly must be global.
- Backward compatibility is **not required** unless explicitly requested.
  - If a change breaks CLI flags, config schema, or specs, call it out clearly in the summary.
- Preserve existing naming conventions:
  - Benchmark slugs are lowercase and used consistently across `benchmarks/` and `config/`.
  - Tests are selected by config and must match spec names.
- If you change an abstraction (types, traits, helper APIs, route helpers, template contracts), update all usages in the repo in the same change.
  - Prefer searching for all references/usages and adjusting them consistently.
  - Run the narrowest relevant `cargo check` / `cargo test` to validate the refactor.

## 4) Verification loop (preferred commands)

Use the narrowest check first:

- `cargo check -p wfb-server`
- `cargo check -p wfb-runner`
- `cargo test -p wfb-server` or `cargo test -p wfb-runner`

Then repo-wide style/lint when appropriate:

- `cargo fmt --all`
- `cargo clippy --all-targets -- -D warnings`

For benchmark implementations:

- For developing/checking a single benchmark implementation, always run it as a Docker container (don’t run the framework binary directly on the host).
  - Preferred dev loop (builds + starts the benchmark container):
    - `cargo run --release --bin wfb-runner -- dev <benchmark_name> --env local`
  - Contract/spec verification (runs the verification suite):
    - `cargo run --release --bin wfb-runner -- verify --benchmark <benchmark_slug> --env local`

Do not claim a benchmark “works” unless verification passes.

## 5) Rules for benchmark implementations

If editing or adding a folder under `benchmarks/`:

- The app MUST listen on port `8080`.
- A `HEALTHCHECK` MUST be present in the `Dockerfile`.
- Implement `/health` and any endpoints required by `docs/specs/*.md` for the tests listed in config.
- Respect runner-injected env vars (see `docs/GUIDE_ADDING_BENCHMARKS.md`), especially DB vars.
- For local iteration on a specific implementation, prefer `wfb-runner dev` so the benchmark is built/run through Docker with the same wiring as real runs.
- It’s also OK to locally build a benchmark image with `docker build` for quick feedback, but ensure you use the correct build context (the runner normally injects `benchmarks_data` into the context).

If you add a new benchmark:

- Create `benchmarks/<language>/<framework>/` with a `Dockerfile`.
- Register it in:
  - `config/benchmarks/<language>.yaml`
  - `config/frameworks.yaml`
  - `config/languages.yaml` (only if a new language)

## 6) UI/templates rules (wfb-server)

- UI templates and assets live under `wfb-server/`.
- If the user mentions “frontend”, “UI”, “dashboard”, “templates”, “CSS”, “assets”, or “HTMX”, immediately inspect both:
  - `wfb-server/templates/` + `wfb-server/assets/` (Askama templates, macros, static assets, Tailwind)
  - the corresponding web/API handlers in `wfb-server/src/` (routes/endpoints, response shapes, asset paths)
  Keep template contracts and API payloads in sync; don’t change one side without updating the other.
- When changing UI/layout/styling, consider both mobile and desktop variants: the page may differ across breakpoints.
  - Check responsive behavior (Tailwind breakpoints/classes, conditional template fragments) and avoid desktop-only assumptions.
- When working on frontend changes, don’t start/restart or “rebuild” `wfb-server` unless explicitly asked.
  - Assume the server is already running with watch/auto-rebuild enabled; focus on code/template changes without disrupting the running dev session.
- Follow existing template/component patterns; keep HTML structure and CSS class conventions consistent.
- On the frontend, external links are marked with an external-link indicator via CSS (currently implemented as a `::after` mask on `a[target="_blank"]`, excluding `.wfb-btn`).
  - Do not add or require a dedicated SVG icon file for this indicator unless explicitly requested.
  - When reviewing templates, verify the current rule in `wfb-server/assets/src/css/app.css` before flagging “missing external-link icon”.
- Do **not** hardcode repository host URLs (e.g. `github.com`) in templates or handlers.
  - Using `wfb-server/src/handlers/web/types.rs` (`REPOSITORY_URL`) / `chrome.repository_url` is the intended single source of truth.
  - In Askama templates, build repo/doc links via the `url_join` filter instead of embedding `https://...`.
  - Some links may still assume the default branch/path shape (e.g. `blob/main/...`, `tree/main/...`); don’t treat that as a “hardcoded GitHub URL” unless the user asks to remove the assumption.
- Prefer server-side rendering patterns already present; avoid introducing new frontend frameworks unless requested.
- For UI work, follow `docs/UI_GUIDE.md` (Askama contexts, macros, HTMX contracts, assets).

### Tailwind/Twinland notes (important)

- This repo builds CSS/JS assets via the Rust build pipeline (`wfb-server/build.rs`) which runs Tailwind + esbuild and fingerprints outputs.
  - Do NOT manually run `npx tailwindcss ...` as part of normal work.
  - Instead, validate frontend changes by running the usual Rust commands (e.g. `cargo check -p wfb-server`), which will invoke the asset builder.
  - Manual Tailwind runs are only acceptable for debugging a specific Tailwind error message, and should not be presented as the normal workflow.

- When using Tailwind utilities inside `@apply` in `wfb-server/assets/src/css/app.css`, prefer the existing component patterns (e.g. `.wfb-popover`) and token names already defined in `wfb-server/tailwind.config.js`.

- Gotcha: opacity modifiers on CSS-var colors.
  - Colors like `bg-background` are defined as CSS variables (`var(--background)`). Tailwind cannot always generate opacity variants like `bg-background/70` for these in `@apply`, and it may error with “class does not exist”.
  - If you need a “glass” look (translucency + blur), use utilities that Tailwind can generate reliably (e.g. `bg-white/70 dark:bg-black/40` + `backdrop-blur-*`), or define an explicit custom color that supports alpha if the design requires it.
  - Avoid introducing custom `color-mix(...)`/hand-rolled gradients for “glass” unless explicitly requested; use Tailwind utilities/patterns first.

- Troubleshooting (when the asset build fails):
  - The failure will usually surface during `cargo check -p wfb-server` / `cargo test -p wfb-server` because the Rust build pipeline runs Tailwind + esbuild.
  - Read the error output from `wfb-server/build.rs` first; Tailwind errors normally include the source file + line (often `wfb-server/assets/src/css/app.css` and an `@apply` line).
  - If you see “class does not exist”, check for unsupported `@apply` combinations (especially opacity modifiers like `*/70` on CSS-var theme colors).

### Header/Navigation UX (keep consistent)

- The header brand (logo image + “WFB” wordmark) should always be clickable and navigate to the index/root (`/`).
- Don’t nest links: keep the mini-nav links separate from the brand link.
- Prefer HTMX-friendly navigation (avoid forcing full reloads with `hx-boost="false"` unless there is a clear reason).

## 7) What to include in PR-quality changes

- A short summary of what changed and why.
- The exact command(s) you ran to verify the change.
- If behavior changes, note any config/schema impact.
- Keep documentation up to date: when behavior/CLI/config/specs/template contracts change, update the relevant files under `docs/` (and any other user-facing docs) in the same change.

## 8) Preparing for commit (Rust workflow)

If the user asks to “prepare for commit” / “get it ready to commit”, treat this as a defined checklist:

- **Review the diff first**: ensure the change is minimal, coherent, and does not include drive-by refactors.
  - Prefer readable, explicit code over cleverness.
  - Keep error handling intentional (avoid papering over failures; don’t weaken validation contracts).
  - Preserve existing naming/style conventions and update all call sites when changing an API.
- **Format** (required): `cargo fmt --all`
- **Lint** (required): `cargo clippy --all-targets -- -D warnings`
- **Test/Check** (required, pick the narrowest relevant):
  - `cargo check -p wfb-server` and/or `cargo test -p wfb-server`
  - `cargo check -p wfb-runner` and/or `cargo test -p wfb-runner`
  - Use `cargo test --workspace` only when the change is cross-cutting.

When handing back the result, include:

- A short “ready-to-commit” summary (what/why).
- The exact commands executed (fmt/clippy/tests).
- Any follow-ups needed (docs updates, benchmark verification, config/schema notes).
