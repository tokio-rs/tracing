extern crate tower_service;
#[macro_use]
extern crate tokio_trace;
extern crate futures;
extern crate tokio_trace_futures;

use std::fmt;
use tokio_trace::field;
use tokio_trace_futures::{Instrument, Instrumented};
use tower_service::Service;

// TODO: Can this still be `Clone` (some kind of SharedSpan type?)
#[derive(Debug)]
pub struct InstrumentedService<T> {
    inner: T,
    span: tokio_trace::Span,
}

pub trait InstrumentableService<Request>: Service<Request> + Sized {
    fn instrument(self, span: tokio_trace::Span) -> InstrumentedService<Self> {
        InstrumentedService { inner: self, span }
    }
}

impl<T: Service<Request>, Request> Service<Request> for InstrumentedService<T>
where
    // TODO: it would be nice to do more for HTTP services...
    Request: fmt::Debug + Clone + Send + Sync + 'static,
{
    type Response = T::Response;
    type Future = Instrumented<T::Future>;
    type Error = T::Error;

    fn poll_ready(&mut self) -> futures::Poll<(), Self::Error> {
        let span = &mut self.span;
        let inner = &mut self.inner;
        span.enter(|| inner.poll_ready())
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let span = &mut self.span;
        let inner = &mut self.inner;
        span.enter(|| {
            // TODO: custom `Value` impls for `http` types would be nice...
            span!("request", request = &field::debug(&req))
                .enter(move || inner.call(req).in_current_span())
        })
    }
}
