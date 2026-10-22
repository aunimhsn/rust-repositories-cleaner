# rust-repositories-cleaner

A minimalist console application to protect and bulk delete your GitHub repositories.

See [specs.md](specs.md) for the full specifications.

## Requirements

- Rust 1.93 or later (stable toolchain, see `rust-toolchain.toml`)
- [GitHub CLI](https://cli.github.com/) (`gh`), logged in, with the `delete_repo` scope to delete repositories. See [docs/github-cli-setup.md](docs/github-cli-setup.md) for a step-by-step setup.

## Configuration

The application reads and writes its files in the current directory, so run it from the project directory:

- `config.toml`: interface language and protected repositories, created with default values on first run. See [config.example.toml](config.example.toml) for the available options.
- `logs/deletions.log`: history of the deletions.

Both are ignored by Git.

## Keyboard shortcuts

| Keys | Action |
|---|---|
| `↑` / `↓` | Move |
| `Shift + ↑` / `Shift + ↓` (or `PgUp` / `PgDn`) | Jump 5 repositories |
| `Ctrl + ↑` / `Ctrl + ↓` (or `Home` / `End`) | First or last repository |
| `Space` | Protect, or mark for deletion |
| `Enter` | Validate |
| `Esc` | Go back or cancel |
| `Ctrl + C` | Quit from any screen |

## Build

```sh
cargo build --release
```

## Development

```sh
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
