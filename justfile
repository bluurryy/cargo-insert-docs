default:
    @just --list

pre-release:
    just update-cli-md
    cargo xtask ci
    cargo +nightly test -p test-crate

update-cli-md:
    #!/usr/bin/env nu
    stty cols 120
    let s = cargo run -q -- -h
    open docs/cli.md 
    | str replace --regex '(?<=```console\n)[\s\S]*?(?=```)' ("$ cargo insert-docs -h\n\n" ++ $s ++ "\n") 
    | save -f docs/cli.md