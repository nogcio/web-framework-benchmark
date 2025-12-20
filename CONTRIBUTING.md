# Contributing

Thank you for wanting to contribute! Please follow these guidelines to help us review and merge changes quickly.

1. Fork & branches
- Fork the repository.
- Create a branch with a descriptive name, for example: `feat/...`, `fix/...`, `chore/...`.

2. Code style & checks
- For Rust: run `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings`.
- For the frontend: in `web-app` run `npm install` and `npm run lint`.
- Add tests for changed logic and run `cargo test` / `npm test`.

3. Commits & pull requests
- Use clear, imperative commit messages, e.g. "Add X", "Fix Y".
- Open a Pull Request against `main` with a description of what changed, why, and how to test.
- Reference related issues if applicable.

4. PR checklist (for authors)
- Code is formatted and linters pass.
- Tests added/updated when applicable.
- Documentation or `README.md` updated if necessary.

5. Communication & security
- Do not commit secrets, tokens, or private keys.
- If you discover a security issue, report it privately following `SECURITY.md`.

If you need help with a PR, leave a comment on the PR or contact the maintainers.
