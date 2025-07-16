// NOTE: expect_test does not work here because it unindents its own string and we can't disable that

use std::{
    any::Any,
    io::{self, Write},
};

use color_eyre::eyre::{self, Report, bail, eyre};
use expect_test::expect;
use tracing::{
    Level, debug, error, info, info_span, level_filters::LevelFilter, trace, trace_span, warn,
};
use tracing_error::ErrorLayer;
use tracing_subscriber::{
    EnvFilter, Layer as _, layer::SubscriberExt as _, util::SubscriberInitExt,
};
use unindent::unindent;

use super::{PrettyLog, Tally, WithErrorSeverity as _, pretty_eyre};

fn prepend_newline(str: &str) -> String {
    format!("\n{str}")
}

fn strip_ansi(str: &str) -> String {
    let mut stream = anstream::StripStream::new(Vec::new());
    stream.write_all(str.as_bytes()).unwrap();
    String::from_utf8(stream.into_inner()).unwrap()
}

fn prepare_for_compare(str: &str) -> String {
    unindent(&prepend_newline(&strip_ansi(str)))
}

fn with_log(pretty_filter: &str, rustlog_filter: &str, f: impl FnOnce(PrettyLog)) -> String {
    if let Ok((panic_hook, eyre_hook)) = color_eyre::config::HookBuilder::default()
        .capture_span_trace_by_default(true)
        .try_into_hooks()
    {
        panic_hook.install();
        _ = eyre::set_hook(pretty_eyre::wrap_hook(eyre_hook.into_eyre_hook()));
    }

    let log = PrettyLog::new(Box::new(Vec::<u8>::new()));

    let guard = tracing_subscriber::registry()
        .with(ErrorLayer::default().boxed())
        .with(log.clone().with_filter(EnvFilter::new(pretty_filter)).boxed())
        .with(
            tracing_subscriber::fmt::layer()
                .without_time()
                .with_writer({
                    let log = log.clone();
                    move || log.clone()
                })
                .with_filter(
                    EnvFilter::builder()
                        .with_default_directive(LevelFilter::OFF.into())
                        .parse_lossy(rustlog_filter),
                )
                .boxed(),
        )
        .set_default();

    f(log.clone());

    drop(guard);

    let sink: Box<dyn Any> = log.replace_sink(Box::new(io::empty()));
    let sink: Box<Vec<u8>> = sink.downcast().unwrap();
    String::from_utf8(*sink).unwrap()
}

fn simple_log(f: impl FnOnce(PrettyLog)) -> String {
    with_log("info", "", f)
}

#[test]
fn test_event() {
    let out = simple_log(|log| {
        trace!("i'm a trace");
        debug!("i'm a debug");
        info!("i'm an info");
        warn!("i'm a warning");
        error!("i'm an error");

        assert_eq!(log.tally(), Tally { warnings: 1, errors: 1 })
    });

    expect![[r#"
           info: i'm an info

        warning: i'm a warning

          error: i'm an error
    "#]]
    .assert_eq(&prepare_for_compare(&out));
}

#[test]
fn test_event_spanned() {
    let out = simple_log(|log| {
        let _span = info_span!("pets", cat = "cute", dog = "too").entered();
        info!("i'm an info");
        assert_eq!(log.tally(), Tally { warnings: 0, errors: 0 })
    });

    expect![[r#"
        info: i'm an info
        span: pets
         cat: cute
         dog: too
    "#]]
    .assert_eq(&prepare_for_compare(&out));
}

#[test]
fn test_event_spanned_empty_name() {
    let out = simple_log(|log| {
        let _span = info_span!("", cat = "cute", dog = "too").entered();
        info!("i'm an info");
        assert_eq!(log.tally(), Tally { warnings: 0, errors: 0 })
    });

    expect![[r#"
        info: i'm an info
         cat: cute
         dog: too
    "#]]
    .assert_eq(&prepare_for_compare(&out));
}

#[test]
#[ignore = "needs to be run separately because of hooks"]
fn test_report() {
    let out = simple_log(|log| {
        log.print_report(&eyre!("coffee machine broke"));
    });

    expect![[r#"
        error: coffee machine broke
    "#]]
    .assert_eq(&prepare_for_compare(&out));
}

#[test]
#[ignore = "needs to be run separately because of hooks"]
fn test_report_spanned() {
    let out = simple_log(|log| {
        let _span = info_span!("", coffee = "missing", machine = "broken").entered();
        log.print_report(
            &eyre!("coffee machine broke").wrap_err("did not drink coffee").wrap_err("i'm tired"),
        );
    });

    expect![[r#"
          error: i'm tired
          cause: did not drink coffee
          cause: coffee machine broke
         coffee: missing
        machine: broken
    "#]]
    .assert_eq(&prepare_for_compare(&out));
}

#[test]
#[ignore = "needs to be run separately because of hooks"]
fn test_report_spanned_with_severity() {
    let out = simple_log(|log| {
        let _span = info_span!("", coffee = "missing", machine = "broken").entered();
        log.print_report(
            &eyre!("coffee machine broke")
                .with_severity(Level::WARN)
                .wrap_err("did not drink coffee")
                .wrap_err("i'm tired"),
        );
        assert_eq!(log.tally(), Tally { warnings: 1, errors: 0 });
    });

    expect![[r#"
        warning: i'm tired
          cause: did not drink coffee
          cause: coffee machine broke
         coffee: missing
        machine: broken
    "#]]
    .assert_eq(&prepare_for_compare(&out));
}

#[test]
#[ignore = "needs to be run separately because of hooks"]
fn test_result_spanned() {
    fn something() -> Result<(), Report> {
        let _span = info_span!("", coffee = "missing", machine = "broken").entered();
        bail!("coffee machine broke")
    }

    let out = simple_log(|log| {
        log.print_report(&something().unwrap_err());
    });

    expect![[r#"
          error: coffee machine broke
         coffee: missing
        machine: broken
    "#]]
    .assert_eq(&prepare_for_compare(&out));
}

#[test]
#[ignore = "needs to be run separately because of hooks"]
fn test_regular_logs_between_pretty() {
    let out = with_log("info", "trace", |log| {
        let _span = trace_span!("trace span").entered();
        let _span = info_span!("info span").entered();

        trace!("trace event");
        info!("info event");
        trace!("trace event 2");
        log.print_report(&eyre!("error report"));
        trace!("trace event 3");
    });

    expect![[r#"
        TRACE trace span:info span: cargo_insert_docs::pretty_log::tests: trace event

                  info: info event
                  span: info span

         INFO trace span:info span: cargo_insert_docs::pretty_log::tests: info event
        TRACE trace span:info span: cargo_insert_docs::pretty_log::tests: trace event 2

                 error: error report
                  span: info span

        TRACE trace span:info span: cargo_insert_docs::pretty_log::tests: trace event 3
    "#]]
    .assert_eq(&prepare_for_compare(&out));
}
