use std::path::Path;
use std::path::PathBuf;

use color_eyre::eyre::Context as _;
use color_eyre::eyre::OptionExt as _;
use color_eyre::eyre::Result;
use tracing::error_span;

/// Better error reporting version of [`std::fs::read_to_string`].
pub fn read_to_string(path: &Path) -> Result<String> {
    let _span = error_span!("", path = %path.display()).entered();

    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| path.display().to_string());

    std::fs::read_to_string(path).with_context(|| format!("failed to read {file_name}"))
}

/// Better error reporting version of [`std::fs::write`].
pub fn write(path: &Path, content: &[u8]) -> Result<()> {
    let _span = error_span!("", path = %path.display()).entered();

    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| path.display().to_string());

    std::fs::write(path, content).with_context(|| format!("failed to write to {file_name}"))
}

// for better error messages when reading / writing files
#[derive(Clone)]
pub struct RelativePath {
    pub full_path: PathBuf,
    pub relative: PathBuf,
}

impl RelativePath {
    pub fn from_origin_relative(origin: &Path, relative: &Path) -> Self {
        Self { full_path: origin.join(relative), relative: relative.into() }
    }

    pub fn relative_to_parent(path: &Path) -> Result<Self> {
        Ok(Self {
            relative: path.file_name().ok_or_eyre("path has no file name")?.into(),
            full_path: path.into(),
        })
    }

    pub fn read_to_string(&self) -> Result<String> {
        let _span = error_span!("", path = %self.full_path.display()).entered();

        std::fs::read_to_string(&self.full_path)
            .with_context(|| format!("failed to read {}", self.relative.display()))
    }

    pub fn write(&self, contents: &str) -> Result<()> {
        let _span = error_span!("", path = %self.full_path.display()).entered();

        std::fs::write(&self.full_path, contents)
            .with_context(|| format!("failed to write {}", self.relative.display()))
    }
}
