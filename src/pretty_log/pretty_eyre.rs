//! We use our own span trace because our span formatting does not play well
//! with `Display` impls or `color-spantrace`.
//!
//! There can only be one formatter for `SpanTrace` and we can't get it's internal
//! span so we have to do this ourselves.

use core::fmt;
use std::{error::Error, panic};

use color_eyre::eyre::{EyreHandler, Report};
use tracing::{Level, Span};

pub fn hook(base_hook: HookFunc) -> HookFunc {
    Box::new(move |e| {
        Box::new(PrettyHandler { base: base_hook(e), level: Level::ERROR, span: Span::current() })
    })
}

pub fn extract_span(report: &Report) -> Option<&Span> {
    report.handler().downcast_ref::<PrettyHandler>().map(|h| &h.span)
}

pub fn extract_severity(report: &Report) -> Level {
    report.handler().downcast_ref::<PrettyHandler>().map(|h| h.level).unwrap_or(Level::ERROR)
}

pub fn set_severity(report: &mut Report, level: Level) {
    if let Some(handler) = report.handler_mut().downcast_mut::<PrettyHandler>() {
        handler.level = level;
    }
}

struct PrettyHandler {
    base: Box<dyn EyreHandler>,
    span: Span,
    level: Level,
}

impl EyreHandler for PrettyHandler {
    fn debug(&self, error: &(dyn Error + 'static), f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.base.debug(error, f)
    }

    fn display(&self, error: &(dyn Error + 'static), f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.base.display(error, f)
    }

    fn track_caller(&mut self, location: &'static panic::Location<'static>) {
        self.base.track_caller(location);
    }
}

type HookFunc = Box<dyn Fn(&(dyn Error + 'static)) -> Box<dyn EyreHandler> + Send + Sync + 'static>;
