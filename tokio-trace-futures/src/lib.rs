extern crate futures;
extern crate tokio_trace;

use futures::{Async, Future, Poll, Sink, StartSend, Stream};
use tokio_trace::{Dispatch, Span, Subscriber};

pub mod executor;

// TODO: seal?
pub trait Instrument: Sized {
    // TODO: consider renaming to `in_span` for consistency w/
    // `in_current_span`?
    fn instrument(self, span: Span) -> Instrumented<Self> {
        Instrumented { inner: self, span }
    }

    fn in_current_span(self) -> Instrumented<Self> {
        self.instrument(Span::current())
    }
}

pub trait WithSubscriber: Sized {
    fn with_subscriber<S>(self, subscriber: S) -> WithDispatch<Self>
    where
        S: Subscriber + Send + Sync + 'static,
    {
        WithDispatch {
            inner: self,
            dispatch: Dispatch::new(subscriber),
        }
    }
}

#[derive(Debug)]
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

impl<T: Future> Future for Instrumented<T> {
    type Item = T::Item;
    type Error = T::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let span = &mut self.span;
        let inner = &mut self.inner;
        span.enter(|| match inner.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            done => {
                // The future finished, either successfully or with an error.
                // The span should now close.
                Span::current().close();
                done
            }
        })
    }
}

impl<T: Stream> Stream for Instrumented<T> {
    type Item = T::Item;
    type Error = T::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let span = &mut self.span;
        let inner = &mut self.inner;
        span.enter(|| match inner.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(Some(thing))) => Ok(Async::Ready(Some(thing))),
            done => {
                // The stream finished, either with `Ready(None)` or with an error.
                // The span should now close.
                Span::current().close();
                done
            }
        })
    }
}

impl<T: Sink> Sink for Instrumented<T> {
    type SinkItem = T::SinkItem;
    type SinkError = T::SinkError;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        let span = &mut self.span;
        let inner = &mut self.inner;
        span.enter(|| inner.start_send(item))
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        let span = &mut self.span;
        let inner = &mut self.inner;
        span.enter(|| inner.poll_complete())
    }
}

impl<T: Sized> WithSubscriber for T {}

impl<T: Future> Future for WithDispatch<T> {
    type Item = T::Item;
    type Error = T::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let dispatch = &self.dispatch;
        let inner = &mut self.inner;
        dispatch.as_default(|| inner.poll())
    }
}

impl<T> WithDispatch<T> {
    pub(crate) fn with_dispatch<U: Sized>(&self, inner: U) -> WithDispatch<U> {
        WithDispatch {
            dispatch: self.dispatch.clone(),
            inner,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{future, stream, task};
    use tokio_trace::{span, subscriber, Dispatch};

    #[test]
    fn future_enter_exit_is_reasonable() {
        struct MyFuture {
            polls: usize,
        }

        impl Future for MyFuture {
            type Item = ();
            type Error = ();
            fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
                self.polls += 1;
                if self.polls == 2 {
                    Ok(Async::Ready(()))
                } else {
                    task::current().notify();
                    Ok(Async::NotReady)
                }
            }
        }
        let subscriber = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .close(span::mock().named(Some("foo")))
            .done()
            .run();
        Dispatch::new(subscriber).as_default(|| {
            MyFuture { polls: 0 }
                .instrument(span!("foo"))
                .wait()
                .unwrap();
        })
    }

    #[test]
    fn future_error_ends_span() {
        struct MyFuture {
            polls: usize,
        }

        impl Future for MyFuture {
            type Item = ();
            type Error = ();
            fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
                self.polls += 1;
                if self.polls == 2 {
                    Err(())
                } else {
                    task::current().notify();
                    Ok(Async::NotReady)
                }
            }
        }
        let subscriber = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .close(span::mock().named(Some("foo")))
            .done()
            .run();
        Dispatch::new(subscriber).as_default(|| {
            MyFuture { polls: 0 }
                .instrument(span!("foo"))
                .wait()
                .unwrap_err();
        })
    }

    #[test]
    fn stream_enter_exit_is_reasonable() {
        let subscriber = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .close(span::mock().named(Some("foo")))
            .run();
        Dispatch::new(subscriber).as_default(|| {
            stream::iter_ok::<_, ()>(&[1, 2, 3])
                .instrument(span!("foo"))
                .for_each(|_| future::ok(()))
                .wait()
                .unwrap();
        })
    }
}
