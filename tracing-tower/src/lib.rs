extern crate tower_service;
#[macro_use]
extern crate tracing;
extern crate futures;
extern crate tracing_futures;

use std::fmt;
use tower_service::Service;
use tracing::{field, Level};
use tracing_futures::{Instrument, Instrumented};

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
        let _enter = self.span.enter();
        self.inner.poll_ready()
    }

    fn call(&mut self, req: Request) -> Self::Future {
        // TODO: custom `Value` impls for `http` types would be nice...
        let span = span!(Level::TRACE, parent: &self.span, "request", request = ?req);
        let _enter = span.enter();
        self.inner.call(req).instrument(span.clone())
    }
}
