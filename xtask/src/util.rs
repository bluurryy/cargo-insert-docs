use std::{
    borrow::Cow,
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::OnceLock,
};

use color_eyre::eyre::{Context, OptionExt, bail};

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

pub(crate) enum Arg<'a> {
    #[expect(dead_code)]
    Verbatim(&'a str),
    WhitespaceSeparate(&'a str),
}

pub trait IntoArg<'a> {
    fn into_arg(self) -> Arg<'a>;
}

impl<'a> IntoArg<'a> for &'a str {
    fn into_arg(self) -> Arg<'a> {
        Arg::WhitespaceSeparate(self)
    }
}

impl<'a> IntoArg<'a> for Arg<'a> {
    fn into_arg(self) -> Arg<'a> {
        self
    }
}

enum OutKind {
    Ignore,
    Capture,
    Inherit,
}

impl OutKind {
    fn io(self) -> Stdio {
        match self {
            OutKind::Ignore => Stdio::null(),
            OutKind::Capture => Stdio::piped(),
            OutKind::Inherit => Stdio::inherit(),
        }
    }
}

pub struct Cmd {
    args: Vec<String>,
    unchecked: bool,
    stdout: OutKind,
    stderr: OutKind,
    #[expect(clippy::type_complexity)]
    hooks: Vec<Box<dyn FnOnce(&mut Command)>>,
}

impl Cmd {
    fn new(args: Vec<String>) -> Self {
        Self {
            args,
            unchecked: false,
            stdout: OutKind::Inherit,
            stderr: OutKind::Inherit,
            hooks: Vec::new(),
        }
    }

    pub fn unchecked(mut self) -> Self {
        self.unchecked = true;
        self
    }

    #[expect(dead_code)]
    pub fn ignore_stdout(mut self) -> Self {
        self.stdout = OutKind::Ignore;
        self
    }

    pub fn ignore_stderr(mut self) -> Self {
        self.stderr = OutKind::Ignore;
        self
    }

    pub fn capture_stdout(mut self) -> Self {
        self.stdout = OutKind::Capture;
        self
    }

    pub fn capture_stderr(mut self) -> Self {
        self.stderr = OutKind::Capture;
        self
    }

    pub fn capture(mut self) -> Self {
        self.stdout = OutKind::Capture;
        self.stderr = OutKind::Capture;
        self
    }

    #[expect(dead_code)]
    pub fn hook(mut self, f: impl FnOnce(&mut Command) + 'static) -> Self {
        self.hooks.push(Box::new(f));
        self
    }

    pub fn stdout(self) -> Result<String> {
        Ok(String::from_utf8(self.capture_stdout().output()?.stdout)?)
    }

    pub fn stderr(self) -> Result<String> {
        Ok(String::from_utf8(self.capture_stderr().output()?.stderr)?)
    }

    pub fn output(self) -> Result<Output> {
        let Self { args, unchecked, stdout, stderr, hooks } = self;

        let mut cmd = Command::new(&args[0]);
        cmd.args(&args[1..]);
        cmd.current_dir(WORKSPACE_DIR.get().unwrap());
        cmd.stdout(stdout.io());
        cmd.stderr(stderr.io());

        for hook in hooks {
            hook(&mut cmd);
        }

        let output = cmd.output()?;

        if !unchecked && !output.status.success() {
            let command = args
                .iter()
                .map(|arg| {
                    if arg.contains(char::is_whitespace) {
                        Cow::Owned(format!("{arg:?}"))
                    } else {
                        Cow::Borrowed(arg.as_str())
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");

            bail!(
                "command did not succeed!\n\
                command: {command}"
            )
        }

        Ok(output)
    }
}

pub(crate) fn cmd_new(args: &[Arg]) -> Cmd {
    let mut real_args = vec![];

    for arg in args {
        match *arg {
            Arg::Verbatim(s) => real_args.push(s),
            Arg::WhitespaceSeparate(s) => real_args.extend(s.split_whitespace()),
        }
    }

    Cmd::new(real_args.into_iter().map(|s| s.to_string()).collect())
}

macro_rules! cmd {
    ($($args:expr),* $(,)?) => {{
        #[allow(unused_imports)]
        use $crate::util::Arg::Verbatim;

        $crate::util::cmd_new(&[
            $(
                $crate::util::IntoArg::into_arg($args),
            )*
        ])
    }};
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
