default:
    @just --list

pre-release:
    cargo fmt --check
    cargo clippy --all-features --tests --examples -- -D warnings
    cargo xtask
    just update-cli-md
    cargo +nightly test -p test-crate

update:
    just update-cli-md
    just update-expect

update-cli-md:
    #!/usr/bin/env nu
    stty cols 120
    let s = cargo run -q -- -h
    open --raw docs/cli.md 
    | str replace --regex '(?<=```console\n)[\s\S]*?(?=```)' ("$ cargo insert-docs -h\n\n" ++ $s ++ "\n") 
    | save -f docs/cli.md

update-expect:
    cargo run -- -p test-crate
    UPDATE_EXPECT=1 cargo xtask

get-json crate:
    cat ./target/insert-docs/doc/{{ crate }}.json
