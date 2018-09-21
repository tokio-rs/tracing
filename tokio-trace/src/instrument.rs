use super::Span;
use futures::{Future, Sink, Stream, Poll};


pub trait Instrument: Sized {
    fn instrument(self, span: Span) -> Instrumented<Self> {
        Instrumented {
            inner: self,
            span
        }
    }
}

#[derive(Clone, Debug)]
pub struct Instrumented<T> {
    inner: T,
    span: Span,
}

impl<T: Future + Sized> Instrument for T {}
impl<T: Stream + Sized> Instrument for T {}
impl<T: Sink + Sized> Instrument for T {}

impl<T: Future> Future for Instrument<T> {
    type Item = T::Item;
    type Error = T::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.span.enter(|| {
            self.inner.poll()
        })
    }
}

impl<T: Stream> Stream for Instrument<T> {
    type Item = T::Item;
    type Error = T::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.span.enter(|| {
            self.inner.poll()
        })
    }
}

impl<T: Sink> Sink for Instrument<T> {
    type SinkItem = T::SinkItem;
    type SinkError = T::SinkError;

    fn start_send(
        &mut self,
        item: Self::SinkItem
    ) -> StartSend<Self::SinkItem, Self::SinkError> {
        self.span.enter(|| self.inner.start_send(item))
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.span.enter(|| { self.inner.poll_complete() })
    }
}
