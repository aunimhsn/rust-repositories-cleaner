# rust-repositories-cleaner

A minimalist console application to protect and bulk delete your GitHub repositories.

See [specs.md](specs.md) for the full specifications.

## Requirements

- Rust 1.93 or later (stable toolchain, see `rust-toolchain.toml`)
- [GitHub CLI](https://cli.github.com/) (`gh`), logged in with the `delete_repo` scope:

  ```sh
  gh auth login
  gh auth refresh -s delete_repo
  ```

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
