# rust-repositories-cleaner

A minimalist console application to protect and bulk delete your GitHub repositories.

See [specs.md](specs.md) for the full specifications.

## Requirements

- [GitHub CLI](https://cli.github.com/) (`gh`), logged in, with the `delete_repo` scope to delete repositories. See [docs/github-cli-setup.md](docs/github-cli-setup.md) for a step-by-step setup.
- To build from source only: Rust 1.93 or later (stable toolchain, see `rust-toolchain.toml`).

## Download

Download the archive of your system from the [latest release](https://github.com/aunimhsn/rust-repositories-cleaner/releases/latest), extract it, then run the executable from a terminal:

| System | Archive | Run |
|---|---|---|
| Windows | `…-x86_64-pc-windows-msvc.zip` | `.\rust-repositories-cleaner.exe`, or double-click it |
| macOS (Apple Silicon) | `…-aarch64-apple-darwin.tar.gz` | `./rust-repositories-cleaner` |
| macOS (Intel) | `…-x86_64-apple-darwin.tar.gz` | `./rust-repositories-cleaner` |
| Linux | `…-x86_64-unknown-linux-musl.tar.gz` | `./rust-repositories-cleaner` |

The executables are not signed, so the first launch shows a warning:

- **macOS**: "cannot be opened because the developer cannot be verified". Remove the quarantine flag once, then run it again:

  ```sh
  xattr -d com.apple.quarantine rust-repositories-cleaner
  ```

- **Windows**: "Windows protected your PC". Click **More info**, then **Run anyway**.

`SHA256SUMS.txt`, in the release, lists the checksums of the archives.

## Configuration

The application reads and writes its files in its own directory, created on first run inside the user configuration directory of the system:

| System | Directory |
|---|---|
| Linux | `~/.config/rust-repositories-cleaner` (or `$XDG_CONFIG_HOME/rust-repositories-cleaner`) |
| macOS | `~/Library/Application Support/rust-repositories-cleaner` |
| Windows | `%APPDATA%\rust-repositories-cleaner` |

- `config.toml`: interface language and protected repositories, created with default values on first run. See [config.example.toml](config.example.toml) for the available options.
- `logs/deletions.log`: history of the deletions.

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

## Release

1. Update `version` in `Cargo.toml` (then run `cargo build` to update `Cargo.lock`) and merge it into `main`.
2. Tag the merged commit with the same version and push the tag:

   ```sh
   git switch main && git pull
   git tag v0.1.0
   git push origin v0.1.0
   ```

The [Release workflow](.github/workflows/release.yml) builds the archives for Windows, macOS and Linux and publishes them as a GitHub release. It fails if the tag does not match the version of `Cargo.toml`.

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
