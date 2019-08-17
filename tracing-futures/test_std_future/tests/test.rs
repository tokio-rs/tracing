#[macro_use]
extern crate tracing;
extern crate tracing_core;

use test_std_future::{block_on_future, support::*, PollN};

use tokio_test::task::MockTask;
use tracing::{subscriber::with_default, Level};
use tracing_futures::Instrument;

#[test]
fn enter_exit_is_reasonable() {
    let (subscriber, handle) = subscriber::SubscriberTest::new()
        .enter(span::mock().named("foo"))
        .exit(span::mock().named("foo"))
        .enter(span::mock().named("foo"))
        .exit(span::mock().named("foo"))
        .drop_span(span::mock().named("foo"))
        .done()
        .run_with_handle();
    let mut task = MockTask::new();
    with_default(subscriber, || {
        let future = PollN::new_ok(2).instrument(span!(Level::TRACE, "foo"));
        block_on_future(&mut task, future).unwrap();
    });
    handle.assert_finished();
}

#[test]
fn error_ends_span() {
    let (subscriber, handle) = subscriber::SubscriberTest::new()
        .enter(span::mock().named("foo"))
        .exit(span::mock().named("foo"))
        .enter(span::mock().named("foo"))
        .exit(span::mock().named("foo"))
        .drop_span(span::mock().named("foo"))
        .done()
        .run_with_handle();
    let mut task = MockTask::new();
    with_default(subscriber, || {
        let future = PollN::new_err(2).instrument(span!(Level::TRACE, "foo"));
        block_on_future(&mut task, future).unwrap_err();
    });
    handle.assert_finished();
}
