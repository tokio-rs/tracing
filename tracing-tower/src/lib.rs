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
    Request: fmt::Debug,
{
    fn instrument(self, span: tracing::Span) -> InstrumentedService<Self, Request> {
        let req_span: fn(&Request) -> tracing::Span =
            |request| tracing::span!(Level::TRACE, "request", ?request);
        let svc = request_span::Service::new(self, req_span);
        service_span::Service::new(svc, span)
    }
}
