//! Middleware which instruments a service with a span entered when that service
//! is called.
use crate::GetSpan;
use futures::{future::Future, Async, Poll};
use std::marker::PhantomData;

#[derive(Debug)]
pub struct Service<S> {
    inner: S,
    span: tracing::Span,
}

#[cfg(feature = "tower-layer")]
pub use self::layer::*;

#[cfg(feature = "tower-util")]
pub use self::make::MakeService;

#[cfg(feature = "tower-layer")]
mod layer {
    use super::*;

    #[derive(Debug)]
    pub struct Layer<S, R, G = fn(&S) -> tracing::Span>
    where
        G: GetSpan<S>,
        S: tower_service::Service<R>,
    {
        get_span: G,
        _p: PhantomData<fn(S, R)>,
    }

    pub fn layer<S, R, G>(get_span: G) -> Layer<S, R, G>
    where
        G: GetSpan<S>,
        S: tower_service::Service<R>,
    {
        Layer {
            get_span,
            _p: PhantomData,
        }
    }

    // === impl Layer ===

    impl<S, R, G> tower_layer::Layer<S> for Layer<S, R, G>
    where
        G: GetSpan<S>,
        S: tower_service::Service<R>,
    {
        type Service = Service<S>;

        fn layer(&self, inner: S) -> Self::Service {
            let span = self.get_span.span_for(&inner);
            Service { span, inner }
        }
    }

    impl<S, R, G> Clone for Layer<S, R, G>
    where
        G: GetSpan<S> + Clone,
        S: tower_service::Service<R>,
    {
        fn clone(&self) -> Self {
            Self {
                get_span: self.get_span.clone(),
                _p: PhantomData,
            }
        }
    }
}

#[cfg(feature = "tower-util")]
pub mod make {
    use super::*;

    #[derive(Debug)]
    pub struct MakeService<M, T, R, G = fn(&T) -> tracing::Span>
    where
        G: GetSpan<T>,
    {
        get_span: G,
        inner: M,
        _p: PhantomData<fn(T, R)>,
    }

    #[derive(Debug)]
    pub struct MakeFuture<F> {
        inner: F,
        span: Option<tracing::Span>,
    }

    #[derive(Debug)]
    pub struct MakeLayer<T, R, G = fn(&T) -> tracing::Span>
    where
        G: GetSpan<T> + Clone,
    {
        get_span: G,
        _p: PhantomData<fn(T, R)>,
    }

    #[cfg(feature = "tower-layer")]
    pub fn layer<T, R, G>(get_span: G) -> MakeLayer<T, R, G>
    where
        G: GetSpan<T> + Clone,
    {
        MakeLayer {
            get_span,
            _p: PhantomData,
        }
    }

    // === impl MakeLayer ===

    #[cfg(feature = "tower-layer")]
    impl<M, T, R, G> tower_layer::Layer<M> for MakeLayer<T, R, G>
    where
        M: tower_util::MakeService<T, R>,
        G: GetSpan<T> + Clone,
    {
        type Service = MakeService<M, T, R, G>;

        fn layer(&self, inner: M) -> Self::Service {
            MakeService::new(inner, self.get_span.clone())
        }
    }

    #[cfg(feature = "tower-layer")]
    impl<T, R, G> Clone for MakeLayer<T, R, G>
    where
        G: GetSpan<T> + Clone,
    {
        fn clone(&self) -> Self {
            Self {
                get_span: self.get_span.clone(),
                _p: PhantomData,
            }
        }
    }

    // === impl MakeService ===

    impl<M, T, R, G> tower_service::Service<T> for MakeService<M, T, R, G>
    where
        M: tower_util::MakeService<T, R>,
        G: GetSpan<T>,
    {
        type Response = Service<M::Service>;
        type Error = M::MakeError;
        type Future = MakeFuture<M::Future>;

        fn poll_ready(&mut self) -> Poll<(), Self::Error> {
            self.inner.poll_ready()
        }

        fn call(&mut self, target: T) -> Self::Future {
            let span = self.get_span.span_for(&target);
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
            Ok(Async::Ready(Service::new(inner, span)))
        }
    }

    impl<M, T, R, G> MakeService<M, T, R, G>
    where
        M: tower_util::MakeService<T, R>,
        G: GetSpan<T>,
    {
        pub fn new(inner: M, get_span: G) -> Self {
            MakeService {
                get_span,
                inner,
                _p: PhantomData,
            }
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
        let _enter = self.span.enter();
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
