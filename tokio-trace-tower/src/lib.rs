extern crate tower_service;
#[macro_use]
extern crate tokio_trace;
extern crate futures;

use tokio_trace::instrument::{Instrumented, Instrument};
use tower_service::Service;
use std::fmt;

#[derive(Clone, Debug)]
pub struct InstrumentedService<T> {
    inner: T,
    span: tokio_trace::Span,
}

pub trait InstrumentableService: Service + Sized {
    fn instrument(self, span: tokio_trace::Span) -> InstrumentedService<Self> {
        InstrumentedService {
            inner: self,
            span,
        }
    }
}

impl<T: Service> Service for InstrumentedService<T>
where
    // TODO: it would be nice to do more for HTTP services...
    T::Request: fmt::Debug + Clone + Send + Sync + 'static,
{
    type Request = T::Request;
    type Response = T::Response;
    type Future = Instrumented<T::Future>;
    type Error = T::Error;

    fn poll_ready(&mut self) -> futures::Poll<(), Self::Error> {
        let span = &self.span;
        let inner = &mut self.inner;
        span.enter(|| { inner.poll_ready() })
    }

    fn call(&mut self, request: Self::Request) -> Self::Future {
        let span = &self.span;
        let inner = &mut self.inner;
        span.enter(|| {
            let request_span = span!("request", request = request.clone());
            request_span.clone().enter(move || {
                inner.call(request).instrument(request_span)
            })
        })
    }
}
