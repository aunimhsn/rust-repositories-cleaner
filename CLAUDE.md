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
- Exception: the French translations of the user interface strings (see the Language feature in `specs.md`). Only the translated strings themselves are in French; the code around them stays in English.
- Exception: the code review notes in `outputs/` are written in French.

## Git workflow

- Never commit or push directly to `main`.
- For each task, create a dedicated branch from `main`, commit and push only on that branch.
- Every change must go through a pull request targeting `main`.

## Code review

- Before opening any pull request that changes code (`src/`, tests, `Cargo.toml`), run the `code-reviewer` subagent (`.claude/agents/code-reviewer.md`) on the branch.
- It writes its recommendations, in French, to `outputs/<YYYY-MM-DD>-<branch>-code-review.md`. These notes are ignored by Git.
- Summarize the recommendations to the user and apply only the ones they approve, then open the pull request.

## Development

- Before committing, run and make sure all of these pass:
  - `cargo fmt --all --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all-features`
- The same checks run in CI (`.github/workflows/ci.yml`) on every pull request.
- `unsafe` code is forbidden and Clippy pedantic lints are enabled (see `[lints]` in `Cargo.toml`).

## Roadmap

- At the start of each session, read `roadmap.md` to know the progress; when asked to continue, resume at the first unchecked step.
- Each step is delivered in its own pull request, which checks the step's box in `roadmap.md`.
- Keep `roadmap.md` consistent with `specs.md` when the specifications change.
