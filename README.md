# cargo-insert-docs

[![Crates.io](https://img.shields.io/crates/v/cargo-insert-docs.svg)](https://crates.io/crates/cargo-insert-docs)
[![License](https://img.shields.io/crates/l/cargo-insert-docs)](#license)
[![Build Status](https://github.com/bluurryy/cargo-insert-docs/workflows/Release/badge.svg)](https://github.com/bluurryy/cargo-insert-docs/actions/workflows/release.yml)

## Overview

`cargo-insert-docs` does two independent tasks
1. Inserts feature documentation from `Cargo.toml` into your crate docs.
2. Inserts crate documentation from `lib.rs` into your `README.md`.

You can use either task on its own by disabling the other with `--no-feature-docs` or `--no-crate-docs`.

## Installation

```sh
cargo install cargo-insert-docs
```

## Compatibility

The *Inserts crate documentation from `lib.rs` into your `README.md`.* part of `cargo-insert-docs` relies on the unstable 
[rustdoc JSON](https://github.com/rust-lang/rust/issues/76578) of a recent Rust nightly toolchain. A new nightly release may no longer be compatible with the current version and `cargo-insert-docs` will need to be updated. Alternatively you can choose a specific nightly version that is known to be compatible using the `--toolchain` argument.

#### Compatiblity Matrix

|Version|Understands the rustdoc JSON output of|
|---|---|
|0.1.x|nightly-2025-06-22 — ?|

## Usage

Add feature documentation to your `Cargo.toml` using `##` for documentation of individual features and `#!` to add documentation between features:
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