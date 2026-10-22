# Roadmap

Implementation steps of the features described in [specs.md](specs.md). Each step is delivered in its own pull request, which checks its box.

## Technical choices

| Topic | Choice | Reason |
|---|---|---|
| Console interface | `ratatui` with `crossterm` | Reference TUI library in Rust: scrolling lists, colors, live header. |
| GitHub access | Call `gh api` from Rust | `gh` already handles the token, pagination and errors; no HTTP client or async code needed. |
| Translations | `Language` enum with one function per message, no library | The compiler guarantees every message exists in both languages; plurals are handled in code. |

## Steps

- [ ] **1. Config and local files**: read and write `config.toml`, add `config.example.toml`, ignore `/config.toml` and `/logs` in `.gitignore`.
- [ ] **2. Translations**: `Language` type with English and French messages.
- [ ] **3. GitHub access**: check `gh` and the `delete_repo` scope, fetch the repository list.
- [ ] **4. Main menu and scope menu.**
- [ ] **5. Protection list.**
- [ ] **6. Deletion list**: confirmation, deletion and log.
- [ ] **7. "Since last deletion" filter**: last deletion date read from the log.
