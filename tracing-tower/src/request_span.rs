//! Middleware which instruments each request passing through a service with a new span.
use super::GetSpan;
use futures::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};
use tracing::Instrument;

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
#[cfg_attr(docsrs, doc(cfg(feature = "tower-layer")))]
pub use self::layer::*;

#[cfg(feature = "tower-layer")]
#[cfg_attr(docsrs, doc(cfg(feature = "tower-layer")))]
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

    // === impl Subscriber ===
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

#[cfg(feature = "tower-make")]
#[cfg_attr(docsrs, doc(cfg(feature = "tower-make")))]
pub use self::make::MakeService;

#[cfg(feature = "tower-make")]
#[cfg_attr(docsrs, doc(cfg(feature = "tower-make")))]
pub mod make {
    use super::*;
    use pin_project::pin_project;

    #[derive(Debug)]
    pub struct MakeService<S, R, G = fn(&R) -> tracing::Span> {
        get_span: G,
        inner: S,
        _p: PhantomData<fn(R)>,
    }

    #[cfg(feature = "tower-layer")]
    #[cfg_attr(docsrs, doc(cfg(feature = "tower-layer")))]
    #[derive(Debug)]
    pub struct MakeLayer<R, T, G = fn(&R) -> tracing::Span>
    where
        G: GetSpan<R> + Clone,
    {
        get_span: G,
        _p: PhantomData<fn(T, R)>,
    }

    #[pin_project]
    #[derive(Debug)]
    pub struct MakeFuture<F, R, G = fn(&R) -> tracing::Span> {
        get_span: Option<G>,
        #[pin]
        inner: F,
        _p: PhantomData<fn(R)>,
    }

    #[cfg(feature = "tower-layer")]
    #[cfg_attr(docsrs, doc(cfg(feature = "tower-layer")))]
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
    #[cfg_attr(docsrs, doc(cfg(feature = "tower-layer")))]
    impl<S, R, G, T> tower_layer::Layer<S> for MakeLayer<R, T, G>
    where
        S: tower_make::MakeService<T, R>,
        G: GetSpan<R> + Clone,
    {
        type Service = MakeService<S, R, G>;

        fn layer(&self, inner: S) -> Self::Service {
            MakeService::new(inner, self.get_span.clone())
        }
    }

    #[cfg(feature = "tower-layer")]
    #[cfg_attr(docsrs, doc(cfg(feature = "tower-layer")))]
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
        S: tower_make::MakeService<T, R>,
        G: GetSpan<R> + Clone,
    {
        type Response = Service<S::Service, R, G>;
        type Error = S::MakeError;
        type Future = MakeFuture<S::Future, R, G>;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.inner.poll_ready(cx)
        }

        fn call(&mut self, target: T) -> Self::Future {
            let inner = self.inner.make_service(target);
            let get_span = Some(self.get_span.clone());
            MakeFuture {
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
            S: tower_make::MakeService<T, R>,
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

    impl<F, R, G, S, E> Future for MakeFuture<F, R, G>
    where
        F: Future<Output = Result<S, E>>,
        S: tower_service::Service<R>,
        G: GetSpan<R> + Clone,
    {
        type Output = Result<Service<S, R, G>, E>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.project();
            let inner = futures::ready!(this.inner.poll(cx));
            let get_span = this.get_span.take().expect("polled after ready");
            Poll::Ready(inner.map(|inner| Service {
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
    type Future = tracing::instrument::Instrumented<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
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
