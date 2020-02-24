mod support;
use support::*;

use crate::support::field::mock;
use crate::support::span::NewSpan;
use tracing::subscriber::with_default;
use tracing_attributes::instrument;

#[instrument(fields(foo = "bar", dsa = true, num = 1))]
fn fn_no_param() {}

#[instrument(fields(foo = "bar"))]
fn fn_param(param: u32) {}

#[instrument(fields(foo = "bar", empty))]
fn fn_empty_field() {}

#[test]
fn fields() {
    let span = span::mock().with_field(
        mock("foo")
            .with_value(&"bar")
            .and(mock("dsa").with_value(&true))
            .and(mock("num").with_value(&1))
            .only(),
    );
    run_test(span, || {
        fn_no_param();
    });
}

#[test]
fn parameters_with_fields() {
    let span = span::mock().with_field(
        mock("foo")
            .with_value(&"bar")
            .and(mock("param").with_value(&format_args!("1")))
            .only(),
    );
    run_test(span, || {
        fn_param(1);
    });
}

#[test]
fn empty_field() {
    let span = span::mock().with_field(mock("foo").with_value(&"bar").only());
    run_test(span, || {
        fn_empty_field();
    });
}

fn run_test<F: FnOnce() -> T, T>(span: NewSpan, fun: F) {
    let (subscriber, handle) = subscriber::mock()
        .new_span(span)
        .enter(span::mock())
        .exit(span::mock())
        .done()
        .run_with_handle();

    with_default(subscriber, fun);
    handle.assert_finished();
}
