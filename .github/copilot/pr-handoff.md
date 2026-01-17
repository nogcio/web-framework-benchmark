# PR-quality handoff

Include:

- A short summary of what changed and why.
- The exact command(s) you ran to verify the change.
- If behavior changes, note any config/schema impact.
- Keep documentation up to date when behavior/CLI/config/specs/template contracts change.

## Preparing for commit (Rust workflow)

If the user asks to “prepare for commit” / “get it ready to commit”, use this checklist:

- Review the diff: minimal, coherent, no drive-by refactors.
- Format (required): `cargo fmt --all`
- Lint (required): `cargo clippy --all-targets -- -D warnings`
- Test/Check (required, pick narrowest relevant):
  - `cargo check -p wfb-server` and/or `cargo test -p wfb-server`
  - `cargo check -p wfb-runner` and/or `cargo test -p wfb-runner`
  - Use `cargo test --workspace` only when change is cross-cutting.

When handing back the result, include:

- A short “ready-to-commit” summary (what/why).
- The exact commands executed (fmt/clippy/tests).
- Any follow-ups needed (docs updates, benchmark verification, config/schema notes).
