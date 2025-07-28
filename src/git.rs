use std::path::Path;

use git2::Repository;

pub fn file_status(path: &Path) -> Option<Status> {
    let repository = Repository::discover(path).ok()?;
    let relative_path = path.strip_prefix(repository.path().parent()?).ok()?;
    repository.status_file(relative_path).ok().map(Into::into)
}

pub enum Status {
    Current,
    Staged,
    Dirty,
}

impl From<git2::Status> for Status {
    fn from(value: git2::Status) -> Self {
        match value {
            git2::Status::CURRENT => Status::Current,
            git2::Status::INDEX_NEW
            | git2::Status::INDEX_MODIFIED
            | git2::Status::INDEX_DELETED
            | git2::Status::INDEX_RENAMED
            | git2::Status::INDEX_TYPECHANGE => Status::Staged,
            _ => Status::Dirty,
        }
    }
}
