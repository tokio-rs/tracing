//! A demo showing how filtering on values and dynamic filter reloading can be
//! used together to help make sense of complex or noisy traces.
//!
//! This example runs a simple HTTP server that implements highly advanced,
//! cloud-native "character repetition as a service", on port 3000. The server's
//! `GET /${CHARACTER}` route will respond with a string containing that
//! character repeated up to the requested `Content-Length`. A load generator
//! runs in the background and constantly sends requests for various characters
//! to be repeated.
//!
//! As the load generator's logs indicate, the server will sometimes return
//! errors, including HTTP 500s! Because the logs at high load are so noisy,
//! tracking down the root cause of the errors can be difficult, if not
//! impossible. Since the character-repetition service is absolutely
//! mission-critical to our organization, we have to determine what is causing
//! these errors as soon as possible!
//!
//! Fortunately, an admin service running on port 3001 exposes a `PUT /filter`
//! route that can be used to change the trace filter for the format subscriber.
//! By dynamically changing the filter we can try to track down the cause of the
//! error.
//!
//! As a hint: all spans and events from the load generator have the "gen" target
#![deny(rust_2018_idioms)]
use futures::{future, Future, Poll, Stream};
use hyper::{header, Method, Request, Response, StatusCode};
use tokio_tcp::TcpListener;
use tower::{MakeService, Service, ServiceBuilder};
use tower_hyper::body::Body;
use tower_hyper::server::Server;

use std::{error::Error, fmt, net::SocketAddr};
use tracing;
use tracing_futures::Instrument;
use tracing_subscriber::FmtSubscriber;

fn main() {
    let filter = "info,tower_load=debug"
        .parse()
        .expect("filter should be valid");
    let builder = FmtSubscriber::builder()
        .with_filter(filter)
        .with_filter_reloading();
    let handle = builder.reload_handle();

    let _ = tracing::subscriber::set_global_default(builder.finish());

    let addr = "[::1]:3000".parse().unwrap();
    let admin_addr = "[::1]:3001".parse().unwrap();

    let admin = serve(AdminSvc { handle }, &admin_addr, "admin");
    let server = serve(MakeSvc, &addr, "serve")
        .join(admin)
        .join(load_gen(&addr))
        .join(load_gen(&addr))
        .join(load_gen(&addr))
        .map(|_| ())
        .map_err(|_| ());

    hyper::rt::run(server);
}

