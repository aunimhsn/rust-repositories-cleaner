# rust-repositories-cleaner

A command-line tool to clean Rust repositories.

## Requirements

- Rust 1.93 or later (stable toolchain, see `rust-toolchain.toml`)

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
