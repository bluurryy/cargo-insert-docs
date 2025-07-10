# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->
## [Unreleased]

### Added

- Add support for links of all item kinds (enum variant, macro attribute, etc.)
- Don't print extraneous newline

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
[Unreleased]: https://github.com/bluurryy/cargo-insert-docs/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/bluurryy/cargo-insert-docs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/bluurryy/cargo-insert-docs/compare/v0.1.2...v0.2.0
[0.1.2]: https://github.com/bluurryy/cargo-insert-docs/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/bluurryy/cargo-insert-docs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/bluurryy/cargo-insert-docs/compare/v0.1.0...0.1.0
