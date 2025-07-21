# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->
## [Unreleased]

### Added

- **Breaking?:** Resolve default manifest path and default package using `cargo metadata`
- Improve error messages
- Improve help output with sections

## [0.8.0] - 2025-07-18

### Added

- Add `--allow-staged`
- Don't check whether file is dirty when running `--check`

### Changed

- **Breaking:** Rename `--force` to `--allow-dirty`
- **Breaking:** Remove `-f` shorthand

## [0.7.0] - 2025-07-17

### Added

- Support rustdoc json version 54 (nightly-2025-07-16)
- Better help message when rustdoc format doesn't match

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
[Unreleased]: https://github.com/bluurryy/cargo-insert-docs/compare/v0.8.0...HEAD
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