fn serve<M>(
    make_svc: M,
    addr: &SocketAddr,
    name: &str,
) -> impl Future<Item = (), Error = ()> + Send + 'static
where
    M: MakeService<
            (),
            Request<Body>,
            Response = Response<Body>,
            Error = hyper::Error,
            MakeError = hyper::Error,
        > + Send
        + 'static,
    M::Future: Send + 'static,
    M::Service: Send + 'static,
    <M::Service as Service<Request<Body>>>::Future: Send + 'static,
{
    let bind = TcpListener::bind(addr).expect("bind");
    let span = tracing::info_span!("server", name = %name, local = %addr);
    let trace_req: fn(&Request<_>) -> tracing::Span = |req: &Request<_>| {
        let span = tracing::debug_span!(
            "request",
            req.method = ?req.method(),
            req.path = ?req.uri().path(),
        );
        span.in_scope(|| {
            tracing::debug!(message = "received request.", req.headers = ?req.headers(), req.version = ?req.version());
        });
        span
    };
    future::lazy(move || {
        let svc = ServiceBuilder::new()
            .layer(tracing_tower::request_span::make::layer(trace_req))
            .service(make_svc);

        let server = Server::new(svc);

        tracing::info!("listening");

        bind.incoming()
            .fold(server, |mut server, sock| {
                // Construct a new span for each accepted connection.
                let addr = sock.peer_addr().expect("can't get addr");
                let span = tracing::info_span!("conn", remote = %addr);
                let _enter = span.enter();

                tracing::debug!("accepted connection");

                if let Err(e) = sock.set_nodelay(true) {
                    return Err(e);
                }

                let serve = server
                    .serve(sock)
                    .map_err(|error| tracing::error!(message = "http error!", %error))
                    .map(|_| {
                        tracing::trace!("finished serving connection");
                    })
                    .instrument(span.clone());
                hyper::rt::spawn(serve);

                Ok(server)
            })
            .map_err(|error| tracing::error!(message = "serve error!", %error))
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
        let rsp = Self::handle_request(req)
            .map(|body| {
                tracing::trace!("sending response");
                rsp(StatusCode::OK, body)
            })
            .unwrap_or_else(|e| {
                tracing::trace!(rsp.error = %e);
                let status = match e {
                    HandleError::BadPath => {
                        tracing::warn!(rsp.status = ?StatusCode::NOT_FOUND);
                        StatusCode::NOT_FOUND
                    }
                    HandleError::NoContentLength | HandleError::BadRequest(_) => {
                        StatusCode::BAD_REQUEST
                    }
                    HandleError::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
                };
                rsp(status, format!("{}", e))
            });
        future::ok(rsp)
    }
}

impl Svc {
    fn handle_request(req: Request<Body>) -> Result<String, HandleError> {
        const BAD_METHOD: WrongMethod = WrongMethod(&[Method::GET]);
        tracing::trace!("handling request...");
        match (req.method(), req.uri().path()) {
            (&Method::GET, "/z") => {
                tracing::trace!(error = %"i don't like this letter.", letter = "z");
                Err(HandleError::Unknown)
            }
            (&Method::GET, path) => {
                let ch = path.get(1..2).ok_or(HandleError::BadPath)?;
                let content_length = req
                    .headers()
                    .get(header::CONTENT_LENGTH)
                    .ok_or(HandleError::NoContentLength)?;
                tracing::trace!(req.content_length = ?content_length);
                let content_length = content_length
                    .to_str()
                    .map_err(HandleError::bad_request)?
                    .parse::<usize>()
                    .map_err(HandleError::bad_request)?;
                let mut body = String::new();
                let span =
                    tracing::debug_span!("build_rsp", rsp.len = content_length, rsp.character = ch);
                let _enter = span.enter();
                for idx in 0..content_length {
                    body.push_str(ch);
                    tracing::trace!(rsp.body = ?body, rsp.body.idx = idx);
                }
                Ok(body)
            }
            _ => Err(HandleError::bad_request(BAD_METHOD)),
        }
    }
}

#[derive(Debug)]
enum HandleError {
    BadPath,
    NoContentLength,
    BadRequest(Box<dyn Error + Send + 'static>),
    Unknown,
}

#[derive(Debug, Clone)]
struct WrongMethod(&'static [Method]);

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
    handle: tracing_subscriber::reload::Handle<tracing_subscriber::filter::Filter, S>,
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
    type Future = Box<dyn Future<Item = Self::Response, Error = Self::Error> + Send + 'static>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(().into())
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        match (req.method(), req.uri().path()) {
            (&Method::PUT, "/filter") => {
                let handle = self.clone();
                tracing::trace!("setting filter");
                let f = req
                    .into_body()
                    .concat2()
                    .map(move |chunk| match handle.set_from(chunk) {
                        Err(error) => {
                            tracing::warn!(message = "setting filter failed!", %error);
                            rsp(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", error))
                        }
                        Ok(()) => rsp(StatusCode::NO_CONTENT, Body::empty()),
                    });
                Box::new(f)
            }
            _ => Box::new(future::ok(rsp(StatusCode::NOT_FOUND, "try `/filter`"))),
        }
    }
}

impl<S> AdminSvc<S>
where
    S: tracing::Subscriber,
{
    fn set_from(&self, chunk: hyper::Chunk) -> Result<(), String> {
        use std::str;
        let bytes = chunk.into_bytes();
        let body = str::from_utf8(&bytes.as_ref()).map_err(|e| format!("{}", e))?;
        tracing::trace!(request.body = ?body);
        let new_filter = body
            .parse::<tracing_subscriber::filter::Filter>()
            .map_err(|e| format!("{}", e))?;
        self.handle.reload(new_filter).map_err(|e| format!("{}", e))
    }
}

fn rsp(status: StatusCode, body: impl Into<Body>) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(body.into())
        .expect("builder with known status code must not fail")
}

impl HandleError {
    fn bad_request(e: impl std::error::Error + Send + 'static) -> Self {
        HandleError::BadRequest(Box::new(e))
    }
}

impl fmt::Display for HandleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HandleError::BadPath => f.pad("path must be a single ASCII character"),
            HandleError::NoContentLength => f.pad("request must have Content-Length header"),
            HandleError::BadRequest(ref e) => write!(f, "bad request: {}", e),
            HandleError::Unknown => f.pad("unknown internal error"),
        }
    }
}

