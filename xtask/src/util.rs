use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use color_eyre::eyre::{Context, OptionExt};

pub type Error = color_eyre::eyre::Report;
pub type Result<T = (), E = Error> = std::result::Result<T, E>;
pub const OK: Result = Result::Ok(());

pub static WORKSPACE_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn init() -> Result {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = manifest_dir.parent().ok_or_eyre("manifest dir has no parent")?;
    WORKSPACE_DIR.set(workspace_dir.into()).unwrap();
    OK
}

pub fn relative_to_workspace(path: impl AsRef<Path>) -> PathBuf {
    WORKSPACE_DIR.get().unwrap().join(path)
}

pub fn read(relative_to_workspace_path: impl AsRef<Path>) -> Result<String> {
    let path = relative_to_workspace(relative_to_workspace_path);
    fs::read_to_string(&path).wrap_err_with(|| format!("failed to read from {}", path.display()))
}

pub fn write(relative_to_workspace_path: impl AsRef<Path>, content: &str) -> Result {
    let path = relative_to_workspace(relative_to_workspace_path);
    fs::write(&path, content).wrap_err_with(|| format!("failed to write to {}", path.display()))
}

macro_rules! cmd {
    ($program:expr $(, $arg:expr )* $(,)?) => {
        duct::cmd!($program $(, $arg)*).dir($crate::util::WORKSPACE_DIR.get().unwrap())
    };
}

macro_rules! re {
    ($lit:literal) => {{
        fn get() -> &'static fancy_regex::Regex {
            static REGEX: std::sync::OnceLock<fancy_regex::Regex> = std::sync::OnceLock::new();
            REGEX.get_or_init(|| fancy_regex::Regex::new($lit).unwrap())
        }
        get()
    }};
}

pub(crate) use cmd;
pub(crate) use re;
