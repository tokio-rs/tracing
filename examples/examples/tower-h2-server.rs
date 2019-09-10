#![deny(rust_2018_idioms)]

use bytes::{Bytes, IntoBuf};
use futures::*;
use http::Request;
use tokio::executor::DefaultExecutor;
use tokio::net::TcpListener;
use tower_h2::{Body, RecvBody, Server};
use tower_service::Service;
use tracing_futures::Instrument;
use tracing_tower::InstrumentMake;

type Response = http::Response<RspBody>;

struct RspBody(Option<Bytes>);

impl RspBody {
    fn new(body: Bytes) -> Self {
        RspBody(Some(body))
    }

    fn empty() -> Self {
        RspBody(None)
    }
}

impl Body for RspBody {
    type Data = <Bytes as IntoBuf>::Buf;
    type Error = h2::Error;

    fn poll_data(&mut self) -> Poll<Option<Self::Data>, Self::Error> {
        let data = self.0.take().and_then(|b| {
            if b.is_empty() {
                None
            } else {
                Some(b.into_buf())
            }
        });
        Ok(Async::Ready(data))
    }

    fn poll_trailers(&mut self) -> Poll<Option<http::HeaderMap>, Self::Error> {
        Ok(None.into())
    }
}

const ROOT: &'static str = "/";

#[derive(Debug)]
struct Svc;
impl Service<Request<RecvBody>> for Svc {
    type Response = Response;
    type Error = h2::Error;
    type Future = future::FutureResult<Response, Self::Error>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(Async::Ready(()))
    }

    fn call(&mut self, req: Request<RecvBody>) -> Self::Future {
        tracing::trace!(message = "received request", request.headers = ?req.headers());
        let mut rsp = http::Response::builder();
        rsp.version(http::Version::HTTP_2);

        let uri = req.uri();
        let rsp = if uri.path() != ROOT {
            let body = RspBody::empty();
            tracing::warn!(rsp.error = %"unrecognized path", request.path = ?uri.path());
            rsp.status(404).body(body).unwrap()
        } else {
            let body = RspBody::new("heyo!".into());
            rsp.status(200).body(body).unwrap()
        };

        tracing::debug!(rsp.status = %rsp.status(), message = "sending response...");
        future::ok(rsp)
    }
}

#[derive(Debug)]
struct NewSvc;
impl tower_service::Service<()> for NewSvc {
    type Response = Svc;
    type Error = ::std::io::Error;
    type Future = future::FutureResult<Svc, Self::Error>;
    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(Async::Ready(()))
    }

    fn call(&mut self, _target: ()) -> Self::Future {
        future::ok(Svc)
    }
}

fn main() {
    // Set the default subscriber to record all traces emitted by this example
    // and by the `tracing_tower` library's helpers.
    use tracing_subscriber::filter::{Directive, Filter};
    let filter = Filter::from_default_env()
        .add_directive("tower_h2_server=trace".parse::<Directive>().unwrap())
        .add_directive("tracing_tower=trace".parse::<Directive>().unwrap());
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_filter(filter)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    let addr = "[::1]:8888".parse().unwrap();
    let bind = TcpListener::bind(&addr).expect("bind");

    // Construct a span for the server task, annotated with the listening IP
    // address and port.
    let span = tracing::trace_span!("server", ip = %addr.ip(), port = addr.port());

    let server = lazy(|| {
        let executor = DefaultExecutor::current();

        // Enrich the `MakeService` with a wrapper so that each request is
        // traced with its own span.
        let new_svc = NewSvc.with_traced_requests(tracing_tower::http::debug_request);
        let h2 = Server::new(new_svc, Default::default(), executor);

        tracing::info!("listening");

        bind.incoming()
            .fold(h2, |mut h2, sock| {
                // Construct a new span for each accepted connection.
                let addr = sock.peer_addr().expect("can't get addr");
                let span = tracing::trace_span!("conn", ip = %addr.ip(), port = addr.port());
                let _enter = span.enter();

                tracing::debug!("accepted connection");

                if let Err(e) = sock.set_nodelay(true) {
                    return Err(e);
                }

                let serve = h2
                    .serve(sock)
                    .map_err(|error| tracing::error!(message = "h2 error", %error))
                    .map(|_| {
                        tracing::trace!("finished serving connection");
                    })
                    .instrument(span.clone());
                tokio::spawn(serve);

                Ok(h2)
            })
            .map_err(|error| tracing::error!(message = "serve error", %error))
            .map(|_| {})
    })
    .instrument(span);

    tokio::run(server);
}
