//! A logging setup which prints pretty [`tracing`] events
//! and can print [`eyre::Report`] with spans and a set severity [`tracing::Level`]
//! using a custom [`eyre`] hook.
//!
//! Its [`install`](PrettyLog::install) method:
//! - Adds an [`ErrorLayer`] for span traces used by [`color_eyre`].
//!   Our own [`print_report`](PrettyLog::print_report) does not make use of them but records its own
//!   span traces, see [`pretty_eyre`].
//! - Adds a [`mod@tracing_subscriber::fmt`] layer with an env filter for regular `RUST_LOG` tracing
//!   messages. Those won't be shown unless the `RUST_LOG` env var is used.
//! - Adds our own [`PrettyLog`] as a layer with a filter so only our own crate's message are pretty
//!   printed.

mod pretty_eyre;
#[cfg(test)]
pub(crate) mod tests;
mod visit_str;

use std::{
    any::Any,
    fmt::Write as _,
    io, mem,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use anstyle::{AnsiColor, Color, Effects, Style};
use color_eyre::eyre::{self, Report};
use tracing::{
    Event, Level, Subscriber,
    field::{Field, Visit},
    level_filters::LevelFilter,
    span::{Attributes, Id},
};
use tracing_error::ErrorLayer;
use tracing_subscriber::{
    EnvFilter, Layer, Registry,
    layer::{Context, SubscriberExt as _},
    registry::LookupSpan,
};

use crate::pretty_log::visit_str::{VisitAsStr, VisitStr};

pub trait AnyWrite: Any + io::Write + Send {}

impl<T: Any + io::Write + Send> AnyWrite for T {}

trait Mutx {
    type Out;

    fn lck(&self) -> MutexGuard<'_, Self::Out>;
}

impl<T> Mutx for Mutex<T> {
    type Out = T;

    fn lck(&self) -> MutexGuard<'_, Self::Out> {
        self.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[derive(Clone)]
pub struct PrettyLog {
    inner: Arc<Mutex<PrettyLogInner>>,
}

impl PrettyLog {
    pub fn new(sink: Box<dyn AnyWrite>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(PrettyLogInner {
                sink,
                tally: Default::default(),
                last_print_kind: None,
            })),
        }
    }

    pub fn subscriber(&self, filter: &str) -> impl Subscriber + Send + Sync + 'static {
        tracing_subscriber::registry()
            .with(ErrorLayer::default())
            .with(self.clone().with_filter(EnvFilter::new(filter)).boxed())
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer({
                        let log = self.clone();
                        move || log.clone()
                    })
                    .with_filter(
                        EnvFilter::builder()
                            .with_default_directive(LevelFilter::OFF.into())
                            .from_env_lossy(),
                    )
                    .boxed(),
            )
    }

    pub fn install(&self, filter: &str) {
        let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default()
            .capture_span_trace_by_default(true)
            .into_hooks();

        panic_hook.install();

        eyre::set_hook(pretty_eyre::wrap_hook(eyre_hook.into_eyre_hook()))
            .expect("eyre hook already set");

        tracing::subscriber::set_global_default(self.subscriber(filter))
            .expect("tracing subscriber already set");
    }

    pub fn tally(&self) -> Tally {
        self.inner.lck().tally
    }

    fn print_formatted_event(&self, level: Level, message: &str) {
        self.inner.lck().print_event(level, message);
    }

    pub fn print_report(&self, report: &Report) {
        self.inner.lck().print_report(report);
    }

    pub fn print_tally(&self) {
        self.inner.lck().print_tally();
    }

    #[cfg_attr(not(test), expect(dead_code))]
    pub fn replace_sink(&self, new_sink: Box<dyn AnyWrite>) -> Box<dyn AnyWrite> {
        mem::replace(&mut self.inner.lck().sink, new_sink)
    }

    pub fn foreign_write_incoming(&self) {
        let mut inner = self.inner.lck();
        let out = inner.begin_print(PrintKind::Foreign);
        _ = inner.sink.write_all(out.as_bytes());
    }
}

impl io::Write for PrettyLog {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.lck().write_direct(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq)]
enum PrintKind {
    Pretty,
    Direct,
    Foreign,
}

impl PrintKind {
    fn always_wants_separator(self) -> bool {
        use PrintKind::*;
        matches!(self, Pretty | Foreign)
    }
}

struct PrettyLogInner {
    sink: Box<dyn AnyWrite>,
    tally: Tally,
    last_print_kind: Option<PrintKind>,
}

impl PrettyLogInner {
    fn begin_print(&mut self, print_kind: PrintKind) -> String {
        let mut out = String::new();

        if let Some(last_print_kind) = self.last_print_kind
            && (print_kind.always_wants_separator() || last_print_kind != print_kind)
        {
            out.push('\n');
        }

        self.last_print_kind = Some(print_kind);

        out
    }

    fn write_direct(&mut self, string: &[u8]) {
        let mut out = self.begin_print(PrintKind::Direct).into_bytes();
        out.extend_from_slice(string);
        _ = self.sink.write_all(&out);
    }

    fn print_event(&mut self, level: Level, message: &str) {
        let mut out = self.begin_print(PrintKind::Pretty);
        self.tally.inc(level);
        out.push_str(message);
        _ = self.sink.write_all(out.as_bytes());
    }

    fn print_report(&mut self, report: &Report) {
        let mut out = self.begin_print(PrintKind::Pretty);
        let level = pretty_eyre::extract_severity(report);
        self.tally.inc(level);

        let mut errors = report.chain();

        format_level(&mut out, level);
        format_field_value(&mut out, &errors.next().unwrap().to_string());

        for error in errors {
            format_field(&mut out, "cause", &error.to_string());
        }

        if let Some(span) = pretty_eyre::extract_span(report) {
            span.with_subscriber(|(id, sub)| {
                if let Some(reg) = sub.downcast_ref::<Registry>() {
                    let span =
                        reg.span(id).expect("registry should have a span for the current ID");

                    for span in span.scope() {
                        if let Some(FormattedField(fields)) = span.extensions().get() {
                            out.push_str(fields);
                        }
                    }
                }
            });
        }

        _ = self.sink.write_all(out.as_bytes());
    }

