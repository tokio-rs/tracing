//! Middleware which instruments each request passing through a service with a new span.
use super::GetSpan;
use futures::{future::Future, Async, Poll};
use std::marker::PhantomData;
use tracing_futures::Instrument;

#[derive(Debug)]
pub struct Service<S, R, G = fn(&R) -> tracing::Span>
where
    S: tower_service::Service<R>,
    G: GetSpan<R>,
{
    get_span: G,
    inner: S,
    _p: PhantomData<fn(R)>,
}

#[cfg(feature = "tower-layer")]
pub use self::layer::*;

#[cfg(feature = "tower-layer")]
mod layer {
    use super::*;

    #[derive(Debug)]
    pub struct Layer<R, G = fn(&R) -> tracing::Span>
    where
        G: GetSpan<R> + Clone,
    {
        get_span: G,
        _p: PhantomData<fn(R)>,
    }

    pub fn layer<R, G>(get_span: G) -> Layer<R, G>
    where
        G: GetSpan<R> + Clone,
    {
        Layer {
            get_span,
            _p: PhantomData,
        }
    }

    // === impl Layer ===
    impl<S, R, G> tower_layer::Layer<S> for Layer<R, G>
    where
        S: tower_service::Service<R>,
        G: GetSpan<R> + Clone,
    {
        type Service = Service<S, R, G>;

        fn layer(&self, service: S) -> Self::Service {
            Service::new(service, self.get_span.clone())
        }
    }

    impl<R, G> Clone for Layer<R, G>
    where
        G: GetSpan<R> + Clone,
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
pub use self::make::MakeService;

#[cfg(feature = "tower-util")]
pub mod make {
    use super::*;

    pub type MakeFuture<S, R, G> = MakeService<S, R, Option<G>>;

    #[derive(Debug)]
    pub struct MakeService<S, R, G = fn(&R) -> tracing::Span> {
        get_span: G,
        inner: S,
        _p: PhantomData<fn(R)>,
    }

    #[cfg(feature = "tower-layer")]
    #[derive(Debug)]
    pub struct MakeLayer<R, T, G = fn(&R) -> tracing::Span>
    where
        G: GetSpan<R> + Clone,
    {
        get_span: G,
        _p: PhantomData<fn(T, R)>,
    }

    #[cfg(feature = "tower-layer")]
    pub fn layer<R, T, G>(get_span: G) -> MakeLayer<R, T, G>
    where
        G: GetSpan<R> + Clone,
    {
        MakeLayer {
            get_span,
            _p: PhantomData,
        }
    }

    // === impl MakeLayer ===

    #[cfg(feature = "tower-layer")]
    impl<S, R, G, T> tower_layer::Layer<S> for MakeLayer<R, T, G>
    where
        S: tower_util::MakeService<T, R>,
        G: GetSpan<R> + Clone,
    {
        type Service = MakeService<S, R, G>;

        fn layer(&self, inner: S) -> Self::Service {
            MakeService::new(inner, self.get_span.clone())
        }
    }

    #[cfg(feature = "tower-layer")]
    impl<R, T, G> Clone for MakeLayer<R, T, G>
    where
        G: GetSpan<R> + Clone,
    {
        fn clone(&self) -> Self {
            Self {
                get_span: self.get_span.clone(),
                _p: PhantomData,
            }
        }
    }

    // === impl MakeService ===

    impl<S, R, G, T> tower_service::Service<T> for MakeService<S, R, G>
    where
        S: tower_util::MakeService<T, R>,
        G: GetSpan<R> + Clone,
    {
        type Response = Service<S::Service, R, G>;
        type Error = S::MakeError;
        type Future = MakeFuture<S::Future, R, G>;

        fn poll_ready(&mut self) -> Poll<(), Self::Error> {
            self.inner.poll_ready()
        }

        fn call(&mut self, target: T) -> Self::Future {
            let inner = self.inner.make_service(target);
            let get_span = Some(self.get_span.clone());
            MakeService {
                get_span,
                inner,
                _p: PhantomData,
            }
        }
    }

    impl<S, R, G> MakeService<S, R, G>
    where
        G: GetSpan<R> + Clone,
    {
        pub fn new<T>(inner: S, get_span: G) -> Self
        where
            S: tower_util::MakeService<T, R>,
        {
            Self {
                get_span,
                inner,
                _p: PhantomData,
            }
        }
    }

    impl<S, R, G> Clone for MakeService<S, R, G>
    where
        G: GetSpan<R> + Clone,
        S: Clone,
    {
        fn clone(&self) -> Self {
            Self {
                get_span: self.get_span.clone(),
                inner: self.inner.clone(),
                _p: PhantomData,
            }
        }
    }

    impl<S, R, G> Future for MakeService<S, R, Option<G>>
    where
        S: Future,
        S::Item: tower_service::Service<R>,
        G: GetSpan<R> + Clone,
    {
        type Item = Service<S::Item, R, G>;
        type Error = S::Error;

        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            let inner = futures::try_ready!(self.inner.poll());
            let get_span = self.get_span.take().expect("polled after ready");
            Ok(Async::Ready(Service {
                inner,
                get_span,
                _p: PhantomData,
            }))
        }
    }
}

// === impl Service ===

impl<S, R, G> tower_service::Service<R> for Service<S, R, G>
where
    S: tower_service::Service<R>,
    G: GetSpan<R> + Clone,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = tracing_futures::Instrumented<S::Future>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.inner.poll_ready()
    }

    fn call(&mut self, request: R) -> Self::Future {
        let span = self.get_span.span_for(&request);
        let _enter = span.enter();
        self.inner.call(request).instrument(span.clone())
    }
}

impl<S, R, G> Clone for Service<S, R, G>
where
    S: tower_service::Service<R> + Clone,
    G: GetSpan<R> + Clone,
{
    fn clone(&self) -> Self {
        Service {
            get_span: self.get_span.clone(),
            inner: self.inner.clone(),
            _p: PhantomData,
        }
    }
}

impl<S, R, G> Service<S, R, G>
where
    S: tower_service::Service<R>,
    G: GetSpan<R> + Clone,
{
    pub fn new(inner: S, get_span: G) -> Self {
        Service {
            get_span,
            inner,
            _p: PhantomData,
        }
    }
}
