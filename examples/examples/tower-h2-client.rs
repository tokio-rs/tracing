#![deny(rust_2018_idioms)]

use bytes::Bytes;
use futures::*;
use h2::Reason;
use http::{Request, Response};
use std::net::SocketAddr;
use string::{String, TryFrom};
use tokio::net::TcpStream;
use tokio::runtime::{Runtime, TaskExecutor};
use tower_h2::client::Connect;
use tower_h2::{Body, RecvBody};
use tower_service::Service;
use tower_util::MakeService;
use tracing_futures::Instrument;
use tracing_tower::InstrumentableService;

pub struct Conn(SocketAddr);

fn main() {
    use tracing_subscriber::{fmt, EnvFilter};
    // Set the default subscriber to record all traces emitted by this example
    // and by the `tracing_tower` library's helpers.
    let filter = EnvFilter::from_default_env()
        .add_directive("tower_h2_client=trace".parse().unwrap())
        .add_directive("tracing_tower=trace".parse().unwrap());

    fmt::Subscriber::builder()
        .with_env_filter(filter)
        .init();

    let mut rt = Runtime::new().unwrap();
    let executor = rt.executor();

    let addr = "[::1]:8888".parse().unwrap();

    impl Service<()> for Conn {
        type Response = TcpStream;
        type Error = ::std::io::Error;
        type Future = Box<dyn Future<Item = TcpStream, Error = ::std::io::Error> + Send>;

        fn poll_ready(&mut self) -> Poll<(), Self::Error> {
            Ok(().into())
        }

        fn call(&mut self, _: ()) -> Self::Future {
            tracing::debug!("connecting...");

            let c = TcpStream::connect(&self.0)
                .and_then(|tcp| {
                    tcp.set_nodelay(true)?;
                    tracing::info!("connected!");
                    Ok(tcp)
                })
                .map_err(|error| {
                    tracing::error!(%error);
                    error
                });
            Box::new(c)
        }
    }

    let conn = Conn(addr).trace_requests(tracing::debug_span!("connect", remote = %addr));
    let mut h2 = Connect::new(conn, Default::default(), executor.clone());

    let req_span: fn(&http::Request<_>) -> tracing::Span = |req| {
        let span = tracing::trace_span!(
            "request",
            req.method = ?req.method(),
            req.path = ?req.uri().path(),
        );
        {
            // TODO: this is a workaround because tracing-fmt doesn't honor
            // overridden request parents.
            let _enter = span.enter();
            tracing::trace!(parent: &span, "sending request...");
        }
        span
    };

    let done = h2
        .make_service(())
        .map_err(|_| Reason::REFUSED_STREAM.into())
        .and_then(move |h2| {
            let h2 = h2.trace_requests(req_span);
            Serial {
                h2,
                count: 10,
                pending: None,
            }
        })
        .map(|_| println!("done"))
        .map_err(|e| println!("error: {:?}", e));

    rt.spawn(done);
    rt.shutdown_on_idle().wait().unwrap();
}

/// Avoids overflowing max concurrent streams
struct Serial {
    count: usize,
    h2: tracing_tower::request_span::Service<
        tower_h2::client::Connection<TcpStream, TaskExecutor, tower_h2::NoBody>,
        http::Request<tower_h2::NoBody>,
    >,
    pending: Option<Box<dyn Future<Item = (), Error = tower_h2::client::Error> + Send>>,
}

impl Future for Serial {
    type Item = ();
    type Error = tower_h2::client::Error;

    fn poll(&mut self) -> Poll<(), Self::Error> {
        loop {
            if let Some(mut fut) = self.pending.take() {
                if fut.poll()?.is_not_ready() {
                    self.pending = Some(fut);
                    return Ok(Async::NotReady);
                }
            }

            if self.count == 0 {
                return Ok(Async::Ready(()));
            }
            self.count -= 1;
            let mut fut = {
                let span = tracing::debug_span!("serial", req.number = self.count);
                let _enter = span.enter();
                self.h2
                    .call(mkreq())
                    .and_then(move |rsp| read_response(rsp).map_err(Into::into))
                    .instrument(span.clone())
            };

            if fut.poll()?.is_not_ready() {
                self.pending = Some(Box::new(fut));
                return Ok(Async::NotReady);
            }
        }
    }
}

fn mkreq() -> Request<tower_h2::NoBody> {
    Request::builder()
        .method("GET")
        .uri("http://[::1]:8888/")
        .version(http::Version::HTTP_2)
        .body(tower_h2::NoBody)
        .unwrap()
}

fn read_response(rsp: Response<RecvBody>) -> tracing_futures::Instrumented<ReadResponse> {
    let span = tracing::trace_span!("response");
    let f = {
        let _enter = span.enter();
        let (parts, body) = rsp.into_parts();
        tracing::debug!(rsp.status = %parts.status);
        ReadResponse { body }
    };
    f.instrument(span)
}

struct ReadResponse {
    body: RecvBody,
}

impl Future for ReadResponse {
    type Item = ();
    type Error = tower_h2::client::Error;
    fn poll(&mut self) -> Poll<(), Self::Error> {
        loop {
            match try_ready!(self.body.poll_data()) {
                None => return Ok(Async::Ready(())),
                Some(b) => {
                    let b: Bytes = b.into();
                    {
                        let s = String::try_from(b).expect("decode utf8 string");
                        tracing::trace!(rsp.body = &*s);
                    }
                }
            }
        }
    }
}
