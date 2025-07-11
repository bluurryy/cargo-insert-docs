#[cfg(test)]
mod tests;

use std::{
    any::Any,
    cell::RefCell,
    error::Error,
    fmt::{self, Write as _},
    io,
    ops::Deref,
};

use anstyle::Style;
use color_eyre::eyre::Report;

use crate::style::{BOLD, ERROR, WARNING};

pub trait AnyWrite: io::Write + Any {}

impl<T: io::Write + Any> AnyWrite for T {}

pub struct ErrorSink {
    inner: RefCell<ErrorSinkInner>,
}

impl ErrorSink {
    pub fn new(write: Box<dyn AnyWrite>) -> Self {
        Self {
            inner: RefCell::new(ErrorSinkInner {
                write,
                spans: Vec::new(),
                tally: Tally::default(),
                was_written_to: false,
            }),
        }
    }

    #[cfg(test)]
    pub fn into_inner(self) -> Box<dyn AnyWrite> {
        self.inner.into_inner().write
    }

    pub fn stderr() -> Self {
        ErrorSink::new(Box::new(anstream::stderr()))
    }

    pub fn info(&self, message: impl fmt::Display) -> Event<'_> {
        self.log(Level::Info, message)
    }

    pub fn warn(&self, message: impl fmt::Display) -> Event<'_> {
        self.log(Level::Warning, message)
    }

    #[expect(dead_code)]
    pub fn error(&self, message: impl fmt::Display) -> Event<'_> {
        self.log(Level::Error, message)
    }

    pub fn log(&self, level: Level, message: impl fmt::Display) -> Event<'_> {
        let message = message.to_string();
        Event { error_sink: self, level_and_message: Some(LevelAndMessage { level, message }) }
    }

    pub fn report(&self, report: &Report) {
        self.inner.borrow_mut().report(report);
    }

    pub fn empty_span(&self) -> ErrorSinkSpan<'_> {
        let index;

        {
            let mut inner = self.inner.borrow_mut();
            index = inner.spans.len();
            inner.spans.push(vec![]);
        }

        ErrorSinkSpan { error_sink: self, index }
    }

    pub fn span(&self, name: impl fmt::Display, value: impl fmt::Display) -> ErrorSinkSpan<'_> {
        self.empty_span().span(name, value)
    }

    pub fn tally(&self) -> Tally {
        self.inner.borrow().tally
    }

    pub fn write(&self, value: impl fmt::Display) {
        self.inner.borrow_mut().write_fmt(format_args!("{value}"));
    }

    pub fn was_written_to(&self) -> bool {
        self.inner.borrow().was_written_to
    }

    pub fn set_written_to(&self, value: bool) {
        self.inner.borrow_mut().was_written_to = value;
    }

    pub fn print_tally(&self) {
        let Tally { warnings, errors, .. } = self.tally();

        let mut out = String::new();

        if errors != 0 || warnings != 0 {
            out.push('\n');
        }

        if errors != 0 {
            let s = if errors == 1 { "" } else { "s" };

            out.write_fmt(format_args!("{BOLD}{errors} {BOLD:#}{ERROR}error{s}{ERROR:#}")).unwrap();
        }

        if warnings != 0 {
            if errors != 0 {
                out.write_fmt(format_args!("{BOLD}, {BOLD:#}")).unwrap();
            }

            let s = if warnings == 1 { "" } else { "s" };

            out.write_fmt(format_args!("{BOLD}{warnings} {BOLD:#}{WARNING}warning{s}{WARNING:#}"))
                .unwrap();
        }

        if errors != 0 || warnings != 0 {
            out.push('\n');
        }

        self.write(out);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warning,
    Error,
}

impl Level {
    fn style(self) -> &'static Style {
        use crate::style::{ERROR, INFO, WARNING};

        match self {
            Level::Info => &INFO,
            Level::Warning => &WARNING,
            Level::Error => &ERROR,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Level::Info => "info",
            Level::Warning => "warning",
            Level::Error => "error",
        }
    }
}

pub struct Event<'a> {
    error_sink: &'a ErrorSink,
    level_and_message: Option<LevelAndMessage>,
}

#[derive(Debug)]
struct LevelAndMessage {
    level: Level,
    message: String,
}

impl Event<'_> {
    pub fn into_report(mut self) -> Report {
        let LevelAndMessage { level, message } = self.level_and_message.take().unwrap();

        Report::from(ReportFields {
            level,
            message,
            fields: self
                .error_sink
                .inner
                .borrow()
                .fields()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect(),
        })
    }

    pub fn into_report_err(self) -> Result<(), Report> {
        Err(self.into_report())
    }
}

impl Drop for Event<'_> {
    fn drop(&mut self) {
        if let Some(level_and_message) = self.level_and_message.as_ref() {
            self.error_sink
                .inner
                .borrow_mut()
                .message(level_and_message.level, &level_and_message.message);
        }
    }
}

pub struct ErrorSinkSpan<'a> {
    error_sink: &'a ErrorSink,
    index: usize,
}

