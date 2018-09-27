extern crate bytes;
extern crate futures;
extern crate h2;
extern crate http;
extern crate tokio;
extern crate tokio_current_thread;
#[macro_use]
extern crate tokio_trace;
extern crate tokio_trace_tower_http;
extern crate tower_h2;
extern crate tower_service;

use bytes::Bytes;
use futures::*;
use http::Request;
use tokio::{
    net::TcpListener,
    runtime::current_thread::Runtime,
};
use tokio_current_thread::TaskExecutor;
use tower_h2::{Body, Server, RecvBody};
use tower_service::{NewService, Service};
use tokio_trace::{
    Level,
    instrument::Instrument,
};

#[path = "../../tokio-trace/examples/sloggish_subscriber.rs"]
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
        let data = self.0
            .take()
            .and_then(|b| if b.is_empty() { None } else { Some(b) });
        Ok(Async::Ready(data))
    }
}

const ROOT: &'static str = "/";

#[derive(Debug)]
struct Svc;
impl Service for Svc {
    type Request = Request<RecvBody>;
    type Response = Response;
    type Error = h2::Error;
    type Future = future::FutureResult<Response, Self::Error>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(Async::Ready(()))
    }

    fn call(&mut self, req: Self::Request) -> Self::Future {
        event!(Level::Debug, {}, "received request");
        let mut rsp = http::Response::builder();
        rsp.version(http::Version::HTTP_2);

        let uri = req.uri();
        if uri.path() != ROOT {
            let body = RspBody::empty();
            let rsp = rsp.status(404).body(body).unwrap();
            event!(Level::Warn, { status_code = 404, path = uri.path() }, "unrecognized URI");
            return future::ok(rsp);
        }

        let body = RspBody::new("heyo!".into());
        let rsp = rsp.status(200).body(body).unwrap();
        future::ok(rsp)
    }
}

#[derive(Debug)]
struct NewSvc;
impl NewService for NewSvc {
    type Request = Request<RecvBody>;
    type Response = Response;
    type Error = h2::Error;
    type InitError = ::std::io::Error;
    type Service = Svc;
    type Future = future::FutureResult<Svc, Self::InitError>;

   fn new_service(&self) -> Self::Future {
        future::ok(Svc)
    }
}

fn main() {
    tokio_trace::Dispatcher::builder()
        .add_subscriber(SloggishSubscriber::new(2))
        .init();

    let mut rt = Runtime::new().unwrap();

    let addr = "[::1]:8888".parse().unwrap();
    let bind = TcpListener::bind(&addr).expect("bind");

    let server_span = span!("serve", local_ip = addr.ip(), local_port = addr.port());
    server_span.clone().enter(move || {
        let new_svc = tokio_trace_tower_http::InstrumentedNewService::new(NewSvc);
        let h2 = Server::new(new_svc, Default::default(), TaskExecutor::current());

        let serve = bind.incoming()
            .fold(h2, |h2, sock| {
                let addr = sock.peer_addr().expect("can't get addr");
                let conn_span = span!("conn", remote_ip = addr.ip(), remote_port = addr.port());
                conn_span.clone().enter(|| {
                    if let Err(e) = sock.set_nodelay(true) {
                        return Err(e);
                    }

                    event!(Level::Info, {}, "accepted connection");

                    let serve = h2.serve(sock)
                        .map_err(|e| event!(Level::Error, { }, "error {:?}", e))
                        .map(|_| event!(Level::Debug, { }, "response finished"))
                        .instrument(conn_span);
                    TaskExecutor::current().spawn_local(Box::new(serve)).unwrap();

                    Ok(h2)
                })

            })
            .map_err(|e| event!(Level::Error, {}, "serve error {:?}", e))
            .map(|_| {})
            ;

        rt.block_on(serve).unwrap()
    })
}
