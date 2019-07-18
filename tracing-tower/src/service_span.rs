//! Middleware which instruments a service with a span entered when that service
//! is called.
use futures::{future::Future, Async, Poll};
use std::marker::PhantomData;

#[derive(Debug)]
pub struct Service<S> {
    inner: S,
    span: tracing::Span,
}

#[derive(Debug)]
pub struct MakeService<M, T, R, F = fn(&T) -> tracing::Span>
where
    F: FnMut(&T) -> tracing::Span,
{
    f: F,
    inner: M,
    _p: PhantomData<fn(T, R)>,
}

#[derive(Debug)]
pub struct MakeFuture<F> {
    inner: F,
    span: Option<tracing::Span>,
}

#[derive(Debug)]
pub struct Layer<S, R, F = fn(&S) -> tracing::Span>
where
    F: Fn(&S) -> tracing::Span,
    S: tower_service::Service<R>,
{
    f: F,
    _p: PhantomData<fn(S, R)>,
}

#[derive(Debug)]
pub struct MakeLayer<T, R, F = fn(&T) -> tracing::Span>
where
    F: FnMut(&T) -> tracing::Span + Clone,
{
    f: F,
    _p: PhantomData<fn(T, R)>,
}

pub fn layer<S, R, F>(f: F) -> Layer<S, R, F>
where
    F: Fn(&S) -> tracing::Span,
    S: tower_service::Service<R>,
{
    Layer { f, _p: PhantomData }
}

pub fn make_layer<T, R, F>(f: F) -> MakeLayer<T, R, F>
where
    F: FnMut(&T) -> tracing::Span + Clone,
{
    MakeLayer { f, _p: PhantomData }
}

// === impl Layer ===

impl<S, R, F> tower_layer::Layer<S> for Layer<S, R, F>
where
    F: Fn(&S) -> tracing::Span,
    S: tower_service::Service<R>,
{
    type Service = Service<S>;

    fn layer(&self, inner: S) -> Self::Service {
        let span = (self.f)(&inner);
        Service { span, inner }
    }
}

impl<S, R, F> Clone for Layer<S, R, F>
where
    F: Fn(&S) -> tracing::Span + Clone,
    S: tower_service::Service<R>,
{
    fn clone(&self) -> Self {
        Self {
            f: self.f.clone(),
            _p: PhantomData,
        }
    }
}

// === impl MakeLayer ===

impl<F, T, M, R> tower_layer::Layer<M> for MakeLayer<T, R, F>
where
    M: tower_util::MakeService<T, R>,
    F: FnMut(&T) -> tracing::Span + Clone,
{
    type Service = MakeService<M, T, R, F>;

    fn layer(&self, inner: M) -> Self::Service {
        MakeService {
            f: self.f.clone(),
            inner,
            _p: PhantomData,
        }
    }
}

impl<T, R, F> Clone for MakeLayer<T, R, F>
where
    F: FnMut(&T) -> tracing::Span + Clone,
{
    fn clone(&self) -> Self {
        Self {
            f: self.f.clone(),
            _p: PhantomData,
        }
    }
}

// === impl Service ===

impl<S> Service<S> {
    pub fn new(inner: S, span: tracing::Span) -> Self {
        Self { span, inner }
    }
}

impl<S, R> tower_service::Service<R> for Service<S>
where
    S: tower_service::Service<R>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.inner.poll_ready()
    }

    fn call(&mut self, request: R) -> Self::Future {
        let _enter = self.span.enter();
        self.inner.call(request)
    }
}

impl<S> Clone for Service<S>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Service {
            span: self.span.clone(),
            inner: self.inner.clone(),
        }
    }
}

// === impl MakeService ===

impl<M, T, R, F> tower_service::Service<T> for MakeService<M, T, R, F>
where
    M: tower_util::MakeService<T, R>,
    F: FnMut(&T) -> tracing::Span + Clone,
{
    type Response = Service<M::Service>;
    type Error = M::MakeError;
    type Future = MakeFuture<M::Future>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.inner.poll_ready()
    }

    fn call(&mut self, target: T) -> Self::Future {
        let span = (self.f)(&target);
        let inner = self.inner.make_service(target);
        MakeFuture {
            span: Some(span),
            inner,
        }
    }
}

impl<F> Future for MakeFuture<F>
where
    F: Future,
{
    type Item = Service<F::Item>;
    type Error = F::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let inner = {
            let _guard = self.span.as_ref().map(tracing::Span::enter);
            futures::try_ready!(self.inner.poll())
        };

        let span = self.span.take().expect("polled after ready");
        Ok(Async::Ready(Service { inner, span }))
    }
}
