#[macro_use]
extern crate tracing;
extern crate tracing_core;

pub use self::support as test_support;
// This has to have the same name as the module in `tracing`.
#[path = "../../../tracing/tests/support/mod.rs"]
pub mod support;

use std::pin::Pin;
use std::task::Context;

use support::*;
use tokio_test::task::MockTask;
use tracing::{subscriber::with_default, Level};
use tracing_futures::Instrument;

struct PollN<T, E> {
    and_return: Option<Result<T, E>>,
    finish_at: usize,
    polls: usize,
}

impl<T, E> std::future::Future for PollN<T, E>
where
    T: Unpin,
    E: Unpin,
{
    type Output = Result<T, E>;
    fn poll(self: Pin<&mut Self>, lw: &mut Context) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();

        this.polls += 1;
        if this.polls == this.finish_at {
            let value = this.and_return.take().expect("polled after ready");

            std::task::Poll::Ready(value)
        } else {
            lw.waker().wake_by_ref();
            std::task::Poll::Pending
        }
    }
}

impl PollN<(), ()> {
    fn new_ok(finish_at: usize) -> Self {
        Self {
            and_return: Some(Ok(())),
            finish_at,
            polls: 0,
        }
    }

    fn new_err(finish_at: usize) -> Self {
        Self {
            and_return: Some(Err(())),
            finish_at,
            polls: 0,
        }
    }
}

fn block_on_future<F>(task: &mut MockTask, future: F) -> F::Output
where
    F: std::future::Future,
{
    let mut future = Box::pin(future);

    loop {
        match task.poll(&mut future) {
            std::task::Poll::Ready(v) => break v,
            _ => {}
        }
    }
}

#[test]
fn std_future_enter_exit_is_reasonable() {
    let (subscriber, handle) = subscriber::mock()
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
fn std_future_error_ends_span() {
    let (subscriber, handle) = subscriber::mock()
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
