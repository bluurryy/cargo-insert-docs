use std::path::Path;

use crate::git::{Status, file_status};

#[test]
fn test_example() {
    let paths = [
        "src/main.rs",
        "src/git.rs",
        "Cargo.toml",
        "justfile",
        "target/.rustc_info.json",
        "foobar",
        "src",
    ];
    let status = file_status(paths);

    for (path, status) in paths.iter().zip(status) {
        println!("{path} ({status})");
    }
}

fn check_test_crate(set_cur_dir: bool) {
    let workspace_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tests_dir = workspace_dir.join("tests").join("test-crate");

    if set_cur_dir {
        std::env::set_current_dir(&tests_dir).unwrap();
    }

    let paths = ["lib.rs", "MEREAD.md"].iter().map(|path| tests_dir.join(path)).collect::<Vec<_>>();

    let status = file_status(&paths);

    for (path, status) in paths.iter().zip(&status) {
        let path = path.display();
        println!("{path} ({status})");
    }

    for status in status {
        assert!(matches!(status, Status::Current | Status::Staged | Status::Dirty));
    }
}

#[test]
fn test_outside_subdir() {
    check_test_crate(false);
}

#[test]
fn test_in_subdir() {
    check_test_crate(true);
}
