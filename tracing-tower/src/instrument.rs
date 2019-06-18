use tokio_trace_futures::{Instrument, Instrumented};

#[derive(Clone, Debug)]
pub struct Request<S, F> {
    f: F,
    inner: S,
}

impl<S, F, R> Service<R> for Request<S, F> {
    type Response = T::Response;
    type Error = Error;
    type Future = ResponseFuture<T::Future>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        // If the inner service has errored, then we error here.
        self.tx.poll_ready().map_err(|_| self.get_worker_error())
    }

    fn call(&mut self, request: Request) -> Self::Future {
}
