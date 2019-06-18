// #[derive(Clone, Debug)]
// pub struct Service<S> {
//     span: tokio_trace::Span,
//     inner: S,
// }

// pub struct MakeService<F> {

// }

// // === impl Service ===

// impl<S, R> tower_service::Service<R> for Service<S>
// where
//     S: tower_service::Service<R>,
// {
//     type Response = S::Response;
//     type Error = S::Error;
//     type Future = S::Future;

//     fn poll_ready(&mut self) -> Poll<(), Self::Error> {
//         let _enter = self.span.enter();
//         self.inner.poll_ready()
//     }

//     fn call(&mut self, request: Request) -> Self::Future {
//         let _enter = self.span.enter();
//         self.inner.call()
//     }
// }
