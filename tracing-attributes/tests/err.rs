#[path = "../../tracing-futures/tests/support.rs"]
// we don't use some of the test support functions, but `tracing-futures` does.
#[allow(dead_code)]
mod support;
use support::*;

use tracing::collector::with_default;
use tracing::Level;
use tracing_attributes::instrument;

use std::convert::TryFrom;
use std::num::TryFromIntError;

#[instrument(err)]
fn err() -> Result<u8, TryFromIntError> {
    u8::try_from(1234)
}

#[test]
fn test() {
    let span = span::mock().named("err");
    let (collector, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(event::mock().at_level(Level::ERROR))
        .exit(span.clone())
        .drop_span(span)
        .done()
        .run_with_handle();
    with_default(collector, || err().ok());
    handle.assert_finished();
}

#[instrument(err)]
fn err_early_return() -> Result<u8, TryFromIntError> {
    u8::try_from(1234)?;
    Ok(5)
}

#[test]
fn test_early_return() {
    let span = span::mock().named("err_early_return");
    let (subscriber, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(event::mock().at_level(Level::ERROR))
        .exit(span.clone())
        .drop_span(span)
        .done()
        .run_with_handle();
    with_default(subscriber, || err_early_return().ok());
    handle.assert_finished();
}

#[instrument(err)]
async fn err_async(polls: usize) -> Result<u8, TryFromIntError> {
    let future = PollN::new_ok(polls);
    tracing::trace!(awaiting = true);
    future.await.ok();
    u8::try_from(1234)
}

#[test]
fn test_async() {
    let span = span::mock().named("err_async");
    let (collector, handle) = collector::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(
            event::mock()
                .with_fields(field::mock("awaiting").with_value(&true))
                .at_level(Level::TRACE),
        )
        .exit(span.clone())
        .enter(span.clone())
        .event(event::mock().at_level(Level::ERROR))
        .exit(span.clone())
        .drop_span(span)
        .done()
        .run_with_handle();
    with_default(collector, || {
        block_on_future(async { err_async(2).await }).ok();
    });
    handle.assert_finished();
}
