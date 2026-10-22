# Roadmap

Implementation steps of the features described in [specs.md](specs.md). Each step is delivered in its own pull request, which checks its box.

## Technical choices

| Topic | Choice | Reason |
|---|---|---|
| Console interface | `ratatui` with `crossterm` | Reference TUI library in Rust: scrolling lists, colors, live header. |
| GitHub access | Call `gh api` from Rust | `gh` already handles the token, pagination and errors; no HTTP client or async code needed. |
| Translations | [Fluent](https://projectfluent.org/) files in `i18n/<language>/messages.ftl`, read with `i18n-embed` and its `fl!` macro | Texts are separated from the code and plurals follow each language's rules. `fl!` checks at compile time that each message and its arguments exist in English, and a test checks that every language has the same messages. |

## Steps

- [x] **1. Config and local files**: read and write `config.toml`, add `config.example.toml`, ignore `/config.toml` and `/logs` in `.gitignore`.
- [x] **2. Translations**: English and French Fluent files, loader and key parity test.
- [x] **3. GitHub access**: check `gh` and the `delete_repo` scope, fetch the repository list.
- [ ] **4. Main menu and scope menu.**
- [ ] **5. Protection list.**
- [ ] **6. Deletion list**: confirmation, deletion and log.
  - Before testing for real, add the `delete_repo` scope to the owner's `gh` token (approved by the owner, only when needed): run `gh auth refresh -h github.com -s delete_repo` in the background and give the owner the one-time code to enter at https://github.com/login/device.
- [ ] **7. "Since last deletion" filter**: last deletion date read from the log.
