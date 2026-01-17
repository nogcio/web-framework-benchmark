# Config & Docs

Applies when editing `config/` and `docs/`.

## Naming & schema

- Preserve naming conventions:
  - Benchmark slugs are lowercase and used consistently across `benchmarks/` and `config/`.
  - Tests are selected by config and must match spec names.

## Compatibility

- Backward compatibility is **not required** unless explicitly requested.
- If a change breaks CLI flags, config schema, or specs, call it out clearly in the summary.
