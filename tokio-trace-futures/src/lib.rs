extern crate futures;
#[cfg_attr(test, macro_use)]
extern crate tokio_trace;

use futures::{Async, Future, Poll, Sink, StartSend, Stream};
use tokio_trace::{Dispatch, Span, Subscriber};

pub mod executor;

// TODO: seal?
pub trait Instrument: Sized {
    fn instrument(self, span: Span) -> Instrumented<Self> {
        Instrumented {
            inner: self,
            span: Some(span),
        }
    }
}

pub trait WithSubscriber: Sized {
    fn with_subscriber<S>(self, subscriber: S) -> WithDispatch<Self>
    where
        S: Subscriber + Send + Sync + 'static,
    {
        WithDispatch {
            inner: self,
            dispatch: Dispatch::to(subscriber),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Instrumented<T> {
    inner: T,
    span: Option<Span>,
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
        let span = self.span.take().expect("polled after Ready");
        let new_span = &mut self.span;
        let inner = &mut self.inner;
        span.clone().enter(move || match inner.poll() {
            Ok(Async::NotReady) => {
                *new_span = Some(span);
                Ok(Async::NotReady)
            }
            poll => poll,
        })
    }
}

impl<T: Stream> Stream for Instrumented<T> {
    type Item = T::Item;
    type Error = T::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let span = self.span.take().expect("polled after None");
        let new_span = &mut self.span;
        let inner = &mut self.inner;
        span.clone().enter(move || match inner.poll() {
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Err(e) => Err(e),
            poll => {
                *new_span = Some(span);
                poll
            }
        })
    }
}

impl<T: Sink> Sink for Instrumented<T> {
    type SinkItem = T::SinkItem;
    type SinkError = T::SinkError;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        let span = self
            .span
            .as_ref()
            .cloned()
            .expect("span never goes away for Sinks");
        let inner = &mut self.inner;
        span.enter(move || inner.start_send(item))
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        let span = self
            .span
            .as_ref()
            .cloned()
            .expect("span never goes away for Sinks");
        let inner = &mut self.inner;
        span.enter(move || inner.poll_complete())
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
            .exit(
                span::mock()
                    .named(Some("foo"))
                    .with_state(span::State::Idle),
            ).enter(span::mock().named(Some("foo")))
            .exit(
                span::mock()
                    .named(Some("foo"))
                    .with_state(span::State::Done),
            ).run();
        Dispatch::to(subscriber).as_default(|| {
            MyFuture { polls: 0 }
                .instrument(span!("foo"))
                .wait()
                .unwrap();
        })
    }

    #[test]
    fn future_completion_leaves_reachable_spans_idle() {
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
            .exit(
                span::mock()
                    .named(Some("foo"))
                    .with_state(span::State::Idle),
            ).enter(span::mock().named(Some("foo")))
            .exit(
                span::mock()
                    .named(Some("foo"))
                    .with_state(span::State::Idle),
            ).run();
        Dispatch::to(subscriber).as_default(|| {
            let foo = span!("foo");
            MyFuture { polls: 0 }
                .instrument(foo.clone())
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
            .exit(
                span::mock()
                    .named(Some("foo"))
                    .with_state(span::State::Idle),
            ).enter(span::mock().named(Some("foo")))
            .exit(
                span::mock()
                    .named(Some("foo"))
                    .with_state(span::State::Done),
            ).run();
        Dispatch::to(subscriber).as_default(|| {
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
            .exit(
                span::mock()
                    .named(Some("foo"))
                    .with_state(span::State::Idle),
            ).enter(span::mock().named(Some("foo")))
            .exit(
                span::mock()
                    .named(Some("foo"))
                    .with_state(span::State::Idle),
            ).enter(span::mock().named(Some("foo")))
            .exit(
                span::mock()
                    .named(Some("foo"))
                    .with_state(span::State::Idle),
            ).enter(span::mock().named(Some("foo")))
            .exit(
                span::mock()
                    .named(Some("foo"))
                    .with_state(span::State::Done),
            ).run();
        Dispatch::to(subscriber).as_default(|| {
            stream::iter_ok::<_, ()>(&[1, 2, 3])
                .instrument(span!("foo"))
                .for_each(|_| future::ok(()))
                .wait()
                .unwrap();
        })
    }
}
