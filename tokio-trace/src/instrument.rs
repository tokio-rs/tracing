use super::Span;
use futures::{Future, Sink, Stream, Poll, StartSend};

// TODO: seal?
pub trait Instrument: Sized {
    fn instrument(self, span: Span) -> Instrumented<Self> {
        Instrumented {
            inner: self,
            span,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Instrumented<T> {
    inner: T,
    span: Span,
}

impl<T: Sized> Instrument for T {}

impl<T: Future> Future for Instrumented<T> {
    type Item = T::Item;
    type Error = T::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let span = &self.span;
        let inner = &mut self.inner;
        span.enter(move || {
            inner.poll()
        })
    }
}

impl<T: Stream> Stream for Instrumented<T> {
    type Item = T::Item;
    type Error = T::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let span = &self.span;
        let inner = &mut self.inner;
        span.enter(move || {
            inner.poll()
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
        let span = &self.span;
        let inner = &mut self.inner;
        span.enter(move || {
            inner.start_send(item)
        })
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        let span = &self.span;
        let inner = &mut self.inner;
        span.enter(move || {
            inner.poll_complete()
        })
    }
}
