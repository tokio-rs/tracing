extern crate bytes;
extern crate futures;
extern crate h2;
extern crate http;
extern crate tokio;
#[macro_use]
extern crate tokio_trace;
extern crate env_logger;
extern crate tokio_trace_futures;
extern crate tokio_trace_log;
extern crate tokio_trace_subscriber;
extern crate tokio_trace_tower_http;
extern crate tower_h2;
extern crate tower_service;

use bytes::Bytes;
use futures::*;
use http::Request;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use tokio_trace::{field::Value, Level};
use tokio_trace_futures::Instrument;
use tower_h2::{Body, RecvBody, Server};
use tower_service::{MakeService, Service};

#[path = "../../tokio-trace/examples/sloggish/sloggish_subscriber.rs"]
mod sloggish;
use self::sloggish::SloggishSubscriber;

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
    type Data = Bytes;

    fn is_end_stream(&self) -> bool {
        self.0.as_ref().map(|b| b.is_empty()).unwrap_or(false)
    }

    fn poll_data(&mut self) -> Poll<Option<Bytes>, h2::Error> {
        let data = self
            .0
            .take()
            .and_then(|b| if b.is_empty() { None } else { Some(b) });
        Ok(Async::Ready(data))
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
        event!(Level::Debug, {}, "received request");
        let mut rsp = http::Response::builder();
        rsp.version(http::Version::HTTP_2);

        let uri = req.uri();
        if uri.path() != ROOT {
            let body = RspBody::empty();
            let rsp = rsp.status(404).body(body).unwrap();
            event!(Level::Warn, { status_code = Value::display(404), path = Value::debug(uri.path()) }, "unrecognized URI");
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

    fn call(&mut self, target: ()) -> Self::Future {
        future::ok(Svc)
    }
}

fn main() {
    let subscriber = SloggishSubscriber::new(2);

    tokio_trace::Dispatch::new(subscriber).as_default(|| {
        let mut rt = Runtime::new().unwrap();
        let reactor = rt.executor();

        let addr = "[::1]:8888".parse().unwrap();
        let bind = TcpListener::bind(&addr).expect("bind");

        span!(
            "serve",
            local_ip = Value::debug(addr.ip()),
            local_port = addr.port() as u64
        ).enter(move || {
            let new_svc = tokio_trace_tower_http::InstrumentedMakeService::new(NewSvc);
            let h2 = Server::new(new_svc, Default::default(), reactor.clone());

            let serve = bind
                .incoming()
                .fold((h2, reactor), |(mut h2, reactor), sock| {
                    let addr = sock.peer_addr().expect("can't get addr");
                    span!(
                        "conn",
                        remote_ip = Value::debug(addr.ip()),
                        remote_port = addr.port() as u64
                    ).enter(|| {
                        if let Err(e) = sock.set_nodelay(true) {
                            return Err(e);
                        }

                        event!(Level::Info, {}, "accepted connection");

                        let serve = h2
                            .serve(sock)
                            .map_err(|e| event!(Level::Error, {}, "error {:?}", e))
                            .and_then(|_| {
                                event!(Level::Debug, {}, "response finished");
                                future::ok(())
                            }).in_current_span();
                        reactor.spawn(Box::new(serve));

                        Ok((h2, reactor))
                    })
                }).map_err(|e| event!(Level::Error, {}, "serve error {:?}", e))
                .map(|_| {})
                .in_current_span();

            rt.spawn(serve);
            rt.shutdown_on_idle().wait().unwrap();
        });
    });
}
