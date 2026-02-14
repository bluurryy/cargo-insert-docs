# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->
## [Unreleased]

### Changed

- Update default nightly toolchain to `nightly-2026-02-12`

### Fixed

- Fix url for trait provided methods

## [1.3.0] - 2026-01-02

### Added

- Add `hidden-features` setting to hide features from the documentation

## [1.2.0] - 2025-12-29

### Added

- Add support for parsing toml version 1.1.0 files

### Fixed

- Allow other lib-like targets like `dylib`, `cdylib`, `rlib`, `staticlib` and `proc-macro` by @DenisGorbachev in #19

## [1.1.0] - 2025-12-06

### Added

- Update default nightly toolchain to `nightly-2025-12-05` (rustdoc json version 57)

## [1.0.0] - 2025-11-12

_This release does not make any breaking changes. The version is bumped to `1.0.0` to reflect the status of this project._ 

### Added

- Update to cargo `1.91.0` cli colors
- Update default nightly toolchain to `nightly-2025-11-11`

## [0.19.1] - 2025-09-14

### Added

- Improve documentation

### Fixed

- Fix doc link to trait methods

## [0.19.0] - 2025-09-08

### Added

- Parse code block language like rustdoc
- Consistent order for warnings about `failed to resolve doc link` by sorting the doc links beforehand

### Changed

- **Breaking:** Upgrade to rustdoc json version 56 (nightly-2025-09-08)

## [0.18.1] - 2025-08-31

### Fixed

- Fix removing hidden lines of code blocks that are quoted, in a list, etc.

## [0.18.0] - 2025-08-30

### Added

- Remove link definitions with unresolved doc links

### Changed

- Removed buggy fallback for a cargo version < 1.71, `cargo-insert-docs` now requires cargo >= 1.71

### Fixed

- Fix errors when multiple sections are inside html tags
- Fix `(error: unknown)` when checking git status of files when `cargo-insert-docs` is run in a subfolder

## [0.17.0] - 2025-08-29

### Changed

- Now creates markdown link definitions for doc references instead of converting references into links

### Fixed

- Fix handling of quotes in link titles
- Fix handling of collapsed references

## [0.16.1] - 2025-08-26

### Added 

- Add `shrink-headings` option that allows you to configure the amount markdown headings shrink when inserting documentation into the readme.

## [0.16.0] - 2025-08-20

### Changed

- Change default target directory to the workspace target directory joined with `insert-docs` to not clean the cache of regular `cargo doc` runs and vice versa

## [0.15.0] - 2025-08-20

### Fixed

- Fix version control error message to mention `cargo insert-docs` instead of `cargo fix`
- Fix error message for an incompatible nightly toolchain to take into account that this error does not happen unless the user chooses a toolchain explicitly

### Changed

- Replace dependency `git2` with `gix`

## [0.14.0] - 2025-08-18

### Added

- Add support for configuration values in `workspace.metadata.insert-docs` and `package.metadata.insert-docs`. See [docs/config.md](docs/config.md).
- Add `--print-config` to print configuration values and their sources
- Add `--print-supported-toolchain` argument to print a supported nightly toolchain
- Add even more verbose messages with `-vv`
- Don't turn link references with a definition into inline links
- Replace intra doc links in link definitions, like `[Foo]: my_mod::Foo` too

### Fixed

- Fix ignored attributes in code blocks

### Changed

- **Breaking:** The default toolchain is now a known compatible one instead of `nightly`

## [0.13.0] - 2025-08-14

### Added

- Add support for inserting subsections of the crate documentation
  by adding a html comment tag in both crate docs and readme like
  ```md
  <!-- crate documentation my_subsection_name start -->
  some content...
  <!-- crate documentation my_subsection_name end -->
  ``` 

## [0.12.0] - 2025-08-03

### Changed

- **Breaking:** Upgrade to rustdoc json version 55 (nightly-2025-08-02)

## [0.11.0] - 2025-07-28

### Added

- Add help for subcommands `feature-into-crate` and `crate-into-readme`

### Changed

- **Breaking:** Require `--workspace` for `--exclude` just like other cargo tools

## [0.10.0] - 2025-07-28

### Added

- Add `feature-into-crate` command that only inserts feature documentation into crate documentation 
- Add `crate-into-readme` command only inserts crate documentation into the readme
- Add `--allow-missing-section` argument to not error when a section is missing
- Improve `--check` argument help

### Changed

- **Breaking:** Error when a section is missing, you can suppress the error with `--allow-missing-section`

### Removed

