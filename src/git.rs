use std::path::Path;

use git2::{Repository, Status};

pub fn is_file_dirty(path: &Path) -> Option<bool> {
    let repository = Repository::discover(path).ok()?;
    let relative_path = path.strip_prefix(repository.path().parent()?).ok()?;
    let status = repository.status_file(relative_path).ok()?;
    Some(status != Status::CURRENT && !status.contains(Status::IGNORED))
}
