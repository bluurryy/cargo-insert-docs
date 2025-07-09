# cargo-insert-docs

[![Crates.io](https://img.shields.io/crates/v/cargo-insert-docs.svg)](https://crates.io/crates/cargo-insert-docs)
[![License](https://img.shields.io/crates/l/cargo-insert-docs)](#license)
[![Build Status](https://github.com/bluurryy/cargo-insert-docs/workflows/Rust/badge.svg)](https://github.com/bluurryy/cargo-insert-docs/actions/workflows/rust.yml)

Insert feature documentation into the crate documentation and
crate documentation into the readme.

## Installation

```sh
cargo install cargo-insert-docs
```

## Compatibility

This tool does two separate jobs, either can be turned off with `--no-feature-docs` and `--no-crate-docs`.

The "insert crate documentation into readme" part of the tool requires a nightly toolchain and uses the unstable rustdoc json output. A new nightly version may no longer be compatible with the tool and the tool will need to be updated. Alternatively you can choose a specific nightly version using the `--toolchain` argument.

|crate version|rust version|
|---|---|
|0.1.x|nightly-2025-06-22 - ?|

## Usage

Use `##` in your `Cargo.toml` to add documentation to individual features and `#!` to add documentation between features:
```toml
#! ### Optional features

## Adds serde implementations for crate types.
serde = []
```

Then add a feature documentation section to your `lib.rs` file:
```rs
//! # Feature Flags
//! <!-- feature documentation start -->
//! <!-- feature documentation end -->
```

And add a crate documentation section to your `README.md`:
```md
<!-- crate documentation start -->
<!-- crate documentation end -->
```

Now run `cargo-insert-docs`:
```sh
cargo insert-docs
```

And documentation will be inserted. Have a look at the [example-crate](tests/example-crate) to see what the output looks like.

To update the sections just run the command again.

You don't have to add both sections for the tool to work. If it doesn't find a section it will just carry on with a warning. You can turn that warning into an error with the `--strict` flag.

You can find details about all the available arguments in [docs/cli.md](docs/cli.md).

## Similar projects
- **`document-features`**: <https://crates.io/crates/document-features>
- **`cargo-readme`**: <https://crates.io/crates/cargo-readme>
- **`cargo-rdme`**: <https://crates.io/crates/cargo-rdme>

## License

Licensed under either of:

 * MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)
 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)

at your option.

---

This project includes code adapted from the Rust standard library 
(<https://github.com/rust-lang/rust>),  
Copyright © The Rust Project Developers.
Such code is also licensed under MIT OR Apache-2.0.

### Your contributions

Unless you explicitly state otherwise,
any contribution intentionally submitted for inclusion in the work by you,
as defined in the Apache-2.0 license, 
shall be dual licensed as above,
without any additional terms or conditions.