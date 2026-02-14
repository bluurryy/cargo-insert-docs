use std::{
    borrow::Cow,
    env, fs,
    io::{self, BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    sync::OnceLock,
    thread::{self, JoinHandle},
};

use anstream::adapter::strip_str;
use color_eyre::eyre::{Context, OptionExt, bail};

pub use anstream::{eprintln, println};

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

#[derive(Clone, Copy, PartialEq, Eq)]
struct Out {
    inherit: bool,
    capture: bool,
}

impl Out {
    const ONLY_INHERIT: Self = Self { capture: false, inherit: true };
    const ONLY_CAPTURE: Self = Self { capture: true, inherit: false };
    const BOTH: Self = Self { capture: true, inherit: true };

    fn io(self) -> Stdio {
        if self.capture {
            return Stdio::piped();
        }

        if self.inherit {
            return Stdio::inherit();
        }

        Stdio::null()
    }
}

#[must_use]
pub struct Cmd {
    args: Vec<String>,
    unchecked: bool,
    stdout: Out,
    stderr: Out,
    #[expect(clippy::type_complexity)]
    hooks: Vec<Box<dyn FnOnce(&mut Command)>>,
}

impl Cmd {
    fn new(args: Vec<String>) -> Self {
        Self {
            args,
            unchecked: false,
            stdout: Out::ONLY_INHERIT,
            stderr: Out::ONLY_INHERIT,
            hooks: Vec::new(),
        }
    }

    pub fn unchecked(mut self) -> Self {
        self.unchecked = true;
        self
    }

    #[expect(dead_code)]
    pub fn ignore_stdout(mut self) -> Self {
        self.stdout.inherit = false;
        self
    }

    pub fn ignore_stderr(mut self) -> Self {
        self.stderr.inherit = false;
        self
    }

    pub fn capture_stdout(mut self) -> Self {
        if self.stdout != Out::BOTH {
            self.stdout = Out::ONLY_CAPTURE;
        }

        self
    }

    pub fn capture_stderr(mut self) -> Self {
        if self.stderr != Out::BOTH {
            self.stderr = Out::ONLY_CAPTURE;
        }

        self
    }

    pub fn inherit_and_capture(mut self) -> Self {
        self.stdout = Out::BOTH;
        self.stderr = Out::BOTH;
        self
    }

    #[expect(dead_code)]
    pub fn capture(self) -> Self {
        self.capture_stdout().capture_stderr()
    }

    #[expect(dead_code)]
    pub fn hook(mut self, f: impl FnOnce(&mut Command) + 'static) -> Self {
        self.hooks.push(Box::new(f));
        self
    }

    pub fn stdout(self) -> Result<String> {
        Ok(self.capture_stdout().output()?.stdout)
    }

    pub fn stderr(self) -> Result<String> {
        Ok(self.capture_stderr().output()?.stderr)
    }

    #[expect(dead_code)]
    pub fn status(self) -> Result<ExitStatus> {
        Ok(self.output()?.status)
    }

    pub fn run(self) -> Result {
        self.output()?;
        OK
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

        if stdout == Out::BOTH || stderr == Out::BOTH {
            let mut child = cmd.spawn()?;

            fn forward(
                kind: Out,
                stream: impl Read + Send + 'static,
                mut sink: impl Write + Send + 'static,
            ) -> JoinHandle<String> {
                thread::spawn(move || {
                    let reader = BufReader::new(stream);
                    let mut captured = String::new();

                    for line in reader.lines().map_while(Result::ok) {
                        if kind.inherit {
                            writeln!(&mut sink, "{line}").unwrap();
                        }

                        if kind.capture {
                            captured.push_str(&line);
                            captured.push('\n');
                        }
                    }

                    captured
                })
            }

            let stdout_thread = forward(stdout, child.stdout.take().unwrap(), io::stdout());
            let stderr_thread = forward(stderr, child.stderr.take().unwrap(), io::stderr());

            let status = child.wait()?;
            check_status(unchecked, &args, status)?;

            let stdout = stdout_thread.join().unwrap();
            let stderr = stderr_thread.join().unwrap();

            Ok(Output { status, stdout, stderr })
        } else if stdout.capture || stderr.capture {
            let output = cmd.output()?;
            check_status(unchecked, &args, output.status)?;
            Ok(Output {
                status: output.status,
                stdout: String::from_utf8(output.stdout)?,
                stderr: String::from_utf8(output.stderr)?,
            })
        } else {
            let status = cmd.status()?;
            check_status(unchecked, &args, status)?;
            Ok(Output { status, stdout: String::new(), stderr: String::new() })
        }
    }
}

fn check_status(unchecked: bool, args: &[String], status: ExitStatus) -> Result {
    if !unchecked && !status.success() {
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

    OK
}

#[derive(Debug, Clone)]
pub struct Output {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
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

pub trait AnsiStripExt {
    fn strip_ansi(self) -> String;
}

impl AnsiStripExt for String {
    fn strip_ansi(self) -> String {
        strip_str(&self).to_string()
    }
}
