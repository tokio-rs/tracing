use tracing::Instrument;
use tracing::{collect::with_default, Level};
use tracing_mock::*;

#[test]
fn enter_exit_is_reasonable() {
    let span = expect::span().named("foo");
    let (collector, handle) = collector::mock()
        .enter(span.clone())
        .exit(span.clone())
        .enter(span.clone())
        .exit(span.clone())
        .enter(span.clone())
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();
    with_default(collector, || {
        let future = PollN::new_ok(2).instrument(tracing::span!(Level::TRACE, "foo"));
        block_on_future(future).unwrap();
    });
    handle.assert_finished();
}

#[test]
fn error_ends_span() {
    let span = expect::span().named("foo");
    let (collector, handle) = collector::mock()
        .enter(span.clone())
        .exit(span.clone())
        .enter(span.clone())
        .exit(span.clone())
        .enter(span.clone())
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();
    with_default(collector, || {
        let future = PollN::new_err(2).instrument(tracing::span!(Level::TRACE, "foo"));
        block_on_future(future).unwrap_err();
    });
    handle.assert_finished();
}
