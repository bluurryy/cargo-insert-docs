// NOTE: expect_test does not work here because it unindents its own string and we can't disable that

use std::{any::Any, io::Write};

use color_eyre::eyre::{Report, eyre};
use expect_test::expect;
use unindent::unindent;

use super::ErrorSink;

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

fn format_report(report: &Report) -> String {
    with_log(|log| log.report(report))
}

fn with_log(f: impl FnOnce(&ErrorSink)) -> String {
    let log = ErrorSink::new(Box::new(Vec::<u8>::new()));
    f(&log);
    String::from_utf8(*(log.into_inner() as Box<dyn Any>).downcast().unwrap()).unwrap()
}

#[test]
fn test_report_bare() {
    let report =
        eyre!("coffee machine broke").wrap_err("did not drink coffee").wrap_err("i'm tired");

    expect![["
        error: i'm tired
        cause: did not drink coffee
        cause: coffee machine broke
    "]]
    .assert_eq(&prepare_for_compare(&format_report(&report)));
}

#[test]
fn test_report_formatted() {
    let log = ErrorSink::new(Box::new(Vec::<u8>::new()));

    let report = log
        .span("coffee", "missing")
        .span("machine", "broken")
        .warn("coffee machine broke")
        .into_report()
        .wrap_err("did not drink coffee")
        .wrap_err("i'm tired");

    expect![[r#"
        warning: i'm tired
          cause: did not drink coffee
          cause: coffee machine broke
        machine: broken
         coffee: missing
    "#]]
    .assert_eq(&prepare_for_compare(&format_report(&report)));
}

#[test]
fn test_message() {
    expect![[r#"
        warning: coffee machine broke
        machine: broken
         coffee: missing
    "#]]
    .assert_eq(&prepare_for_compare(&with_log(|log| {
        log.span("coffee", "missing").span("machine", "broken").warn("coffee machine broke");

        assert_eq!(log.tally().warnings, 1);
    })));
}

#[test]
fn test_report_duplicate() {
    expect![[r#"
          warning: warning message
        something: else
        duplicate: inner
        duplicate: outer
    "#]]
    .assert_eq(&prepare_for_compare(&with_log(|log| {
        let _span = log.span("duplicate", "outer");

        let report = log
            .span("duplicate", "inner")
            .span("something", "else")
            .warn("warning message")
            .into_report();

        log.report(&report);
    })));
}

#[test]
fn test_report_bare_span() {
    expect![[r#"
        error: oops
         some: context
    "#]]
    .assert_eq(&prepare_for_compare(&with_log(|log| {
        let _span = log.span("some", "context");
        let report = eyre!("oops");
        log.report(&report);
    })));
}
