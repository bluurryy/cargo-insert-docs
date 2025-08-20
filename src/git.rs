use core::fmt;
use std::{
    collections::{HashMap, HashSet, hash_map},
    path::{Path, PathBuf},
};

use arcstr::ArcStr;
use gix::bstr::BString;
use indexmap::IndexMap;
use relative_path::PathExt;

/// Statuses are returned in the same order as the `paths`.
pub fn file_status(paths: impl IntoIterator<Item: AsRef<Path>>) -> Vec<Status> {
    let mut checker = StatusChecker::default();

    for path in paths {
        checker.add(path.as_ref());
    }

    checker.finish()
}

#[derive(Default)]
struct StatusChecker {
    repos: HashMap<PathBuf, RepoAndPaths>,
    statuses: IndexMap<PathBuf, Option<Status>>,
    results: Vec<ResultKind>,
}

struct RepoAndPaths {
    repo: gix::Repository,
    paths: HashSet<BString>,
}

enum ResultKind {
    Status(usize),
    Error(Error),
    Orphan,
}

impl StatusChecker {
    fn add(&mut self, path: &Path) {
        let path = match std::path::absolute(path) {
            Ok(ok) => ok,
            Err(err) => return self.results.push(ResultKind::Error(Error::new(err))),
        };

        match self.try_add(&path) {
            TryAdd::Ok => {
                let (index, _) = self.statuses.insert_full(path, None);
                self.results.push(ResultKind::Status(index))
            }
            TryAdd::Orphan => self.results.push(ResultKind::Orphan),
            TryAdd::Err(err) => self.results.push(ResultKind::Error(err)),
        }
    }

    fn try_add(&mut self, path: &Path) -> TryAdd {
        match path.try_exists() {
            Ok(true) => (),
            Ok(false) => return TryAdd::Err(error!("path does not exist")),
            Err(err) => return TryAdd::Err(Error::new(err)),
        };

        let repo = match self.repo_at(path) {
            Ok(Some(repo)) => repo,
            Ok(None) => return TryAdd::Orphan,
            Err(err) => return TryAdd::Err(Error::new(err)),
        };

        let workdir = match repo.repo.workdir() {
            Some(some) => some,
            None => return TryAdd::Orphan,
        };

        let relative_path = match path.relative_to(workdir) {
            Ok(ok) => ok,
            Err(err) => return TryAdd::Err(Error::new(err)),
        };

        repo.paths.insert(relative_path.into_string().into());
        TryAdd::Ok
    }

    fn repo_at(&mut self, path: &Path) -> Result<Option<&mut RepoAndPaths>> {
        let path = match path.parent() {
            Some(some) => some,
            None => return Err(error!("path has no parent")),
        };

        let repo_path = match gix::discover::upwards(path) {
            Ok(ok) => ok.0.into_repository_and_work_tree_directories().0,
            Err(err) => {
                return match err {
                    gix::discover::upwards::Error::NoGitRepository { .. }
                    | gix::discover::upwards::Error::NoGitRepositoryWithinCeiling { .. }
                    | gix::discover::upwards::Error::NoGitRepositoryWithinFs { .. } => Ok(None),
                    _ => Err(Error::new(err)),
                };
            }
        };

        Ok(Some(match self.repos.entry(repo_path) {
            hash_map::Entry::Occupied(entry) => entry.into_mut(),
            hash_map::Entry::Vacant(entry) => {
                let repo = gix::open(entry.key()).unwrap();
                entry.insert(RepoAndPaths { repo, paths: Default::default() })
            }
        }))
    }

