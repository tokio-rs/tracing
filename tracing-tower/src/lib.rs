extern crate tower_service;
#[macro_use]
extern crate tracing;
extern crate futures;
extern crate tracing_futures;

use std::fmt;
use tower_service::Service;
use tracing_futures::{Instrument, Instrumented};
use tracing::{field, Level};


#[derive(Clone, Debug)]
pub struct InstrumentedService<T> {
    inner: T,
    span: tracing::Span,
}

pub trait InstrumentableService<Request>: Service<Request> + Sized {
    fn instrument(self, span: tracing::Span) -> InstrumentedService<Self> {
        InstrumentedService { inner: self, span }
    }
}

impl<T: Service<Request>, Request> Service<Request> for InstrumentedService<T>
where
    // TODO: it would be nice to do more for HTTP services...
    Request: fmt::Debug + Clone + Send + Sync + 'static,
{
    type Response = T::Response;
    type Error = T::Error;
    type Future = Instrumented<T::Future>;

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
            let span = span!(Level::TRACE, "request", request = &field::debug(&req));
            let span2 = span.clone();
            span.enter(move || inner.call(req).instrument(span2))
        })
    }
}