impl std::error::Error for HandleError {}

impl fmt::Display for WrongMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported method: please use one of {:?}", self.0)
    }
}

impl std::error::Error for WrongMethod {}

fn load_gen(addr: &SocketAddr) -> Box<dyn Future<Item = (), Error = ()> + Send + 'static> {
    use rand::Rng;
    use std::time::{Duration, Instant};
    use tokio::timer;
    use tokio_buf::util::BufStreamExt;
    use tower::ServiceExt;
    use tower_http_util::body::BodyExt;
    use tower_hyper::client::Client;

    static ALPHABET: &'static str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let authority = format!("{}", addr);
    let f = future::lazy(move || {
        let hyper = Client::new();
        let svc = ServiceBuilder::new()
            .buffer(5)
            .timeout(Duration::from_secs(5))
            .service(hyper);

        timer::Interval::new_interval(Duration::from_millis(50))
            .and_then(|_| {
                let sleep = rand::thread_rng().gen_range(0, 25);
                timer::Delay::new(Instant::now() + Duration::from_millis(sleep)).map_err(Into::into)
            })
            .fold((svc, authority), |(svc, authority), _| {
                let mut rng = rand::thread_rng();
                let idx = rng.gen_range(0, ALPHABET.len()+1);
                let len = rng.gen_range(0, 26);
                let letter = ALPHABET.get(idx..idx+1).unwrap_or("");
                let uri = format!("http://{}/{}", authority, letter);
                let req = Request::get(&uri[..])
                    .header("Content-Length", len)
                    .body(Body::empty())
                    .unwrap();
                let span = tracing::debug_span!(
                    target: "gen",
                    "request",
                    req.method = ?req.method(),
                    req.path = ?req.uri().path(),
                );
                let f = svc.clone().ready()
                    .and_then(|mut svc| {
                        tracing::trace!(target: "gen", message = "sending request...");
                        svc.call(req)
                    })
                    .map_err(|e| tracing::error!(target: "gen", message = "request error!", error = %e))
                    .and_then(|response| {
                        let status = response.status();
                        if status != StatusCode::OK {
                            tracing::error!(target: "gen", message = "error received from server!", ?status);
                        }
                        response
                            .into_body()
                            .into_buf_stream()
                            .collect::<Vec<u8>>()
                            .map(|v| String::from_utf8(v).unwrap())
                            .map_err(|e| tracing::error!(target: "gen", message = "body error!", error = ?e))
                    })
                    .and_then(|body| {
                        tracing::trace!(target: "gen", message = "response complete.", rsp.body = %body);
                        Ok(())
                    })
                    .instrument(span);
                hyper::rt::spawn(f);
                future::ok((svc, authority))
            }).map(|_| ())
            .map_err(|e| panic!("timer error {}", e))
    }).instrument(tracing::info_span!(target: "gen", "load_gen", remote.addr=%addr));

    Box::new(f)
}
