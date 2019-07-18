use tower_service::Service;

pub mod request_span;
pub mod service_span;

#[deprecated(since = "0.0.1", note = "use `service_span::Service` instead")]
pub type InstrumentedService<R> = service_span::Service<R>;

pub trait InstrumentableService<Request>: Service<Request> + Sized {
    fn instrument(self, span: tracing::Span) -> service_span::Service<Self> {
        service_span::Service { inner: self, span }
    }
}
