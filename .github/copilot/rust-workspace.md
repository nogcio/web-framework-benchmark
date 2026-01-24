# Rust Workspace (runner/server/storage)

Applies when editing Rust code under: `wfb-runner/`, `wfb-server/`, `wfb-storage/`.

## Performance rule: avoid per-request env reads

- Do **not** call `std::env::var` (or parse env/config) inside per-request hot paths such as Axum handlers, middleware, or template rendering.
- Read/parse env once at startup and store it in application state, or cache it via `std::sync::OnceLock` if it truly must be global.

## Reliability rule: no `unwrap`/`expect` (AI)

- Do not introduce `unwrap()` or `expect()` in non-test code.
	- Use `?` / structured errors for fallible operations.
	- Use `match` / `if let` for `Option`.
	- For poisoned locks, recover explicitly (e.g. `PoisonError::into_inner()`) and log.

## API/abstraction changes

- If you change an abstraction (types, traits, helper APIs, route helpers, template contracts), update all usages in the repo in the same change.
- Prefer searching for all references/usages and adjusting them consistently.
- Run the narrowest relevant `cargo check` / `cargo test` to validate the refactor.
