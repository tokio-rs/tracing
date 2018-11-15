extern crate http;
extern crate tower_service;
#[macro_use]
extern crate tokio_trace;
extern crate futures;
extern crate tokio_trace_futures;

use std::marker::PhantomData;

use futures::{Future, Poll};
use tokio_trace::field::Value;
use tokio_trace_futures::{Instrument, Instrumented};
use tower_service::{MakeService, Service};

#[derive(Debug)]
pub struct InstrumentedHttpService<T> {
    inner: T,
    span: tokio_trace::Span,
}

impl<T> InstrumentedHttpService<T> {
    pub fn new(inner: T, span: tokio_trace::Span) -> Self {
        Self { inner, span }
    }

    pub fn in_current(inner: T) -> Self {
        Self::new(inner, tokio_trace::Span::current())
    }
}

#[derive(Debug)]
pub struct InstrumentedMakeService<T, B> {
    inner: T,
    _p: PhantomData<fn() -> B>,
}

impl<T, B> InstrumentedMakeService<T, B> {
    pub fn new<Target>(inner: T) -> Self
    where
        T: MakeService<Target, http::Request<B>>,
    {
        Self {
            inner,
            _p: PhantomData,
        }
    }
}

impl<T, Target, B> Service<Target> for InstrumentedMakeService<T, B>
where
    T: MakeService<Target, http::Request<B>>,
{
    type Response = InstrumentedHttpService<T::Service>;
    type Error = T::MakeError;
    type Future = InstrumentedMakeServiceFuture<T::Future>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.inner.poll_ready()
    }

    fn call(&mut self, req: Target) -> Self::Future {
        let span = tokio_trace::Span::current();
        let inner = self.inner.make_service(req);
        InstrumentedMakeServiceFuture { inner, span }
    }
}

pub struct InstrumentedMakeServiceFuture<T> {
    inner: T,
    span: tokio_trace::Span,
}

impl<T> Future for InstrumentedMakeServiceFuture<T>
where
    T: Future,
{
    type Item = InstrumentedHttpService<T::Item>;
    type Error = T::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let span = &mut self.span;
        let inner = &mut self.inner;
        span.enter(move || {
            inner
                .poll()
                .map(|ready| ready.map(|svc| InstrumentedHttpService::in_current(svc)))
        })
    }
}

impl<T, B> Service<http::Request<B>> for InstrumentedHttpService<T>
where
    T: Service<http::Request<B>>,
{
    type Response = T::Response;
    type Future = Instrumented<T::Future>;
    type Error = T::Error;

    fn poll_ready(&mut self) -> futures::Poll<(), Self::Error> {
        let span = &mut self.span;
        let inner = &mut self.inner;
        span.enter(move || inner.poll_ready())
    }

    fn call(&mut self, request: http::Request<B>) -> Self::Future {
        let span = &mut self.span;
        let inner = &mut self.inner;
        span.enter(move || {
            span!(
                "request",
                // TODO: custom `Value` impls for `http` types would be nicer
                // than just sticking these in `debug`s...
                method = &Value::debug(request.method()),
                version = &Value::debug(request.version()),
                uri = &Value::debug(request.uri()),
                headers = &Value::debug(request.headers())
            ).enter(move || inner.call(request).in_current_span())
        })
    }
}
