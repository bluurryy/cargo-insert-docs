use std::path::Path;

use git2::{Repository, Status};

pub fn file_status(path: &Path) -> Option<Status> {
    let repository = Repository::discover(path).ok()?;
    let relative_path = path.strip_prefix(repository.path().parent()?).ok()?;
    repository.status_file(relative_path).ok()
}
