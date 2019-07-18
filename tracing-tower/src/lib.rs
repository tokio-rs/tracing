use std::fmt;
use tower_service::Service;
use tracing::Level;

pub mod request_span;
pub mod service_span;

#[cfg(feature = "http")]
pub mod http;

pub type InstrumentedService<S, R> = service_span::Service<request_span::Service<S, R>>;

pub trait InstrumentableService<Request>
where
    Self: Service<Request> + Sized,
{
    fn instrument<G>(self, svc_span: G) -> InstrumentedService<Self, Request>
    where
        G: GetSpan<Self>,
        Request: fmt::Debug,
    {
        let req_span: fn(&Request) -> tracing::Span =
            |request| tracing::span!(Level::TRACE, "request", ?request);
        let svc_span = svc_span.span_for(&self);
        self.instrument_request(req_span)
            .instrument_service(svc_span)
    }

    fn instrument_request<G>(self, get_span: G) -> request_span::Service<Self, Request, G>
    where
        G: GetSpan<Request> + Clone,
    {
        request_span::Service::new(self, get_span)
    }

    fn instrument_service<G>(self, get_span: G) -> service_span::Service<Self>
    where
        G: GetSpan<Self>,
    {
        let span = get_span.span_for(&self);
        service_span::Service::new(self, span)
    }
}

impl<S, R> InstrumentableService<R> for S where S: Service<R> + Sized {}

pub trait GetSpan<T>: crate::sealed::Sealed<T> {
    fn span_for(&self, target: &T) -> tracing::Span;
}

impl<T, F> crate::sealed::Sealed<T> for F where F: Fn(&T) -> tracing::Span {}

impl<T, F> GetSpan<T> for F
where
    F: Fn(&T) -> tracing::Span,
{
    #[inline]
    fn span_for(&self, target: &T) -> tracing::Span {
        (self)(target)
    }
}

impl<T> crate::sealed::Sealed<T> for tracing::Span {}

impl<T> GetSpan<T> for tracing::Span {
    #[inline]
    fn span_for(&self, _: &T) -> tracing::Span {
        self.clone()
    }
}

mod sealed {
    pub trait Sealed<T = ()> {}
}
