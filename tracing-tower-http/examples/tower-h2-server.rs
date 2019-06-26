extern crate bytes;
extern crate futures;
extern crate h2;
extern crate http;
extern crate tokio;
#[macro_use]
extern crate tracing;
extern crate env_logger;
extern crate tower_h2;
extern crate tower_service;
extern crate tracing_fmt;
extern crate tracing_futures;
extern crate tracing_tower_http;

use bytes::{Bytes, IntoBuf};
use futures::*;
use http::Request;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use tower_h2::{Body, RecvBody, Server};
use tower_service::Service;
use tracing::{field, Level};
use tracing_futures::Instrument;

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
        debug!("received request");
        let mut rsp = http::Response::builder();
        rsp.version(http::Version::HTTP_2);

        let uri = req.uri();
        if uri.path() != ROOT {
            let body = RspBody::empty();
            let rsp = rsp.status(404).body(body).unwrap();
            warn!({ status_code = field::display(404), path = field::debug(uri.path()) }, "unrecognized URI");
            return future::ok(rsp);
        }

        let body = RspBody::new("heyo!".into());
        let rsp = rsp.status(200).body(body).unwrap();
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
    let subscriber = tracing_fmt::FmtSubscriber::builder()
        .with_filter(tracing_fmt::filter::EnvFilter::from(
            "tower_h2_server=trace",
        ))
        .full()
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        let mut rt = Runtime::new().unwrap();
        let reactor = rt.executor();

        let addr = "[::1]:8888".parse().unwrap();
        let bind = TcpListener::bind(&addr).expect("bind");

        let serve_span = span!(
            Level::TRACE,
            "serve",
            local.ip = field::debug(addr.ip()),
            local.port = addr.port() as u64
        );
        let new_svc =
            tracing_tower_http::InstrumentedMakeService::with_span(NewSvc, serve_span.clone());
        let h2 = Server::new(new_svc, Default::default(), reactor.clone());

        let serve = bind
            .incoming()
            .fold((h2, reactor), |(mut h2, reactor), sock| {
                let addr = sock.peer_addr().expect("can't get addr");
                let conn_span = span!(
                    Level::TRACE,
                    "conn",
                    remote.ip = field::debug(addr.ip()),
                    remote.port = addr.port() as u64
                );
                let _enter = conn_span.enter();
                if let Err(e) = sock.set_nodelay(true) {
                    return Err(e);
                }

                info!("accepted connection");

                let serve = h2
                    .serve(sock)
                    .map_err(|e| error!("error {:?}", e))
                    .and_then(|_| {
                        debug!("response finished");
                        future::ok(())
                    })
                    .instrument(conn_span.clone());
                reactor.spawn(Box::new(serve));

                Ok((h2, reactor))
            })
            .map_err(|e| {
                error!("serve error {:?}", e);
            })
            .map(|_| {})
            .instrument(serve_span);

        rt.spawn(serve);
        rt.shutdown_on_idle().wait().unwrap();
    });
}
