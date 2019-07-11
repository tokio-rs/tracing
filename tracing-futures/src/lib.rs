//! Futures compatibility for `tracing`
//!
//! # Feature flags
//! - `tokio`: Enables compatibility with the `tokio` crate, including
//!    [`Instrument`] and [`WithSubscriber`] implementations for
//!    `tokio::executor::Executor`, `tokio::runtime::Runtime`, and
//!    `tokio::runtime::current_thread`. Enabled by default.
//! - `tokio-executor`: Enables compatibility with the `tokio-executor`
//!    crate, including  [`Instrument`] and [`WithSubscriber`]
//!    implementations for types implementing `tokio_executor::Executor`.
//!    This is intended primarily for use in crates which depend on
//!    `tokio-executor` rather than `tokio`; in general the `tokio` feature
//!    should be used instead.
//!
//! [`Instrument`]: trait.Instrument.html
//! [`WithSubscriber`]: trait.WithSubscriber.html
#[cfg(feature = "futures-01")]
extern crate futures;
#[cfg(feature = "std-future")]
extern crate pin_utils;
#[cfg(feature = "tokio")]
extern crate tokio;
#[cfg(feature = "tokio-executor")]
extern crate tokio_executor;
#[cfg_attr(test, macro_use)]
extern crate tracing;

#[cfg(feature = "std-future")]
use std::{pin::Pin, task::Context};

#[cfg(feature = "futures-01")]
use futures::{Sink, StartSend, Stream};
#[cfg(feature = "futures-01")]
use tracing::dispatcher;
use tracing::{Dispatch, Span};

#[cfg(feature = "tokio")]
pub mod executor;

// TODO: seal?
pub trait Instrument: Sized {
    fn instrument(self, span: Span) -> Instrumented<Self> {
        Instrumented { inner: self, span }
    }
}

pub trait WithSubscriber: Sized {
    fn with_subscriber<S>(self, subscriber: S) -> WithDispatch<Self>
    where
        S: Into<Dispatch>,
    {
        WithDispatch {
            inner: self,
            dispatch: subscriber.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Instrumented<T> {
    inner: T,
    span: Span,
}

#[derive(Clone, Debug)]
pub struct WithDispatch<T> {
    inner: T,
    dispatch: Dispatch,
}

impl<T: Sized> Instrument for T {}

#[cfg(feature = "std-future")]
impl<T: std::future::Future> Instrumented<T> {
    pin_utils::unsafe_pinned!(inner: T);
}

#[cfg(feature = "std-future")]
impl<T: std::future::Future> std::future::Future for Instrumented<T> {
    type Output = T::Output;

    fn poll(mut self: Pin<&mut Self>, lw: &mut Context) -> std::task::Poll<Self::Output> {
        let span = self.as_ref().span.clone();
        let _enter = span.enter();
        self.as_mut().inner().poll(lw)
    }
}

#[cfg(feature = "std-future")]
impl<T: Unpin> Unpin for Instrumented<T> {}

#[cfg(feature = "futures-01")]
impl<T: futures::Future> futures::Future for Instrumented<T> {
    type Item = T::Item;
    type Error = T::Error;

    fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
        let _enter = self.span.enter();
        self.inner.poll()
    }
}

#[cfg(feature = "futures-01")]
impl<T: Stream> Stream for Instrumented<T> {
    type Item = T::Item;
    type Error = T::Error;

    fn poll(&mut self) -> futures::Poll<Option<Self::Item>, Self::Error> {
        let _enter = self.span.enter();
        self.inner.poll()
    }
}

#[cfg(feature = "futures-01")]
impl<T: Sink> Sink for Instrumented<T> {
    type SinkItem = T::SinkItem;
    type SinkError = T::SinkError;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        let _enter = self.span.enter();
        self.inner.start_send(item)
    }

    fn poll_complete(&mut self) -> futures::Poll<(), Self::SinkError> {
        let _enter = self.span.enter();
        self.inner.poll_complete()
    }
}

impl<T> Instrumented<T> {
    /// Borrows the `Span` that this type is instrumented by.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Mutably borrows the `Span` that this type is instrumented by.
    pub fn span_mut(&mut self) -> &mut Span {
        &mut self.span
    }

    /// Consumes the `Instrumented`, returning the wrapped type.
    ///
    /// Note that this drops the span.
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T: Sized> WithSubscriber for T {}

#[cfg(feature = "futures-01")]
impl<T: futures::Future> futures::Future for WithDispatch<T> {
    type Item = T::Item;
    type Error = T::Error;

    fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
        let inner = &mut self.inner;
        dispatcher::with_default(&self.dispatch, || inner.poll())
    }
}

impl<T> WithDispatch<T> {
    #[cfg(feature = "tokio")]
    pub(crate) fn with_dispatch<U: Sized>(&self, inner: U) -> WithDispatch<U> {
        WithDispatch {
            dispatch: self.dispatch.clone(),
            inner,
        }
    }

    /// Borrows the `Dispatch` that this type is instrumented by.
    pub fn dispatch(&self) -> &Dispatch {
        &self.dispatch
    }

    /// Consumes the `WithDispatch`, returning the wrapped type.
    pub fn into_inner(self) -> T {
        self.inner
    }
}

#[cfg(test)]
pub use self::support as test_support;
// This has to have the same name as the module in `tracing`.
#[path = "../../tracing/tests/support/mod.rs"]
#[cfg(test)]
pub mod support;

#[cfg(test)]
mod tests {
    extern crate tokio;

    use super::{test_support::*, *};

    #[cfg(feature = "futures-01")]
    use futures::{future, stream, task, Async, Future};
    #[cfg(feature = "std-future")]
    use tokio_test::task::MockTask;
    use tracing::{subscriber::with_default, Level};

    struct PollN<T, E> {
        and_return: Option<Result<T, E>>,
        finish_at: usize,
        polls: usize,
    }

    #[cfg(feature = "std-future")]
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

    #[cfg(feature = "futures-01")]
    impl<T, E> futures::Future for PollN<T, E> {
        type Item = T;
        type Error = E;
        fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
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

    #[cfg(feature = "std-future")]
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

    #[cfg(feature = "futures-01")]
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
                .instrument(span!(Level::TRACE, "foo"))
                .wait()
                .unwrap();
        });
        handle.assert_finished();
    }

    #[cfg(feature = "futures-01")]
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
                .instrument(span!(Level::TRACE, "foo"))
                .wait()
                .unwrap_err();
        });

        handle.assert_finished();
    }

    #[cfg(feature = "futures-01")]
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
                .instrument(span!(Level::TRACE, "foo"))
                .for_each(|_| future::ok(()))
                .wait()
                .unwrap();
        });
        handle.assert_finished();
    }

    #[cfg(feature = "futures-01")]
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
            span!(Level::TRACE, "a").in_scope(|| {
                let future = PollN::new_ok(2)
                    .instrument(span!(Level::TRACE, "b"))
                    .map(|_| {
                        span!(Level::TRACE, "c").in_scope(|| {
                            // "c" happens _outside_ of the instrumented future's
                            // span, so we don't expect it.
                        })
                    });
                runtime.block_on(Box::new(future)).unwrap();
            })
        });
        handle.assert_finished();
    }

    #[cfg(feature = "std-future")]
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

    #[cfg(feature = "std-future")]
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
}