    fn print_tally(&mut self) {
        let Tally { warnings, errors, .. } = self.tally;

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

        _ = self.sink.write_all(out.as_bytes());
    }
}

struct FormattedField(String);

impl<S: Subscriber> Layer<S> for PrettyLog
where
    S: for<'lookup> LookupSpan<'lookup>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let mut fmt = PrettyFields::new();

        fmt.span(attrs.metadata().name());

        attrs.record(&mut fmt.visit());

        ctx.span(id).unwrap().extensions_mut().insert(FormattedField(fmt.out()));
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut fmt = PrettyEvent::new();
        let level = *event.metadata().level();
        event.record(&mut fmt.visit());
        let mut out = fmt.out(level);

        if let Some(scope) = ctx.event_scope(event) {
            for span in scope {
                if let Some(FormattedField(string)) = span.extensions().get() {
                    out.push_str(string);
                }
            }
        }

        self.print_formatted_event(level, &out);
    }
}

#[derive(Default)]
struct PrettyEvent {
    message: String,
    fields: String,
}

impl VisitStr for PrettyEvent {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            if !self.message.is_empty() {
                self.message.push_str(INDENT);
            }

            format_field_value(&mut self.message, value);
        } else {
            format_field(&mut self.fields, field.name(), value);
        }
    }
}

impl PrettyEvent {
    fn new() -> Self {
        Self::default()
    }

    fn visit(&mut self) -> impl Visit {
        VisitAsStr(self)
    }

    fn out(&self, level: Level) -> String {
        let mut out = String::new();
        format_level(&mut out, level);
        out.push_str(&self.message);
        out.push_str(&self.fields);
        out
    }
}

#[derive(Default)]
struct PrettyFields {
    fields: String,
}

impl VisitStr for PrettyFields {
    fn record_str(&mut self, field: &Field, value: &str) {
        format_field(&mut self.fields, field.name(), value);
    }
}

impl PrettyFields {
    fn new() -> Self {
        Self::default()
    }

    fn span(&mut self, name: &str) {
        if !name.is_empty() {
            format_field(&mut self.fields, "span", name);
        }
    }

    fn visit(&mut self) -> impl Visit {
        VisitAsStr(self)
    }

    fn out(self) -> String {
        self.fields
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Tally {
    pub warnings: usize,
    pub errors: usize,
}

impl Tally {
    fn inc(&mut self, level: Level) {
        let mut dummy = 0;

        *(match level {
            Level::WARN => &mut self.warnings,
            Level::ERROR => &mut self.errors,
            _ => &mut dummy,
        }) += 1;
    }
}

const fn label(color: AnsiColor) -> Style {
    Style::new().fg_color(Some(Color::Ansi(color))).effects(Effects::BOLD)
}

pub const TRACE: Style = label(AnsiColor::Magenta);
pub const DEBUG: Style = label(AnsiColor::Cyan);
pub const INFO: Style = label(AnsiColor::Green);
pub const WARNING: Style = label(AnsiColor::Yellow);
pub const ERROR: Style = label(AnsiColor::Red);

pub const BOLD: Style = Style::new().effects(Effects::BOLD);

const KEY_WIDTH: usize = 14;
const INDENT_BYTES: &[u8] = &[b' '; KEY_WIDTH + 2];
const INDENT: &str = match str::from_utf8(INDENT_BYTES) {
    Ok(ok) => ok,
    Err(_) => unreachable!(),
};

fn format_level(out: &mut String, level: Level) {
    let name = match level {
        Level::ERROR => "error",
        Level::WARN => "warning",
        Level::INFO => "info",
        Level::DEBUG => "debug",
        Level::TRACE => "trace",
    };

    let style = match level {
        Level::ERROR => ERROR,
        Level::WARN => WARNING,
        Level::INFO => INFO,
        Level::DEBUG => DEBUG,
        Level::TRACE => TRACE,
    };

    format_field_key(out, name, style)
}

fn format_field(out: &mut String, name: &str, value: &str) {
    format_field_key(out, name, BOLD);
    format_field_value(out, value);
}

fn format_field_key(out: &mut String, key: &str, style: Style) {
    let key_buf;
    let mut key = key;

    if key.contains('_') {
        key_buf = key.replace('_', "-");
        key = &key_buf;
    }

    out.write_fmt(format_args!("{style}{key:>KEY_WIDTH$}{style:#}{BOLD}:{BOLD:#} "))
        .expect("formatting to string can't fail");
}

fn format_field_value(out: &mut String, message: &str) {
    for (i, line) in message.lines().enumerate() {
        if i != 0 {
            out.push_str(INDENT);
        }

        out.push_str(line);
        out.push('\n');
    }
}

pub trait WithResultSeverity<T> {
    fn with_severity(self, level: Level) -> Result<T, Report>;
}

impl<T> WithResultSeverity<T> for Result<T, Report> {
    fn with_severity(self, level: Level) -> Result<T, Report> {
        match self {
            Ok(ok) => Ok(ok),
            Err(err) => Err(err.with_severity(level)),
        }
    }
}

pub trait WithErrorSeverity {
    fn with_severity(self, level: Level) -> Report;
}

impl WithErrorSeverity for Report {
    fn with_severity(mut self, level: Level) -> Report {
        pretty_eyre::set_severity(&mut self, level);
        self
    }
}
