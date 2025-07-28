default:
    @just --list

pre-release:
    cargo run -- --check -p test-crate --feature-section-name "features" --crate-section-name "docs"
    cargo run -- --check -p test-document-features crate-into-readme
    cargo run -- --check -p example-crate
    cargo run -- --check -p test-bin crate-into-readme
    cargo run -- --check --workspace --exclude test-crate --exclude cargo-insert-docs crate-into-readme
    just update-cli-md
    just test
    just test-recurse recurse
    just test-recurse recurse-glob

update-cli-md:
    #!/usr/bin/env nu
    stty cols 120
    let s = cargo run -q -- -h
    open docs/cli.md 
    | str replace --regex '(?<=```console\n)[\s\S]*?(?=```)' ("$ cargo insert-docs -h\n\n" ++ $s ++ "\n") 
    | save -f docs/cli.md

test:
    #!/usr/bin/env nu
    let out = cargo test --color always -- --color always | tee { print }

    let tests_that_need_to_be_run_separately = $out
    | ansi strip
    | parse -r '(?m)^test (?<name>.*)? \.\.\. (?<result>.*)$' 
    | where result == "ignored, needs to be run separately because of hooks" 
    | get name

    print -e $"(ansi cyan_bold)NOW RUNNING PREVIOUSLY IGNORED TESTS(ansi reset)"

    for test in $tests_that_need_to_be_run_separately {
        cargo test --package cargo-insert-docs --bin cargo-insert-docs --all-features -- $test --color always --exact --show-output --ignored 
        | complete 
        | get stdout
        | parse -r '(?m)(?<all>^test (?<name>.*)? \.\.\. (?<result>.*)$)' 
        | get all
        | each { print }
    }

test-recurse feature:
    #!/usr/bin/env nu
    let out = (cargo run -- -p test-crate --feature-section-name "features" --crate-section-name "docs" -F {{feature}} --allow-dirty | complete).stderr | tee { print }
    if not ($out | str contains "recursed too deep while resolving item paths") {
        print -e $out
        exit 1
    }
