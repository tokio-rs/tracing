
pub struct RequestLayer<F> {
    f: F,
}

pub fn instrument_request<F, R>(f: F) -> Layer<F>
where
    F: Fn() -> tokio_trace::Span,
{
    Layer {
        f,
    }
}

impl<S, Request> tower_layer::Layer<S> for Layer<F>
where
    S: Service<Request>,
{
    type Service = Buffer<S, Request>;

    fn layer(&self, service: S) -> Self::Service {
        Buffer::with_executor(service, self.bound, &mut self.executor.clone())
    }
}
