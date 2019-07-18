use std::fmt;
use tracing::Level;
use tracing_futures::{Instrument, Instrumented};

#[derive(Clone, Debug)]
pub struct Service<S> {
    pub(crate) inner: S,
    pub(crate) span: tracing::Span,
}

impl<T, Request> tower_service::Service<Request> for Service<T>
where
    T: tower_service::Service<Request>,
    Request: fmt::Debug + Clone + Send + Sync + 'static,
{
    type Response = T::Response;
    type Error = T::Error;
    type Future = Instrumented<T::Future>;

    fn poll_ready(&mut self) -> futures::Poll<(), Self::Error> {
        let _enter = self.span.enter();
        self.inner.poll_ready()
    }

    fn call(&mut self, request: Request) -> Self::Future {
        // TODO: custom `Value` impls for `http` types would be nice...
        let span = tracing::span!(parent: &self.span, Level::TRACE, "request", ?request);
        let _enter = span.enter();
        self.inner.call(request).instrument(span.clone())
    }
}
