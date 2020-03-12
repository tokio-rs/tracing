use futures_01::{future, stream, task, Async, Future, Stream};
use tracing::subscriber::with_default;

use super::support::*;
use crate::*;

struct PollN<T, E> {
    and_return: Option<Result<T, E>>,
    finish_at: usize,
    polls: usize,
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

impl<T, E> futures_01::Future for PollN<T, E> {
    type Item = T;
    type Error = E;
    fn poll(&mut self) -> futures_01::Poll<Self::Item, Self::Error> {
        self.polls += 1;
        if self.polls == self.finish_at {
            self.and_return
                .take()
                .expect("polled after ready")
                .map(Async::Ready)
        } else {
            task::current().notify();
            Ok(Async::NotReady)
        }
    }
}

#[test]
fn future_enter_exit_is_reasonable() {
    let (subscriber, handle) = subscriber::mock()
        .enter(span::mock().named("foo"))
        .exit(span::mock().named("foo"))
        .enter(span::mock().named("foo"))
        .exit(span::mock().named("foo"))
        .drop_span(span::mock().named("foo"))
        .done()
        .run_with_handle();
    with_default(subscriber, || {
        PollN::new_ok(2)
            .instrument(tracing::trace_span!("foo"))
            .wait()
            .unwrap();
    });
    handle.assert_finished();
}

#[test]
fn future_error_ends_span() {
    let (subscriber, handle) = subscriber::mock()
        .enter(span::mock().named("foo"))
        .exit(span::mock().named("foo"))
        .enter(span::mock().named("foo"))
        .exit(span::mock().named("foo"))
        .drop_span(span::mock().named("foo"))
        .done()
        .run_with_handle();
    with_default(subscriber, || {
        PollN::new_err(2)
            .instrument(tracing::trace_span!("foo"))
            .wait()
            .unwrap_err();
    });

    handle.assert_finished();
}

#[test]
fn stream_enter_exit_is_reasonable() {
    let (subscriber, handle) = subscriber::mock()
        .enter(span::mock().named("foo"))
        .exit(span::mock().named("foo"))
        .enter(span::mock().named("foo"))
        .exit(span::mock().named("foo"))
        .enter(span::mock().named("foo"))
        .exit(span::mock().named("foo"))
        .enter(span::mock().named("foo"))
        .exit(span::mock().named("foo"))
        .drop_span(span::mock().named("foo"))
        .run_with_handle();
    with_default(subscriber, || {
        stream::iter_ok::<_, ()>(&[1, 2, 3])
            .instrument(tracing::trace_span!("foo"))
            .for_each(|_| future::ok(()))
            .wait()
            .unwrap();
    });
    handle.assert_finished();
}

#[test]
fn span_follows_future_onto_threadpool() {
    let (subscriber, handle) = subscriber::mock()
        .enter(span::mock().named("a"))
        .enter(span::mock().named("b"))
        .exit(span::mock().named("b"))
        .enter(span::mock().named("b"))
        .exit(span::mock().named("b"))
        .drop_span(span::mock().named("b"))
        .exit(span::mock().named("a"))
        .drop_span(span::mock().named("a"))
        .done()
        .run_with_handle();
    let mut runtime = tokio::runtime::Runtime::new().unwrap();
    with_default(subscriber, || {
        tracing::trace_span!("a").in_scope(|| {
            let future = PollN::new_ok(2)
                .instrument(tracing::trace_span!("b"))
                .map(|_| {
                    tracing::trace_span!("c").in_scope(|| {
                        // "c" happens _outside_ of the instrumented future's
                        // span, so we don't expect it.
                    })
                });
            runtime.block_on(Box::new(future)).unwrap();
        })
    });
    handle.assert_finished();
}
