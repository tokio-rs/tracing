use super::Span;
use futures::{Async, Future, Sink, Stream, Poll, StartSend};

// TODO: seal?
pub trait Instrument: Sized {
    fn instrument(self, span: Span) -> Instrumented<Self> {
        Instrumented {
            inner: self,
            span: Some(span),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Instrumented<T> {
    inner: T,
    span: Option<Span>,
}

impl<T: Sized> Instrument for T {}

impl<T: Future> Future for Instrumented<T> {
    type Item = T::Item;
    type Error = T::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let span = self.span.take()
            .expect("polled after Ready");
        let new_span = &mut self.span;
        let inner = &mut self.inner;
        span.clone().enter(move || {
            match inner.poll() {
                Ok(Async::NotReady) => {
                    *new_span = Some(span);
                    Ok(Async::NotReady)
                },
                poll => poll,
            }
        })
    }
}

impl<T: Stream> Stream for Instrumented<T> {
    type Item = T::Item;
    type Error = T::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let span = self.span.take()
            .expect("polled after None");
        let new_span = &mut self.span;
        let inner = &mut self.inner;
        span.clone().enter(move || {
            match inner.poll() {
                Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
                Err(e) => Err(e),
                poll => {
                    *new_span = Some(span);
                    poll
                },
            }
        })
    }
}

impl<T: Sink> Sink for Instrumented<T> {
    type SinkItem = T::SinkItem;
    type SinkError = T::SinkError;

    fn start_send(
        &mut self,
        item: Self::SinkItem
    ) -> StartSend<Self::SinkItem, Self::SinkError> {
        let span = self.span.as_ref().cloned()
            .expect("span never goes away for Sinks");
        let inner = &mut self.inner;
        span.enter(move || {
            inner.start_send(item)
        })
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        let span = self.span.as_ref().cloned()
            .expect("span never goes away for Sinks");
        let inner = &mut self.inner;
        span.enter(move || {
            inner.poll_complete()
        })
    }
}


#[cfg(test)]
mod tests {
    use futures::{prelude::*, task, future, stream};
    use ::{span, subscriber};
    use super::*;

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
        subscriber::mock()
            .enter(
                span::mock().named(Some("foo"))
            )
            .exit(
                span::mock().named(Some("foo"))
                    .with_state(span::State::Idle)
            )
            .enter(
                span::mock().named(Some("foo"))
            )
            .exit(
                span::mock().named(Some("foo"))
                    .with_state(span::State::Done)
            )
            .run();
        MyFuture { polls: 0 }
            .instrument(span!("foo",))
            .wait().unwrap();
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
        subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .exit(
                span::mock().named(Some("foo"))
                    .with_state(span::State::Idle)
            )
            .enter(span::mock().named(Some("foo")))
            .exit(
                span::mock().named(Some("foo"))
                    .with_state(span::State::Idle)
            )
            .run();
        let foo = span!("foo",);
        MyFuture { polls: 0 }
            .instrument(foo.clone())
            .wait().unwrap();
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
        subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .exit(
                span::mock().named(Some("foo"))
                    .with_state(span::State::Idle)
            )
            .enter(span::mock().named(Some("foo")))
            .exit(
                span::mock().named(Some("foo"))
                    .with_state(span::State::Done)
            )
            .run();
        MyFuture { polls: 0 }
            .instrument(span!("foo",))
            .wait().unwrap_err();
    }

    #[test]
    fn stream_enter_exit_is_reasonable() {
        subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .exit(
                span::mock().named(Some("foo"))
                    .with_state(span::State::Idle)
            )
            .enter(span::mock().named(Some("foo")))
            .exit(
                span::mock().named(Some("foo"))
                    .with_state(span::State::Idle)
            )
            .enter(span::mock().named(Some("foo")) )
            .exit(
                span::mock().named(Some("foo"))
                    .with_state(span::State::Idle)
            )
            .enter(span::mock().named(Some("foo")))
            .exit(
                span::mock().named(Some("foo"))
                    .with_state(span::State::Done)
            )
            .run();
        stream::iter_ok::<_, ()>(&[1, 2, 3])
            .instrument(span!("foo",))
            .for_each(|_| { future::ok(()) })
            .wait().unwrap();
    }
}
