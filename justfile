default:
    @just --list

update-cli-md:
    #!/usr/bin/env nu
    let s = ^script --return --quiet --command "stty cols 110; cargo run -q -- --help" /dev/null | ansi strip 
    open docs/cli.md | str replace --regex '(?<=```console\n)[\s\S]*?(?=```)' ("$ cargo insert-docs --help\n\n" ++ $s ++ "\n") | save -f docs/cli.md