//! Middleware which instruments each request passing through a service with a new span.
use futures::{future::Future, Async, Poll};
use std::marker::PhantomData;
use tracing_futures::Instrument;

#[derive(Debug)]
pub struct Service<S, R, F = fn(&R) -> tracing::Span>
where
    S: tower_service::Service<R>,
    F: FnMut(&R) -> tracing::Span,
{
    f: F,
    inner: S,
    _p: PhantomData<fn(R)>,
}

#[cfg(feature = "tower-layer")]
pub use self::layer::*;

#[cfg(feature = "tower-layer")]
mod layer {
    use super::*;

    #[derive(Debug)]
    pub struct Layer<R, F = fn(&R) -> tracing::Span>
    where
        F: FnMut(&R) -> tracing::Span + Clone,
    {
        f: F,
        _p: PhantomData<fn(R)>,
    }

    pub fn layer<R, F>(f: F) -> Layer<R, F>
    where
        F: FnMut(&R) -> tracing::Span + Clone,
    {
        Layer { f, _p: PhantomData }
    }

    // === impl Layer ===
    impl<S, R, F> tower_layer::Layer<S> for Layer<R, F>
    where
        S: tower_service::Service<R>,
        F: FnMut(&R) -> tracing::Span + Clone,
    {
        type Service = Service<S, R, F>;

        fn layer(&self, service: S) -> Self::Service {
            Service::new(service, self.f.clone())
        }
    }

    impl<R, F> Clone for Layer<R, F>
    where
        F: FnMut(&R) -> tracing::Span + Clone,
    {
        fn clone(&self) -> Self {
            Self {
                f: self.f.clone(),
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

    pub type MakeFuture<S, R, F> = MakeService<S, R, Option<F>>;

    #[derive(Debug)]
    pub struct MakeService<S, R, F = fn(&R) -> tracing::Span> {
        f: F,
        inner: S,
        _p: PhantomData<fn(R)>,
    }

    #[cfg(feature = "tower-layer")]
    #[derive(Debug)]
    pub struct MakeLayer<R, T, F = fn(&R) -> tracing::Span>
    where
        F: FnMut(&R) -> tracing::Span + Clone,
    {
        f: F,
        _p: PhantomData<fn(T, R)>,
    }

    #[cfg(feature = "tower-layer")]
    pub fn layer<R, T, F>(f: F) -> MakeLayer<R, T, F>
    where
        F: FnMut(&R) -> tracing::Span + Clone,
    {
        MakeLayer { f, _p: PhantomData }
    }

    // === impl MakeLayer ===

    #[cfg(feature = "tower-layer")]
    impl<S, R, F, T> tower_layer::Layer<S> for MakeLayer<R, T, F>
    where
        S: tower_util::MakeService<T, R>,
        F: FnMut(&R) -> tracing::Span + Clone,
    {
        type Service = MakeService<S, R, F>;

        fn layer(&self, inner: S) -> Self::Service {
            MakeService::new(inner, self.f.clone())
        }
    }

    #[cfg(feature = "tower-layer")]
    impl<R, T, F> Clone for MakeLayer<R, T, F>
    where
        F: FnMut(&R) -> tracing::Span + Clone,
    {
        fn clone(&self) -> Self {
            Self {
                f: self.f.clone(),
                _p: PhantomData,
            }
        }
    }

    // === impl MakeService ===

    impl<S, R, F, T> tower_service::Service<T> for MakeService<S, R, F>
    where
        S: tower_util::MakeService<T, R>,
        F: FnMut(&R) -> tracing::Span + Clone,
    {
        type Response = Service<S::Service, R, F>;
        type Error = S::MakeError;
        type Future = MakeFuture<S::Future, R, F>;

        fn poll_ready(&mut self) -> Poll<(), Self::Error> {
            self.inner.poll_ready()
        }

        fn call(&mut self, target: T) -> Self::Future {
            let inner = self.inner.make_service(target);
            let f = Some(self.f.clone());
            MakeService {
                f,
                inner,
                _p: PhantomData,
            }
        }
    }

    impl<S, R, F> MakeService<S, R, F>
    where
        F: FnMut(&R) -> tracing::Span,
    {
        pub fn new<T>(inner: S, f: F) -> Self
        where
            S: tower_util::MakeService<T, R>,
        {
            Self {
                f,
                inner,
                _p: PhantomData,
            }
        }
    }

    impl<S, R, F> Clone for MakeService<S, R, F>
    where
        F: FnMut(&R) -> tracing::Span + Clone,
        S: Clone,
    {
        fn clone(&self) -> Self {
            Self {
                f: self.f.clone(),
                inner: self.inner.clone(),
                _p: PhantomData,
            }
        }
    }

    impl<S, R, F> Future for MakeService<S, R, Option<F>>
    where
        S: Future,
        S::Item: tower_service::Service<R>,
        F: FnMut(&R) -> tracing::Span,
    {
        type Item = Service<S::Item, R, F>;
        type Error = S::Error;

        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            let inner = futures::try_ready!(self.inner.poll());
            let f = self.f.take().expect("polled after ready");
            Ok(Async::Ready(Service {
                inner,
                f,
                _p: PhantomData,
            }))
        }
    }

}

// === impl Service ===

impl<S, R, F> tower_service::Service<R> for Service<S, R, F>
where
    S: tower_service::Service<R>,
    F: FnMut(&R) -> tracing::Span,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = tracing_futures::Instrumented<S::Future>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.inner.poll_ready()
    }

    fn call(&mut self, request: R) -> Self::Future {
        let span = (self.f)(&request);
        let _enter = span.enter();
        self.inner.call(request).instrument(span.clone())
    }
}

impl<S, R, F> Clone for Service<S, R, F>
where
    S: tower_service::Service<R> + Clone,
    F: FnMut(&R) -> tracing::Span + Clone,
{
    fn clone(&self) -> Self {
        Service {
            f: self.f.clone(),
            inner: self.inner.clone(),
            _p: PhantomData,
        }
    }
}

impl<S, R, F> Service<S, R, F>
where
    S: tower_service::Service<R>,
    F: FnMut(&R) -> tracing::Span,
{
    pub fn new(inner: S, f: F) -> Self {
        Service {
            f,
            inner,
            _p: PhantomData,
        }
    }
}
