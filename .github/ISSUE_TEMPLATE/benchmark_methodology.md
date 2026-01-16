---
name: Benchmark methodology / fairness
about: Questions or concerns about benchmark correctness, fairness, configs, or interpretation
---

## What are you questioning?

- [ ] Spec / correctness requirements
- [ ] Load profile (duration, warmup, connections)
- [ ] Implementation config (framework/app settings)
- [ ] Results interpretation (RPS/latency/errors)
- [ ] Something else

## Summary

Describe the concern in 2–5 sentences.

## Evidence / reproduction

Please include at least one of the following:

- A link to the relevant spec in `docs/specs/`.
- The benchmark name + test case name.
- A minimal reproduction command (local):
  - `cargo run --release --bin wfb-runner -- verify --benchmark <name> --env local`
  - `cargo run --release --bin wfb-runner -- dev <name> --env local`
- Logs (runner + container logs) if applicable.

## Proposed fix (preferred)

What change would address it?

- Spec change
- Lua scenario change (`scripts/`)
- Runner change (`wfb-runner/`)
- Config change (`config/`)
- Benchmark implementation change (`benchmarks/`)

## Environment (if results-related)

- CPU model / cores:
- RAM:
- OS + kernel:
- Docker version:
- Notes about network topology (same host, separate host, etc.):
