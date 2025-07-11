# cargo-insert-docs

[![Crates.io](https://img.shields.io/crates/v/cargo-insert-docs.svg)](https://crates.io/crates/cargo-insert-docs)
[![License](https://img.shields.io/crates/l/cargo-insert-docs)](#license)
[![Build Status](https://github.com/bluurryy/cargo-insert-docs/workflows/Release/badge.svg)](https://github.com/bluurryy/cargo-insert-docs/actions/workflows/release.yml)

`cargo-insert-docs` helps you keep your documentation in sync by doing two jobs, it:
1. Inserts feature documentation from `Cargo.toml` into your `lib.rs`.
2. Inserts crate documentation from `lib.rs` into your `README.md`.

- [Installation](#installation)
- [Usage](#usage)
- [FAQ](#faq)
- [Compatibility](#compatibility)
- [Acknowledgements](#acknowledgements)
- [Similar projects](#similar-projects)
- [License](#license)

## Installation

```sh
# If you have `cargo-binstall` installed:
cargo binstall cargo-insert-docs

# Otherwise
cargo install cargo-insert-docs
```

Inserting crate documentation into the `README.md` requires a nightly toolchain to be installed. See [Compatibility](#compatibility). The active toolchain does not need to be nightly. Inserting the feature documentation into the crate and compiling `cargo-insert-docs` itself does not require a nightly toolchain.

```sh
# To be able to insert crate documentation into the readme
rustup install nightly --profile minimal
```

## Usage

Add feature documentation to your `Cargo.toml` using `##` for documentation of individual features and `#!` to add documentation between features:
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

Then add a feature documentation section to your `lib.rs` file:
```rs
//! Use the [Image] type to load images.
//!
//! # Feature Flags
//! <!-- feature documentation start -->
//! <!-- feature documentation end -->
//!
//! # Examples
//! ```
//! # use example_crate::Image;
//! let image = Image::load("cat.png");
//! # println!("this won't show up in the readme");
//! ```
```

And add a crate documentation section to your `README.md`:
```md
# my-crate-name

Badges go here.

<!-- crate documentation start -->
<!-- crate documentation end -->

License goes there.
```

Now run `cargo-insert-docs`:
```sh
cargo insert-docs
```

Then your `lib.rs` will end up looking like this:
```rs
//! Use the [Image] type to load images.
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
//! # println!("this won't show up in the readme");
//! ```
```

And your `README.md` will look like that:
````md
# my-crate-name

Badges go here.

<!-- crate documentation start -->
Use the [Image](https://docs.rs/example-crate/0.0.0/example_crate/struct.Image.html) type to load images.

## Feature Flags
<!-- feature documentation start -->
- **`std`** *(enabled by default)* — Enables loading [`Image`](https://docs.rs/example-crate/0.0.0/example_crate/struct.Image.html)s
  from [`std::io::Read`](https://doc.rust-lang.org/std/io/trait.Read.html).

### Image formats
The following formats are supported.

- **`jpg`** *(enabled by default)* — Enables support for jpg images
- **`png`** — Enables support for png images
<!-- feature documentation end -->

## Examples
```rust
let image = Image::load("cat.png");
```
<!-- crate documentation end -->

License goes there.
````

You can see the rendered version [here](tests/example-crate/README.md).

Notice how:
- doc-links like `Image` and `std::io::Read` get resolved to links to `docs.rs` or `docs.rust-lang.org`
- the code block loses the hidden (`#` prefixed) lines
- the code block gets marked as `rust`; if the code block already had a marking that is considered rust like `compile_fail`, `ignore`, `should_panic` and such, that would also be replaced by `rust`
- headers get one `#` added

To update the sections just run the command again.

You don't have to add both sections for the tool to work. If it doesn't find a section it will carry on with a warning. You can turn that warning into an error with the `--strict` flag or disable either job with `--no-feature-docs` and `--no-crate-docs`.

You can find details about all the available arguments in [docs/cli.md](docs/cli.md).

If you'd like to see what it looks like when used by a real crate then have a look at `bump-scope`'s [docs.rs](https://docs.rs/bump-scope/latest/bump_scope/) and [README.md](https://github.com/bluurryy/bump-scope/blob/main/README.md).

## FAQ

- **Why not just use `#![doc = include_str!("../README.md")]`?**
 
  If you're using doc-links `cargo-insert-docs` has the advantage that it resolves them for you. Linking to the docs manually by writing `[Image](https://docs.rs/...)` is tedious and rustdoc won't we able to tell you about outdated or otherwise unresolved links.

  Using `cargo-insert-docs` you can also use code sections with `should_panic` and `compile_fail` annotations and [hidden lines](https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html#hiding-portions-of-the-example) without it rendering weird in the readme.

  Furthermore the readme might include things like a header, badges, license that you wouldn't want to include in the crate documentation.

## Compatibility

To extract the crate documentation `cargo-insert-docs` relies on the unstable [rustdoc JSON](https://github.com/rust-lang/rust/issues/76578) format, which requires a recent Rust nightly toolchain to be installed.

A new nightly release may no longer be compatible with the current version and `cargo-insert-docs` will need to be updated. Alternatively you can choose a specific nightly version that is known to be compatible using the `--toolchain` argument.

#### Compatibility Matrix

|Version|Understands the rustdoc JSON output of|
|---|---|
|0.1.x|nightly-2025-06-22 — ?|

## Acknowledgements

The comment format for adding feature documentation comes from [`document-features`](https://docs.rs/document-features/latest/document_features/). `document-features` is a great tool that allows you to insert feature documentation using a macro like this: 
```rs
#![doc = document_features::document_features!()]
```

Thank you to `cargo-rdme` for the idea to use html comment tags to 
delimit the sections.

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