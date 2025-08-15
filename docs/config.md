You can configure `cargo-insert-docs` using [cli arguments](cli.md) and by adding fields to the `Cargo.toml` in the
 `[package.metadata.insert-docs]` or `[workspace.metadata.insert-docs]` table. 

Here is an example what setting fields in the `Cargo.toml` looks like: [../tests/test-config/Cargo.toml](../tests/test-config/Cargo.toml).

Configuration fields are read in the following (in precedence) order:
- Command line arguments
- `[package.metadata.insert-docs]`
- `[workspace.metadata.insert-docs]`

## Cli, Workspace and Package fields

These fields can be set in the cli, `[workspace.metadata.insert-docs]` and `[package.metadata.insert-docs]`.

#### Commands

For the cli, these are not arguments but subcommands instead and calling the subcommand sets the other field to `false`.

|Field|Type|Default|Description|
|---|---|---|---|
|feature-into-crate|bool|true|Insert feature documentation into the crate docs|
|crate-into-readme|bool|true|Insert crate documentation into the readme|

#### Options

|Field|Type|Default|Description|
|---|---|---|---|
|feature-label|string|``"**\`{feature}\`**"``|Formatting of the feature label
|feature-section-name|string|`"feature documentation"`|Feature documentation section name|
|crate-section-name|string|`"crate documentation"`|Crate documentation section name|
|link-to-latest|bool|false|Link to the "latest" version on docs.rs. This only affects workspace crates.

#### Mode Selection
|Field|Type|Default|Description|
|---|---|---|---|
|check|bool|false|Runs in 'check' mode, not writing to files but erroring if something is out of date|

#### Error Behavior
|Field|Type|Default|Description|
|---|---|---|---|
|allow-missing-section|bool|false|Don't error when a section is missing
|allow-dirty|bool|false|Insert documentation even if the affected file is dirty or has staged changes
|allow-staged|bool|false|Insert documentation even if the affected file has staged changes

#### Feature Selection
|Field|Type|Default|Description|
|---|---|---|---|
|features|string list||List of features to activate
|all-features|bool|false|Activate all available features
|no-default-features|bool|false|Do not activate the `default` feature

#### Target Selection
|Field|Type|Default|Description|
|---|---|---|---|
|lib|bool||Document only the library targets
|bin|string or bool||Document only the specified binary

#### Compilation Options
|Field|Type|Default|Description|
|---|---|---|---|
|toolchain|string|`"nightly-2025-08-02"`|Which rustup toolchain to use when invoking rustdoc.
|target|string||Target triple to document
|target-dir|string||Directory for all generated artifacts

## Cli and Workspace fields

These fields can be set in the cli and `[workspace.metadata.insert-docs]`.

|Field|Type|Default|Description|
|---|---|---|---|
|package|string list||Package(s) to document
|workspace|bool|false|Document all packages in the workspace
|exclude|string list||Exclude package(s) from documenting

## Cli only fields

These fields can only be set in the cli.

|Field|Type|Default|Description|
|---|---|---|---|
|manifest-path|path||Path to Cargo.toml
|print-supported-toolchain|bool|false|Print the supported toolchain and quits|
|print-config|bool|false|Prints configuration values and their sources for debugging and quits|
|color|string|`auto`, `always`, `never`|Printed messages coloring|
|verbose|bool|false|Print more verbose messages|
|quiet|bool|false|Do not print anything|
|quiet-cargo|bool|false|Do not print cargo log messages