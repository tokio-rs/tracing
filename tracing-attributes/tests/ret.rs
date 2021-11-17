#[path = "../../tracing-futures/tests/support.rs"]
// we don't use some of the test support functions, but `tracing-futures` does.
#[allow(dead_code)]
mod support;
use support::*;

use std::convert::TryFrom;
use std::num::TryFromIntError;

use tracing::{collect::with_default, Level};
use tracing_attributes::instrument;

#[instrument(ret)]
fn ret() -> i32 {
    42
}

#[test]
fn test() {
    let span = span::mock().named("ret");
    let (collector, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(
            event::mock()
                .with_fields(field::mock("return").with_value(&tracing::field::debug(42)))
                .at_level(Level::TRACE),
        )
        .exit(span.clone())
        .drop_span(span)
        .done()
        .run_with_handle();

    with_default(collector, ret);
    handle.assert_finished();
}

#[instrument(ret)]
fn ret_mut(a: &mut i32) -> i32 {
    *a *= 2;
    tracing::info!(?a);
    *a
}

#[test]
fn test_mut() {
    let span = span::mock().named("ret_mut");
    let (collector, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(
            event::mock()
                .with_fields(field::mock("a").with_value(&tracing::field::display(2)))
                .at_level(Level::INFO),
        )
        .event(
            event::mock()
                .with_fields(field::mock("return").with_value(&tracing::field::display(2)))
                .at_level(Level::TRACE),
        )
        .exit(span.clone())
        .drop_span(span)
        .done()
        .run_with_handle();

    with_default(collector, || ret_mut(&mut 1));
    handle.assert_finished();
}

#[instrument(ret)]
async fn ret_async() -> i32 {
    42
}

#[test]
fn test_async() {
    let span = span::mock().named("ret_async");
    let (collector, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(
            event::mock()
                .with_fields(field::mock("return").with_value(&tracing::field::debug(42)))
                .at_level(Level::TRACE),
        )
        .exit(span.clone())
        .drop_span(span)
        .done()
        .run_with_handle();

    with_default(collector, || block_on_future(async { ret_async().await }));
    handle.assert_finished();
}

#[instrument(ret)]
fn ret_impl_type() -> impl Copy {
    42
}

#[test]
fn test_impl_type() {
    let span = span::mock().named("ret_impl_type");
    let (collector, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(
            event::mock()
                .with_fields(field::mock("return").with_value(&tracing::field::debug(42)))
                .at_level(Level::TRACE),
        )
        .exit(span.clone())
        .drop_span(span)
        .done()
        .run_with_handle();

    with_default(collector, ret_impl_type);
    handle.assert_finished();
}

#[instrument(ret(Debug))]
fn ret_dbg() -> i32 {
    42
}

#[test]
fn test_dbg() {
    let span = span::mock().named("ret_dbg");
    let (collector, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(
            event::mock()
                .with_fields(field::mock("return").with_value(&tracing::field::debug(42)))
                .at_level(Level::TRACE),
        )
        .exit(span.clone())
        .drop_span(span)
        .done()
        .run_with_handle();

    with_default(collector, ret_dbg);
    handle.assert_finished();
}

#[instrument(err, ret(Debug))]
fn ret_and_err() -> Result<u8, TryFromIntError> {
    u8::try_from(1234)
}

#[test]
fn test_ret_and_err() {
    let span = span::mock().named("ret_and_err");
    let (collector, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(
            event::mock()
                .with_fields(
                    field::mock("error")
                        .with_value(&tracing::field::display(u8::try_from(1234).unwrap_err()))
                        .only(),
                )
                .at_level(Level::ERROR),
        )
        .exit(span.clone())
        .drop_span(span)
        .done()
        .run_with_handle();

    with_default(collector, || ret_and_err().ok());
    handle.assert_finished();
}

#[instrument(err, ret(Debug))]
fn ret_and_ok() -> Result<u8, TryFromIntError> {
    u8::try_from(123)
}

#[test]
fn test_ret_and_ok() {
    let span = span::mock().named("ret_and_ok");
    let (collector, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(
            event::mock()
                .with_fields(
                    field::mock("return")
                        .with_value(&tracing::field::display(u8::try_from(123).unwrap()))
                        .only(),
                )
                .at_level(Level::TRACE),
        )
        .exit(span.clone())
        .drop_span(span)
        .done()
        .run_with_handle();

    with_default(collector, || ret_and_ok().ok());
    handle.assert_finished();
}
