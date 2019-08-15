#![feature(async_await)]

use test_std_future::{
    PollN,
    block_on_future,
    support::*,
};

use tokio_test::task::MockTask;
use tracing::subscriber::with_default;

#[tracing_attributes::instrument]
async fn test_async_fn(polls: usize) -> Result<(), ()> {
    let future = PollN::new_ok(polls);
    tracing::trace!(awaiting = true);
    future.await
}

#[test]
fn async_fn_only_enters_for_polls() {
    let (subscriber, handle) = subscriber::test()
        .new_span(span::mock().named("test_async_fn"))
        .enter(span::mock().named("test_async_fn"))
        .event(event::mock().with_fields(field::mock("awaiting").with_value(&true)))
        .exit(span::mock().named("test_async_fn"))
        .enter(span::mock().named("test_async_fn"))
        .exit(span::mock().named("test_async_fn"))
        .drop_span(span::mock().named("test_async_fn"))
        .done()
        .run_with_handle();
    let mut task = MockTask::new();
    with_default(subscriber, || {
        block_on_future(&mut task, async { test_async_fn(2).await }).unwrap();
    });
    handle.assert_finished();
}