    fn finish(self) -> Vec<Status> {
        let Self { repos, results, mut statuses } = self;

        for RepoAndPaths { repo, paths } in repos.into_values() {
            let items = match repo_status(&repo, paths.iter().cloned()) {
                Ok(ok) => ok,
                Err(err) => {
                    let err = Error::new(err);

                    for rela_path in paths {
                        let path = repo.workdir_path(rela_path).unwrap();
                        statuses.insert(path, Some(Status::Error(err.clone())));
                    }

                    continue;
                }
            };

            for item in items {
                let new_status = match &item {
                    gix::status::Item::IndexWorktree(item) => match item {
                        gix::status::index_worktree::Item::Modification { .. } => Status::Dirty,
                        gix::status::index_worktree::Item::DirectoryContents { entry, .. } => {
                            Status::from_entry_status(&entry.status)
                        }
                        gix::status::index_worktree::Item::Rewrite { dirwalk_entry, .. } => {
                            Status::from_entry_status(&dirwalk_entry.status)
                        }
                    },
                    gix::status::Item::TreeIndex(_) => Status::Staged,
                };

                let rela_path = item.location();
                let path = repo.workdir_path(rela_path).unwrap();

                if let Some(old_status) = statuses.get_mut(&path) {
                    merge(old_status, new_status);
                }
            }
        }

        results
            .into_iter()
            .map(|result_kind| match result_kind {
                ResultKind::Status(index) => core::mem::take(&mut statuses[index])
                    .unwrap_or(Status::Error(error!("unknown"))),
                ResultKind::Error(error) => Status::Error(error),
                ResultKind::Orphan => Status::Orphan,
            })
            .collect()
    }
}

fn repo_status(
    repo: &gix::Repository,
    paths: impl IntoIterator<Item = BString>,
) -> Result<Vec<gix::status::Item>> {
    let status = repo.status(gix::progress::Discard).map_err(Error::new)?;

    status
        .dirwalk_options(|o| {
            o.emit_tracked(true)
                .emit_untracked(gix::dir::walk::EmissionMode::Matching)
                .emit_ignored(Some(gix::dir::walk::EmissionMode::Matching))
        })
        .into_iter(paths)
        .map_err(Error::new)?
        .map(|result| result.map_err(Error::new))
        .collect()
}

enum TryAdd {
    Ok,
    Orphan,
    Err(Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    // Not part of any git repository.
    Orphan,
    // This file is ignored.
    Ignored,
    // There are no changes.
    Current,
    // There are staged changes.
    Staged,
    // There are changes in the working tree.
    Dirty,
    // An error occured.
    Error(Error),
}

impl Status {
    fn from_entry_status(status: &gix::dir::entry::Status) -> Self {
        match status {
            gix::dir::entry::Status::Pruned => Status::Error(error!("pruned")),
            gix::dir::entry::Status::Tracked => Status::Current,
            gix::dir::entry::Status::Ignored(_) => Status::Ignored,
            gix::dir::entry::Status::Untracked => Status::Dirty,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Error {
    inner: ArcStr,
}

macro_rules! error {
    ($lit:literal) => {
        Error { inner: arcstr::literal!($lit) }
    };
}

use error;

impl Error {
    fn new(str: impl ToString) -> Self {
        Self { inner: str.to_string().into() }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner.as_str(), f)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Display::fmt(&self.inner.as_str(), f)
    }
}

impl std::error::Error for Error {}

type Result<T = ()> = std::result::Result<T, Error>;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum StatusKind {
    Orphan,
    Ignored,
    Current,
    Staged,
    Dirty,
    Error,
}

impl Status {
    fn kind(&self) -> StatusKind {
        match self {
            Status::Orphan => StatusKind::Orphan,
            Status::Current => StatusKind::Current,
            Status::Staged => StatusKind::Staged,
            Status::Dirty => StatusKind::Dirty,
            Status::Ignored => StatusKind::Ignored,
            Status::Error(_) => StatusKind::Error,
        }
    }
}

fn merge(dst: &mut Option<Status>, new_status: Status) {
    match dst {
        Some(old_status) => {
            if old_status.kind() < new_status.kind() {
                *old_status = new_status;
            }
        }
        None => {
            *dst = Some(new_status);
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Current => f.write_str("current"),
            Status::Orphan => f.write_str("orphan"),
            Status::Staged => f.write_str("staged"),
            Status::Dirty => f.write_str("dirty"),
            Status::Ignored => f.write_str("ignored"),
            Status::Error(err) => f.write_fmt(format_args!("error: {err}")),
        }
    }
}

#[test]
fn example() {
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
