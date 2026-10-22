# CLAUDE.md

## Communication

- Always communicate with the user in French through the Claude Code harness (chat responses, explanations, questions).

## Naming conventions

- Follow Rust conventions: Rust source files and module directories use snake_case (e.g. `src/repository_scanner.rs`, `src/repository_cleaner/`).
- All other files and directories use kebab-case (e.g. `docs/getting-started.md`, `scripts/`).
- Standard names keep their conventional form (e.g. `Cargo.toml`, `README.md`, `CLAUDE.md`).
- File and directory names must be in English.

## Language

- Everything in the project (code, identifiers, comments, documentation, commit messages) must be written in English.

## Git workflow

- Never commit or push directly to `main`.
- For each task, create a dedicated branch from `main`, commit and push only on that branch.
- Every change must go through a pull request targeting `main`.

## Development

- Before committing, run and make sure all of these pass:
  - `cargo fmt --all --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all-features`
- The same checks run in CI (`.github/workflows/ci.yml`) on every pull request.
- `unsafe` code is forbidden and Clippy pedantic lints are enabled (see `[lints]` in `Cargo.toml`).
