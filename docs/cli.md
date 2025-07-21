### CLI Arguments

```console
$ cargo insert-docs -h

Inserts crate docs into a readme file and feature docs into the crate docs.

Usage: cargo insert-docs [OPTIONS]

Options:
      --readme-path <PATH>                   Readme path relative to the package manifest [default: README.md]
      --feature-label <FEATURE_LABEL>        Formatting of the feature label [default: **`{feature}`**]
      --feature-docs-section <SECTION_NAME>  Name of the feature documentation section [default: "feature
                                             documentation"]
      --crate-docs-section <SECTION_NAME>    Name of the crate documentation section [default: "crate
                                             documentation"]
      --link-to-latest                       Link to the "latest" version on docs.rs
  -h, --help                                 Print help (see more with '--help')
  -V, --version                              Print version

Cargo Doc Options:
      --document-private-items  Document private items

Mode Selection:
      --check            Runs in 'check' mode, erroring if something is out of date
      --no-feature-docs  Disables inserting the feature documentation into the crate documentation
      --no-crate-docs    Disables inserting the crate documentation into the readme

Error Behavior:
      --strict               Error when a section is missing
      --strict-feature-docs  Error when a feature documentation section is missing
      --strict-crate-docs    Error when a crate documentation section is missing
      --allow-dirty          Insert documentation even if the affected file is dirty or has staged changes
      --allow-staged         Insert documentation even if the affected file has staged changes

Message Options:
  -v, --verbose      Print more verbose messages
  -q, --quiet        Do not print anything
      --quiet-cargo  Do not print cargo log messages

Package Selection:
  -p, --package <SPEC>  Package(s) to document
      --workspace       Document all packages in the workspace
      --exclude <SPEC>  Exclude package(s) from documenting

Feature Selection:
  -F, --features <FEATURES>  Space or comma separated list of features to activate
      --all-features         Activate all available features
      --no-default-features  Do not activate the `default` feature

Compilation Options:
      --toolchain <TOOLCHAIN>  Which rustup toolchain to use when invoking rustdoc. [default: nightly]
      --target <TRIPLE>        Target triple to document

Manifest Options:
      --manifest-path <PATH>  Path to Cargo.toml
```