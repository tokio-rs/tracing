use futures::{future, Future, Poll, Stream};
use hyper::{Request, Response, StatusCode};
use tokio_tcp::TcpListener;
use tower::{Service, ServiceBuilder, MakeService};
use tower_hyper::body::Body;
use tower_hyper::server::Server;

use tracing;
use tracing_subscriber::prelude::*;
use tracing_futures::Instrument;
use std::net::SocketAddr;

fn main() {
    // Set the default subscriber to record all traces emitted by this example
    // and by the `tracing_tower` library's helpers.
    let filter = tracing_subscriber::filter::Filter::new("tracing_tower=trace,load=trace");
    let (filter, handle) = tracing_subscriber::reload::Layer::new(filter);
    let subscriber = tracing_fmt::FmtSubscriber::builder()
        .with_filter(tracing_fmt::filter::none())
        .finish()
        .with(filter);

    let _ = tracing::subscriber::set_global_default(subscriber);

    let addr = "[::1]:3000".parse().unwrap();
    let admin_addr = "[::1]:3001".parse().unwrap();

    let admin = serve(AdminSvc { handle }, admin_addr, "admin");
    let server = serve(MakeSvc, addr, "server")
        .join(admin)
        .map(|_|())
        .map_err(|_|());

    hyper::rt::run(server);
}

fn serve<M>(make_svc: M, addr: SocketAddr, name: &str) -> impl Future<Item = (), Error = ()> + Send + Sync + 'static
where
    M: MakeService<
        (), Request<Body>,
        Response = Response<Body>,
        Error = hyper::Error,
        MakeError = hyper::Error,
    > + Send,
    M::Future: Send + Sync + 'static,
    M::Service: Send + Sync + 'static,
{
    let bind = TcpListener::bind(&addr).expect("bind");
    // Construct a span for the server task, annotated with the listening IP
    // address and port.
    let span = tracing::trace_span!("server", name = %name, ip = %addr.ip(), port = addr.port());
    let req_span: fn(&Request<_>) -> tracing::Span = |req: &Request<_>| {
        tracing::span!(
            tracing::Level::TRACE,
            "request",
            req.method = ?req.method(),
            req.uri = ?req.uri(),
            req.version = ?req.version(),
            req.path = ?req.uri().path(),
        )
    };
    future::lazy(|| {
        let svc = ServiceBuilder::new()
            .layer(tracing_tower::request_span::make::layer(req_span))
            .service(make_svc);

        let server = Server::new(svc);

        tracing::info!("listening");

        bind.incoming()
            .fold(server, |mut server, sock| {
                // Construct a new span for each accepted connection.
                let addr = sock.peer_addr().expect("can't get addr");
                let span = tracing::trace_span!("conn", ip = %addr.ip(), port = addr.port());
                let _enter = span.enter();

                tracing::debug!("accepted connection");

                if let Err(e) = sock.set_nodelay(true) {
                    return Err(e);
                }

                let serve = server
                    .serve(sock)
                    .map_err(|error| tracing::error!(message = "http error", %error))
                    .map(|_| {
                        tracing::trace!("finished serving connection");
                    })
                    .instrument(span.clone());
                hyper::rt::spawn(serve);

                Ok(server)
            })
            .map_err(|error| tracing::error!(message = "serve error", %error))
            .map(|_| {})
    })
    .instrument(span)
}

struct Svc;
impl Service<Request<Body>> for Svc {
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = future::FutureResult<Self::Response, Self::Error>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(().into())
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        tracing::debug!(req.headers = ?req.headers());

        let uri = req.uri();
        let mut rsp = Response::builder();
        let rsp = if uri.path() == "z" {
            tracing::trace!(error = %"i don't like this letter", letter = "z");
            rsp.status(500).body(Body::empty()).unwrap()
        } else {
            rsp.status(200).body(Body::empty()).unwrap()
        };
        future::ok(rsp)
    }
}

struct MakeSvc;
impl Service<()> for MakeSvc {
    type Response = Svc;
    type Error = hyper::Error;
    type Future = future::FutureResult<Self::Response, Self::Error>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(().into())
    }

    fn call(&mut self, _: ()) -> Self::Future {
        future::ok(Svc)
    }
}

struct AdminSvc<S> {
    handle: tracing_subscriber::reload::Handle<tracing_subscriber::filter::Filter, S>
}

impl<S> Clone for AdminSvc<S> {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
        }
    }
}

impl<S> Service<()> for AdminSvc<S>
where
    S: tracing::Subscriber,
{
    type Response = AdminSvc<S>;
    type Error = hyper::Error;
    type Future = future::FutureResult<Self::Response, Self::Error>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(().into())
    }

    fn call(&mut self, _: ()) -> Self::Future {
        future::ok(self.clone())
    }
}

impl<S> Service<Request<Body>> for AdminSvc<S>
where
    S: tracing::Subscriber,
{
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error> + Send + Sync + 'static>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(().into())
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        tracing::debug!(req.headers = ?req.headers());

        let uri = req.uri();
        if uri.path() == "filter" {
            let handle = self.clone();
            tracing::trace!("setting filter");
            let f = req
                .into_body()
                .concat2()
                .map(move |chunk| match handle.set_from(chunk) {
                    Err(error) => {
                        tracing::warn!(message = "setting filter failed", %error);
                        rsp(StatusCode::BAD_REQUEST, format!("{}", error))
                    }
                    Ok(()) => rsp(StatusCode::NO_CONTENT, Body::empty()),
                })
        } else {
            Box::new(future::ok(rsp(StatusCode::NOT_FOUND, Body::empty())))
        }
    }
}

impl<S> AdminSvc<S>
where
    S: tracing::Subscriber + Clone
{
    fn set_from(&self, chunk: hyper::Chunk) -> Result<(), String> {
        use std::str;
        let bytes = chunk.into_bytes();
        let body = str::from_utf8(&bytes.as_ref()).map_err(|e| format!("{}", e))?;
        tracing::trace!(request.body = ?body);
        let new_filter = body.parse::<tracing_subscriber::filter::Filter>().map_err(|e| format!("{}", e))?;
        self.handle.reload(new_filter).map_err(|e| format!("{}", e))
    }
}

fn rsp(status: StatusCode, body: impl Into<Body>) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(body.into())
        .expect("builder with known status code must not fail")
}
