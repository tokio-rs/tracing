extern crate http;
extern crate tower;
extern crate tower_service;
#[macro_use]
extern crate tracing;
extern crate futures;
extern crate tracing_futures;

use std::marker::PhantomData;

use futures::{Async, Future, Poll};
use tower::MakeService;
use tower_service::Service;
use tracing::{field, Span};
use tracing_futures::{Instrument, Instrumented};

#[derive(Debug, Clone)]
pub struct InstrumentedHttpService<T> {
    inner: T,
    span: Option<Span>,
}

impl<T> InstrumentedHttpService<T> {
    pub fn new(inner: T) -> Self {
        Self { inner, span: None }
    }

    pub fn with_span(inner: T, span: impl Into<Option<Span>>) -> Self {
        Self {
            inner,
            span: span.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstrumentedMakeService<T, B> {
    inner: T,
    span: Option<Span>,
    _p: PhantomData<fn() -> B>,
}

impl<T, B> InstrumentedMakeService<T, B> {
    pub fn new<Target>(inner: T) -> Self
    where
        T: MakeService<Target, http::Request<B>>,
    {
        Self {
            inner,
            span: None,
            _p: PhantomData,
        }
    }

    pub fn with_span<Target>(inner: T, span: impl Into<Option<Span>>) -> Self
    where
        T: MakeService<Target, http::Request<B>>,
    {
        Self {
            inner,
            span: span.into(),
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
        if let Some(span) = self.span.as_ref() {
            let _enter = span.enter();
            self.inner.poll_ready()
        } else {
            self.inner.poll_ready()
        }
    }

    fn call(&mut self, req: Target) -> Self::Future {
        let span = self.span.clone();
        let inner = self.inner.make_service(req);
        InstrumentedMakeServiceFuture { inner, span }
    }
}

pub struct InstrumentedMakeServiceFuture<T> {
    inner: T,
    span: Option<Span>,
}

impl<T> Future for InstrumentedMakeServiceFuture<T>
where
    T: Future,
{
    type Item = InstrumentedHttpService<T::Item>;
    type Error = T::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.inner.poll()? {
            Async::Ready(svc) => {
                let svc = InstrumentedHttpService::with_span(
                    svc,
                    self.span
                        .take()
                        .expect("Future was polled after Ready was returned."),
                );
                Ok(Async::Ready(svc))
            }
            Async::NotReady => Ok(Async::NotReady),
        }
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
        if let Some(span) = self.span.as_ref() {
            let _enter = span.enter();
            self.inner.poll_ready()
        } else {
            self.inner.poll_ready()
        }
    }

    fn call(&mut self, request: http::Request<B>) -> Self::Future {
        let span = trace_span!(
            parent: self.span.as_ref().and_then(Span::id),
            "request",
            // TODO: custom `Value` impls for `http` types would be nicer
            // than just sticking these in `debug`s...
            method = &field::debug(request.method()),
            version = &field::debug(request.version()),
            uri = &field::debug(request.uri()),
            headers = &field::debug(request.headers())
        );
        let _enter = span.enter();
        self.inner.call(request).instrument(span.clone())
    }
}
