default:
    @just --list

pre-release:
    # TODO: --check should not require --force
    cargo run -- --workspace --check --force
    just update-cli-md
    just test
    just test-recurse recurse
    just test-recurse recurse-glob

update-cli-md:
    #!/usr/bin/env nu
    stty cols 110
    let s = cargo run -q -- --help
    open docs/cli.md 
    | str replace --regex '(?<=```console\n)[\s\S]*?(?=```)' ("$ cargo insert-docs --help\n\n" ++ $s ++ "\n") 
    | save -f docs/cli.md

test:
    #!/usr/bin/env nu
    let out = cargo test --color always -- --color always

    print $out

    let tests_that_need_to_be_run_separately = $out
    | ansi strip
    | parse -r '(?m)^test (?<name>.*)? \.\.\. (?<result>.*)$' 
    | where result == "ignored, needs to be run separately because of hooks" 
    | get name

    print -e $"(ansi cyan_bold)NOW RUNNING PREVIOUSLY IGNORED TESTS(ansi reset)"

    for test in $tests_that_need_to_be_run_separately {
        let out = cargo test --package cargo-insert-docs --bin cargo-insert-docs --all-features -- $test --color always --exact --show-output --ignored 
        | complete 
        | get stdout
        | parse -r '(?m)(?<all>^test (?<name>.*)? \.\.\. (?<result>.*)$)' 
        | get all.0

        print $out
    }


test-recurse feature:
    #!/usr/bin/env nu
    let out = (cargo run -- -p test-crate -F {{feature}} -f | complete).stderr
    if not ($out | str contains "recursed too deep while resolving item paths") {
        print -e $out
        exit 1
    }