- **Breaking:** Remove `--no-*-section` arguments, use `feature-into-crate` and `crate-into-readme` instead
- **Breaking:** Remove `--strict*` arguments, the behavior is now "strict" by default

## [0.9.0] - 2025-07-22

### Added

- **Breaking?:** Resolve default manifest path and default package using `cargo metadata`
- **Breaking:** Use `readme` field in the `Cargo.toml` as default readme path
- Add `--bin`, `--color`, `--no-deps` and `--target-dir` argument
- Improve error messages
- Improve help output with sections

### Changed

- **Breaking:** Rename `--feature-docs-section` to `--feature-section-name`
- **Breaking:** Rename `--crate-docs-section` to `--crate-section-name`
- **Breaking:** Rename `--no-feature-docs` to `--no-feature-section`
- **Breaking:** Rename `--no-crate-docs` to `--no-crate-section`
- **Breaking:** Rename `--strict-feature-docs` to `--strict-feature-section`
- **Breaking:** Rename `--strict-crate-docs` to `--strict-crate-section`

## [0.8.0] - 2025-07-18

### Added

- Add `--allow-staged`
- Don't check whether file is dirty when running `--check`

### Changed

- **Breaking:** Rename `--force` to `--allow-dirty`
- **Breaking:** Remove `-f` shorthand

## [0.7.0] - 2025-07-17

### Added

- Better help message when rustdoc format doesn't match

### Changed

- **Breaking:** Upgrade to rustdoc json version 54 (nightly-2025-07-16)

## [0.6.0] - 2025-07-17

### Added

- **Breaking:** Return early with an error when any affected file is uncommitted, unless `--force`d
- Add better support for glob imports
- Error instead of stack overflow for recursive items
- More optimized release binaries
- Use `mimalloc` as global allocator

## [0.5.0] - 2025-07-13

### Added

- Upgrade `cargo_metadata` to version 0.21
- Use `tracing` for logs, errors with span traces
- Support `RUST_LOG` env var for debugging

### Fixed

- Fix "could not could not ..." in logs

## [0.4.0] - 2025-07-11

### Fixed

- Module doc links now end in `/index.html` instead of `/`
- Only warn about missing readme when not `--strict(-crate-docs)`
- Fix broken doc links to crates containing `//`

## [0.3.0] - 2025-07-11

### Added

- Add support for links of all item kinds (enum variant, macro attribute, etc.)
- Don't print extraneous newline
- Print stderr of cargo on error even if `--quiet-cargo` is set

## [0.2.1] - 2025-07-10

### Added

- Add `--check` flag to check if the documentation is up to date.
- Improve `--help` output.

## [0.2.0] - 2025-07-10

### Added

- Trim whitespace from the end of feature doc lines to satisfy rustfmt

### Changed

- Make github builds target `x86_64-unknown-linux-musl` instead of `x86_64-unknown-linux-gnu`

## [0.1.2] - 2025-07-10

### Added

- Improve cli documentation.

### Changed

- Upgrade `toml_edit` dependency to version 0.23.0

## [0.1.1] - 2025-07-09

### Fixed

- Fix running as a cargo subcommand

## [0.1.0] - 2025-07-09

<!-- next-url -->
[Unreleased]: https://github.com/bluurryy/cargo-insert-docs/compare/v1.3.0...HEAD
[1.3.0]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v1.3.0
[1.2.0]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v1.2.0
[1.1.0]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v1.1.0
[1.0.0]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v1.0.0
[0.19.1]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v0.19.1
[0.19.0]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v0.19.0
[0.18.1]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v0.18.1
[0.18.0]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v0.18.0
[0.17.0]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v0.17.0
[0.16.1]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v0.16.1
[0.16.0]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v0.16.0
[0.15.0]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v0.15.0
[0.14.0]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v0.14.0
[0.13.0]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v0.13.0
[0.12.0]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v0.12.0
[0.11.0]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v0.11.0
[0.10.0]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v0.10.0
[0.9.0]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v0.9.0
[0.8.0]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v0.8.0
[0.7.0]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v0.7.0
[0.6.0]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v0.6.0
[0.5.0]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v0.5.0
[0.4.0]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v0.4.0
[0.3.0]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v0.3.0
[0.2.1]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v0.2.1
[0.2.0]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v0.2.0
[0.1.2]: https://github.com/bluurryy/cargo-insert-docs/releases/tag/v0.1.2
[0.1.1]: https://github.com/bluurryy/cargo-insert-docs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/bluurryy/cargo-insert-docs/compare/v0.1.0...0.1.0