impl Deref for ErrorSinkSpan<'_> {
    type Target = ErrorSink;

    fn deref(&self) -> &Self::Target {
        self.error_sink
    }
}

impl ErrorSinkSpan<'_> {
    // adds another field to this span
    pub fn span(mut self, name: impl fmt::Display, value: impl fmt::Display) -> Self {
        self.add(name, value);
        self
    }

    pub fn add(&mut self, name: impl fmt::Display, value: impl fmt::Display) {
        self.error_sink.inner.borrow_mut().spans[self.index]
            .push((name.to_string(), value.to_string()));
    }
}

impl Drop for ErrorSinkSpan<'_> {
    fn drop(&mut self) {
        self.error_sink.inner.borrow_mut().spans.pop();
    }
}

struct ErrorSinkInner {
    write: Box<dyn AnyWrite>,
    spans: Vec<Vec<(String, String)>>,
    tally: Tally,
    was_written_to: bool,
}

impl ErrorSinkInner {
    fn write_fmt(&mut self, args: fmt::Arguments) {
        self.was_written_to = true;
        _ = self.write.write_fmt(args);
    }

    fn write_str(&mut self, string: &str) {
        self.was_written_to = true;
        _ = self.write.write_all(string.as_bytes());
    }

    fn prepare_write(&self, out: &mut String) {
        if self.was_written_to {
            out.push('\n');
        }
    }

    fn message(&mut self, level: Level, message: &str) {
        let mut out = String::new();

        self.prepare_write(&mut out);

        *self.tally.get_mut(level) += 1;

        out.push_str(&format_level(level));
        out.push_str(&format_message(message));

        for (name, value) in self.fields() {
            out.push_str(&format_field(name, value));
        }

        self.write_str(&out);
    }

    fn report(&mut self, report: &Report) {
        let mut out = String::new();

        self.prepare_write(&mut out);

        let level;
        let has_report_fields;

        if let Some(report_fields) = report.downcast_ref::<ReportFields>() {
            level = report_fields.level;
            has_report_fields = true;
        } else {
            level = Level::Error;
            has_report_fields = false;
        }

        *self.tally.get_mut(level) += 1;

        let mut fields = vec![];

        out.push_str(&format_level(level));

        let mut first = true;

        for cause in report.chain() {
            if first {
                first = false;
            } else {
                out.push_str(&format_property_key("cause", BOLD));
            }

            if let Some(report) = cause.downcast_ref::<ReportFields>() {
                out.push_str(&format_message(&report.message));

                for (name, value) in &report.fields {
                    fields.push((name.as_str(), value.as_str()));
                }
            } else {
                let message = cause.to_string();
                let message = if message.is_empty() { "<empty message>" } else { message.as_str() };

                for (i, line) in message.lines().enumerate() {
                    if i != 0 {
                        out.push_str(INDENT);
                    }

                    out.push_str(line);
                    out.push('\n');
                }
            }
        }

        for (name, value) in &fields {
            out.push_str(&format_field(name, value));
        }

        if !has_report_fields {
            for (name, value) in self.fields() {
                out.push_str(&format_field(name, value));
            }
        }

        self.write_str(&out);
    }

    fn fields(&self) -> impl Iterator<Item = (&str, &str)> {
        self.spans.iter().rev().flat_map(|fields| {
            fields.iter().rev().map(|(name, value)| (name.as_str(), value.as_str()))
        })
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Tally {
    pub infos: usize,
    pub warnings: usize,
    pub errors: usize,
}

impl Tally {
    fn get_mut(&mut self, level: Level) -> &mut usize {
        match level {
            Level::Info => &mut self.infos,
            Level::Warning => &mut self.warnings,
            Level::Error => &mut self.errors,
        }
    }
}

const LEVEL_WIDTH: usize = 14;
const INDENT_BYTES: &[u8] = &[b' '; LEVEL_WIDTH + 2];
const INDENT: &str = match str::from_utf8(INDENT_BYTES) {
    Ok(ok) => ok,
    Err(_) => unreachable!(),
};

fn format_level(level: Level) -> String {
    format_property_key(level.name(), *level.style())
}

fn format_message(message: &str) -> String {
    let mut out = String::new();

    for (i, line) in message.lines().enumerate() {
        if i != 0 {
            out.push_str(INDENT);
        }

        out.push_str(line);
        out.push('\n');
    }

    out
}

fn format_field(name: &str, value: &str) -> String {
    let mut out = String::new();
    out.push_str(&format_property_key(name, BOLD));
    out.push_str(&format_message(value));
    out
}

fn format_property_key(name: &str, style: Style) -> String {
    format!("{style}{name:>LEVEL_WIDTH$}{style:#}{BOLD}:{BOLD:#} ")
}

#[derive(Debug)]
pub struct ReportFields {
    level: Level,
    message: String,
    fields: Vec<(String, String)>,
}

impl fmt::Display for ReportFields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format_message(&self.message))?;

        for (name, value) in &self.fields {
            f.write_str(&format_field(name, value))?;
        }

        Ok(())
    }
}

impl Error for ReportFields {}
