### CLI Arguments

```console
$ cargo insert-docs --help

Inserts feature documentation into the crate documentation and the crate documentation into the readme.

Website: https://github.com/bluurryy/cargo-insert-docs

Usage: cargo insert-docs [OPTIONS]

Options:
      --readme-path <PATH>
          Readme path relative to the package manifest
          
          [default: README.md]

      --feature-label <FEATURE_LABEL>
          Formatting of the feature label
          
          When inserting feature documentation into the crate documentation.
          
          [default: **`{feature}`**]

      --feature-docs-section <SECTION_NAME>
          Name of the feature documentation section
          
          [default: "feature documentation"]

      --crate-docs-section <SECTION_NAME>
          Name of the crate documentation section
          
          [default: "crate documentation"]

      --link-to-latest
          Link to the "latest" version on docs.rs
          
          For example https://docs.rs/my-crate/latest/my_crate/.
          This only affects workspace crates.

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

Cargo Doc Options:
      --document-private-items
          Document private items

Mode Selection:
      --check
          Runs in 'check' mode, erroring if something is out of date
          
          Exits with 0 if the documentation is up to date.
          Exits with 1 if the documentation is stale or if any errors occured.

      --no-feature-docs
          Disables inserting the feature documentation into the crate documentation

      --no-crate-docs
          Disables inserting the crate documentation into the readme

Error Behavior:
      --strict
          Error when a section is missing
          
          Implies `--strict-feature-docs` and `--strict-crate-docs`.

      --strict-feature-docs
          Error when a feature documentation section is missing

      --strict-crate-docs
          Error when a crate documentation section is missing

      --allow-dirty
          Insert documentation even if the affected file is dirty or has staged changes

      --allow-staged
          Insert documentation even if the affected file has staged changes

Message Options:
  -v, --verbose
          Print more verbose messages

  -q, --quiet
          Do not print anything

      --quiet-cargo
          Do not print cargo log messages

Package Selection:
  -p, --package <SPEC>
          Package(s) to document

      --workspace
          Document all packages in the workspace

      --exclude <SPEC>
          Exclude package(s) from documenting

Feature Selection:
  -F, --features <FEATURES>
          Space or comma separated list of features to activate

      --all-features
          Activate all available features

      --no-default-features
          Do not activate the `default` feature

Compilation Options:
      --toolchain <TOOLCHAIN>
          Which rustup toolchain to use when invoking rustdoc.
          
          Whenever you update your nightly toolchain this tool may also need to be
          updated to be compatible.
          
          With this argument you can choose a nightly version that is guaranteed to be compatible
          with the current version of this tool, like `nightly-2025-07-16`.
          
          [default: nightly]

      --target <TRIPLE>
          Target triple to document

Manifest Options:
      --manifest-path <PATH>
          Path to Cargo.toml
```