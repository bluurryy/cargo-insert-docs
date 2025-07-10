default:
    @just --list

pre-release:
    cargo run -- --workspace --check
    just update-cli-md

update-cli-md:
    #!/usr/bin/env nu
    ^stty cols 110
    let s = ^cargo run -q -- --help
    open docs/cli.md 
    | str replace --regex '(?<=```console\n)[\s\S]*?(?=```)' ("$ cargo insert-docs --help\n\n" ++ $s ++ "\n") 
    | save -f docs/cli.md