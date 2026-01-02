# cargo-insert-docs

[![Crates.io](https://img.shields.io/crates/v/cargo-insert-docs.svg)](https://crates.io/crates/cargo-insert-docs)
[![License](https://img.shields.io/crates/l/cargo-insert-docs)](#license)
[![Build Status](https://github.com/bluurryy/cargo-insert-docs/workflows/Release/badge.svg)](https://github.com/bluurryy/cargo-insert-docs/actions/workflows/release.yml)
[![Build Status](https://github.com/bluurryy/cargo-insert-docs/workflows/CI/badge.svg)](https://github.com/bluurryy/cargo-insert-docs/actions/workflows/ci.yml)

This tool can:
- insert feature documentation from `Cargo.toml` into `lib.rs` and
- insert crate documentation from `lib.rs` into `README.md`.

## Table of Contents

- [Installation](#installation)
- [Usage](#usage)
  - [Insert feature documentation from `Cargo.toml` into `lib.rs`](#insert-feature-documentation-from-cargotoml-into-librs)
  - [Insert crate documentation from `lib.rs` into `README.md`](#insert-crate-documentation-from-librs-into-readmemd)
  - [Run the command](#run-the-command)
  - [Result](#result)
  - [Crate documentation subsections](#crate-documentation-subsections)
- [Configuration](#configuration)
- [Semver policy](#semver-policy)
- [CI Integration](#ci-integration)
- [FAQ](#faq)
- [Known Issues](#known-issues)
- [Acknowledgements](#acknowledgements)
- [Similar projects](#similar-projects)

## Installation

```sh
# If you have `cargo-binstall` installed:
cargo binstall cargo-insert-docs

# Otherwise
cargo install cargo-insert-docs
```

To extract the crate documentation, `cargo-insert-docs` invokes `cargo +nightly-2025-12-05 rustdoc`. This will automatically install the required nightly toolchain.

## Usage

### Insert feature documentation from `Cargo.toml` into `lib.rs`

Document features using `##`, and add documentation between features using `#!`:
```toml
[features]
default = ["std", "jpg"]

## Enables loading [`Image`]s from [`std::io::Read`].
std = []

#! ## Image formats
#! The following formats are supported.

## Enables support for jpg images
jpg = []

## Enables support for png images
png = []
```

Then add a feature documentation section to `lib.rs`:
```rs
//! Use the [`Image`] type to load images.
//!
//! # Feature Flags
//! <!-- feature documentation start -->
//! <!-- feature documentation end -->
//!
//! # Examples
//! ```
//! # use example_crate::Image;
//! let image = Image::load("cat.png");
//! ```
```

### Insert crate documentation from `lib.rs` into `README.md`

Add a crate documentation section to your `README.md`:
```md
# my-crate-name

Badges go here.

<!-- crate documentation start -->
<!-- crate documentation end -->

License goes there.
```

### Run the command

```sh
cargo insert-docs
```

### Result

This will insert the feature documentation into `lib.rs`:
```rs
//! Use the [`Image`] type to load images.
//!
//! # Feature Flags
//! <!-- feature documentation start -->
//! - **`std`** *(enabled by default)* — Enables loading [`Image`]s
//!   from [`std::io::Read`].
//!
//! ## Image formats
//! The following formats are supported.
//!
//! - **`jpg`** *(enabled by default)* — Enables support for jpg images
//! - **`png`** — Enables support for png images
//! <!-- feature documentation end -->
//!
//! # Examples
//! ```
//! # use example_crate::Image;
//! let image = Image::load("cat.png");
//! ```
```

And crate documentation into `README.md`:
````md
# my-crate-name

Badges go here.

<!-- crate documentation start -->
Use the [`Image`] type to load images.

## Feature Flags
<!-- feature documentation start -->
- **`std`** *(enabled by default)* — Enables loading [`Image`]s
  from [`std::io::Read`].

### Image formats
The following formats are supported.

- **`jpg`** *(enabled by default)* — Enables support for jpg images
- **`png`** — Enables support for png images
<!-- feature documentation end -->

## Examples
```rust
let image = Image::load("cat.png");
```

[`Image`]: https://docs.rs/example-crate/0.0.0/example_crate/struct.Image.html
[`std::io::Read`]: https://doc.rust-lang.org/std/io/trait.Read.html

<!-- crate documentation end -->

License goes there.
````

You can see the rendered version [here](tests/example-crate/README.md).

Notice how:
- link definitions for `Image` and `std::io::Read` are added that resolve to `docs.rs` or `doc.rust-lang.org`
- the code block loses the hidden (`#` prefixed) lines
- the code block gets marked as `rust`; if the code block already had a marking that is considered rust like `compile_fail`, `ignore`, `should_panic` and such, that would also be replaced by `rust`
- headings get one `#` added (configurable)

To update the sections just run the command again.

By default, `cargo-insert-docs` tries to insert both feature documentation and crate documentation. To perform only one of these actions use the `feature-into-crate` or `crate-into-readme` subcommand.

### Crate documentation subsections

Instead of inserting the entire crate documentation into the readme you can also insert subsections into the readme. Here is an example:
- lib.rs:
  ```rs
  //! <!-- crate documentation intro start --> 
  //! This is my crate. Bla bla.
  //! <!-- crate documentation intro end --> 
  //! 
  //! <!-- crate documentation rest start -->
  //! ## Features
  //! ...
  //! <!-- crate documentation rest end -->
  ```
- README.md:
  ```md
  <!-- crate documentation intro start -->
  <!-- crate documentation intro end -->

  ## Table of Contents
  - [Features](#features)

  <!-- crate documentation rest start -->
  <!-- crate documentation rest end -->
  ```
This is useful if you want a table of contents in the readme but don't want it in the crate docs because the crate docs already have a side panel for that.

If you'd like to see what this looks like when used by a real crate then have a look at `bump-scope`'s [docs.rs](https://docs.rs/bump-scope/latest/bump_scope/) and [README.md](https://github.com/bluurryy/bump-scope/blob/main/README.md).

## Configuration

You can configure `cargo-insert-docs` using the cli and via metadata tables inside `Cargo.toml`.

See [docs/config.md](docs/config.md) for details.

## Semver policy

These things are considered minor changes:
 - changes to the produced documentation
 - updating the default nightly version of `rustdoc` we invoke
 - updating the supported rustdoc json version (and losing compatibility with the older version)

So if you require consistent output, pin the version of `cargo-insert-docs` you use.

## CI Integration

To verify that the documentation is up to date, you can use the `--check` argument.

You can automate this using GitHub Actions with job steps like these:

```yml
- uses: actions/checkout@v4
- uses: taiki-e/install-action@v2
  with:
    tool: cargo-insert-docs@1.2.0
- run: cargo insert-docs --check --all-features
```

## FAQ

- **Why not `#![doc = include_str!("../README.md")]`?**
 
  If you're using doc-links `cargo-insert-docs` has the advantage that it resolves them for you. Linking to the docs manually by writing `[Image](https://docs.rs/...)` is tedious and rustdoc won't we able to tell you about outdated or otherwise unresolved links.

  Using `cargo-insert-docs` you can also use code sections with `should_panic` and `compile_fail` annotations and [hidden lines](https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html#hiding-portions-of-the-example) with it still rendering nice in the readme.

  Furthermore the readme may include things like a header, badges, license that you might not want to include in the crate documentation.

## Known Issues

- **Can't resolve links to certain items in foreign crates**

  Due to limitations of the rustdoc json format, `cargo-insert-docs` currently can't resolve the following items in foreign crates:
  - methods
  - enum variants
  - associated types
  - associated constants

- **Can't resolve rescursive imports and some cases of glob imports**

  The rustdoc json output currently doesn't give us information what imports resolve to.
  Resolving imports can be very complicated with cycles, shadowing, renaming and visibility, see [rustdoc-types#51](https://github.com/rust-lang/rustdoc-types/issues/51) and [rust#111338](https://github.com/rust-lang/rust/issues/111338). I have no plans of re-implementing proper rust name resolution myself and hope it becomes part of the json format at some point.

## Acknowledgements

The comment format for adding feature documentation comes from [`document-features`](https://crates.io/crates/document-features).
The crate documentation into readme part was inspired by [`cargo-rdme`](https://crates.io/crates/cargo-rdme).

## Similar projects
- [**`document-features`**](https://crates.io/crates/document-features) — 
  Provides a proc macro to insert feature documentation into the crate documentation. 
- [**`cargo-rdme`**](https://crates.io/crates/cargo-rdme) — 
  Subcommand to insert crate documentation into readme sections.
- [**`cargo-sync-rdme`**](https://crates.io/crates/cargo-sync-rdme) — 
  Subcommand to insert crate documentation, title and badges into readme sections.
- [**`cargo-doc2readme`**](https://crates.io/crates/cargo-doc2readme) — 
  Subcommand that creates a readme file from a template with various fields including crate documentation.
- [**`cargo-readme`**](https://crates.io/crates/cargo-readme) — 
  Subcommand that creates a readme file from a template with various fields including crate documentation.

## License

Licensed under either of:

 * MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)
 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)

at your option.

---

This project includes code adapted from the Rust standard library 
(<https://github.com/rust-lang/rust>), 
Copyright © The Rust Project Developers and cargo (<https://github.com/rust-lang/cargo>).
Those projects are also licensed under MIT OR Apache-2.0.

---

This project also vendors code from the
[`markdown-rs`](https://github.com/wooorm/markdown-rs),
which is licensed under the MIT license only
([LICENSE](src/markdown_rs/LICENSE))

### Your contributions

Unless you explicitly state otherwise,
any contribution intentionally submitted for inclusion in the work by you,
as defined in the Apache-2.0 license, 
shall be dual licensed as above,
without any additional terms or conditions.