# Roadmap

Implementation steps of the features described in [specs.md](specs.md). Each step is delivered in its own pull request, which checks its box.

## Technical choices

| Topic | Choice | Reason |
|---|---|---|
| Console interface | `ratatui` with `crossterm` | Reference TUI library in Rust: scrolling lists, colors, live header. |
| GitHub access | Call `gh api` from Rust | `gh` already handles the token, pagination and errors; no HTTP client or async code needed. |
| Local files location | `dirs::config_dir()` | Standard user configuration directory of each system, so that the files do not depend on the directory the application is launched from. |
| Releases | GitHub Actions workflow triggered by version tags | Builds every system on its own runner and publishes the archives with `gh release`, without extra tooling. |
| Translations | [Fluent](https://projectfluent.org/) files in `i18n/<language>/messages.ftl`, read with `i18n-embed` and its `fl!` macro | Texts are separated from the code and plurals follow each language's rules. `fl!` checks at compile time that each message and its arguments exist in English, and a test checks that every language has the same messages. |

## Steps

- [x] **1. Config and local files**: read and write `config.toml`, add `config.example.toml`, ignore `/config.toml` and `/logs` in `.gitignore`.
- [x] **2. Translations**: English and French Fluent files, loader and key parity test.
- [x] **3. GitHub access**: check `gh` and the `delete_repo` scope, fetch the repository list.
- [x] **4. Main menu and scope menu**, with the language menu.
- [x] **5. Protection list**: scrollable table of the repositories, protection toggle, saved to `config.toml`.
- [x] **6. Deletion list**: confirmation, deletion and log. Progress is shown as text ("Deleting repositories… 4/10") until step 7.
  - Real deletions are tested by the owner, in their own terminal, on a throwaway repository: only their `gh` login there has the `delete_repo` scope (`gh auth refresh -h github.com -s delete_repo`). Claude's `gh` login stays without it.
- [x] **7. Deletion progress bar**: replace the progress text with a `ratatui` `LineGauge` showing the percentage and the count (see `specs.md`).
- [x] **8. "Since last deletion" filter**: last deletion date read from the log.
- [x] **9. Application directory**: `config.toml` and `logs/` in the user configuration directory of the system instead of the current directory.
- [x] **10. Releases**: GitHub Actions workflow publishing the Windows, macOS and Linux archives when a version tag is pushed, and download instructions in `README.md`.
