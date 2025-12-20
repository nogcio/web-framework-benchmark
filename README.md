# Web Framework Benchmark

A collection of benchmarks and tools for comparing web frameworks and simple HTTP services.

## Quick Start

1) Build and run the Rust CLI benchmarks:

```bash
# from the repository root
cargo build --release
# view CLI options
cargo run --release -- --help
```

2) Frontend web application (in `web-app`):

```bash
cd web-app
npm install
npm run dev    # development
npm run build  # production build
```

3) Benchmark database (Docker image in `benchmarks_db`):

```bash
cd benchmarks_db
docker build -t wfb-db .
# then run the container with appropriate ports and volumes
```

## Repository layout

- `src/` — Rust CLI and benchmark code
- `benchmarks/` — example services implemented in different languages
- `web-app/` — frontend for demonstrating results
- `benchmarks_db/` — DB image and initialization scripts

## Formatting and linting

```bash
# Rust
cargo fmt --all
cargo clippy --all-targets -- -D warnings

# Frontend
cd web-app
npm run lint
```

## Contribution

Please read `CONTRIBUTING.md` before opening a PR.

## License

Add a `LICENSE` file with the chosen license (for example, MIT or Apache-2.0).

## Contacts

Maintainers: list authors in `Cargo.toml` and `package.json`.

---

If you want, I can also add a `CONTRIBUTING.md` template, set up CI (GitHub Actions) for build/tests/linting, or populate the `LICENSE` file — tell me which task to do next.
