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

use bytes::Bytes;
use futures::{
    future::{self, Ready},
    stream::StreamExt,
    Future,
};
use http::{header, Method, Request, Response, StatusCode};
use hyper::{server::conn::AddrStream, Body, Client, Server};
use rand::Rng;
use std::{
    error::Error,
    fmt,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tokio::{time, try_join};
use tower::{Service, ServiceBuilder, ServiceExt};
use tracing::{self, debug, error, info, span, trace, warn, Level, Span};
use tracing_futures::Instrument;
use tracing_subscriber::{filter::EnvFilter, reload::Handle};
use tracing_tower::{request_span, request_span::make};

type Err = Box<dyn Error + Send + Sync + 'static>;

#[tokio::main]
async fn main() -> Result<(), Err> {
    let builder = tracing_subscriber::fmt()
        .with_env_filter("info,tower_load=debug")
        .with_filter_reloading();
    let handle = builder.reload_handle();
    builder.try_init()?;

    let addr = "[::1]:3000".parse::<SocketAddr>()?;
    let admin_addr = "[::1]:3001".parse::<SocketAddr>()?;

    let admin = ServiceBuilder::new().service(AdminSvc { handle });

    let svc = ServiceBuilder::new()
        .layer(make::layer::<_, Svc, _>(req_span))
        .service(MakeSvc);

    let svc = Server::bind(&addr).serve(svc);
    let admin = Server::bind(&admin_addr).serve(admin);

    let res = try_join!(
        tokio::spawn(load_gen(addr)),
        tokio::spawn(load_gen(addr)),
        tokio::spawn(load_gen(addr)),
        tokio::spawn(svc),
        tokio::spawn(admin)
    );

    match res {
        Ok(_) => info!("load generator exited successfully"),
        Err(e) => {
            error!(error = ?e, "load generator failed");
        }
    }
    Ok(())
}

struct Svc;
impl Service<Request<Body>> for Svc {
    type Response = Response<Body>;
    type Error = Err;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let rsp = Self::handle_request(req)
            .map(|body| {
                trace!("sending response");
                rsp(StatusCode::OK, body)
            })
            .unwrap_or_else(|e| {
                trace!(rsp.error = %e);
                let status = match e {
                    HandleError::BadPath => {
                        warn!(rsp.status = ?StatusCode::NOT_FOUND);
                        StatusCode::NOT_FOUND
                    }
                    HandleError::NoContentLength | HandleError::BadRequest(_) => {
                        StatusCode::BAD_REQUEST
                    }
                    HandleError::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
                };
                rsp(status, e.to_string())
            });
        future::ok(rsp)
    }
}

impl Svc {
    fn handle_request(req: Request<Body>) -> Result<String, HandleError> {
        const BAD_METHOD: WrongMethod = WrongMethod(&[Method::GET]);
        trace!("handling request...");
        match (req.method(), req.uri().path()) {
            (&Method::GET, "/z") => {
                trace!(error = %"i don't like this letter.", letter = "z");
                Err(HandleError::Unknown)
            }
            (&Method::GET, path) => {
                let ch = path.get(1..2).ok_or(HandleError::BadPath)?;
                let content_length = req
                    .headers()
                    .get(header::CONTENT_LENGTH)
                    .ok_or(HandleError::NoContentLength)?;
                trace!(req.content_length = ?content_length);
                let content_length = content_length
                    .to_str()
                    .map_err(HandleError::bad_request)?
                    .parse::<usize>()
                    .map_err(HandleError::bad_request)?;
                let mut body = String::new();
                let span = span!(
                    Level::DEBUG,
                    "build_rsp",
                    rsp.len = content_length,
                    rsp.character = ch
                );
                let _enter = span.enter();
                for idx in 0..content_length {
                    body.push_str(ch);
                    trace!(rsp.body = ?body, rsp.body.idx = idx);
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
impl<T> Service<T> for MakeSvc {
    type Response = Svc;
    type Error = Err;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: T) -> Self::Future {
        future::ok(Svc)
    }
}

struct AdminSvc<S> {
    handle: Handle<EnvFilter, S>,
}

impl<S> Clone for AdminSvc<S> {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
        }
    }
}

impl<'a, S> Service<&'a AddrStream> for AdminSvc<S>
where
    S: tracing::Subscriber,
{
    type Response = AdminSvc<S>;
    type Error = hyper::Error;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: &'a AddrStream) -> Self::Future {
        future::ok(self.clone())
    }
}

impl<S> Service<Request<Body>> for AdminSvc<S>
where
    S: tracing::Subscriber + 'static,
{
    type Response = Response<Body>;
    type Error = Err;
    type Future = Pin<Box<dyn Future<Output = Result<Response<Body>, Err>> + std::marker::Send>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        // we need to clone so that the reference to self
        // isn't outlived by the returned future.
        let handle = self.clone();
        let f = async move {
            let rsp = match (req.method(), req.uri().path()) {
                (&Method::PUT, "/filter") => {
                    trace!("setting filter");

                    let body = hyper::body::to_bytes(req).await?;
                    match handle.set_from(body) {
                        Err(error) => {
                            error!(%error, "setting filter failed!");
                            rsp(StatusCode::INTERNAL_SERVER_ERROR, error)
                        }
                        Ok(()) => rsp(StatusCode::NO_CONTENT, Body::empty()),
                    }
                }
                _ => rsp(StatusCode::NOT_FOUND, "try `/filter`"),
            };
            Ok(rsp)
        };
        Box::pin(f)
    }
}

impl<S> AdminSvc<S>
where
    S: tracing::Subscriber + 'static,
{
    fn set_from(&self, bytes: Bytes) -> Result<(), String> {
        use std::str;
        let body = str::from_utf8(&bytes.as_ref()).map_err(|e| format!("{}", e))?;
        trace!(request.body = ?body);
        let new_filter = body
            .parse::<tracing_subscriber::filter::EnvFilter>()
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

fn gen_uri(authority: &str) -> (usize, String) {
    static ALPHABET: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut rng = rand::thread_rng();
    let idx = rng.gen_range(0, ALPHABET.len() + 1);
    let len = rng.gen_range(0, 26);
    let letter = ALPHABET.get(idx..=idx).unwrap_or("");
    (len, format!("http://{}/{}", authority, letter))
}

#[tracing::instrument(target = "gen", "load_gen")]
async fn load_gen(addr: SocketAddr) -> Result<(), Err> {
    let svc = ServiceBuilder::new()
        .buffer(5)
        .layer(request_span::layer(req_span))
        .timeout(Duration::from_millis(200))
        .service(Client::new());
    let mut interval = time::interval(Duration::from_millis(50));
    while interval.next().await.is_some() {
        let authority = format!("{}", addr);
        let mut svc = svc.clone().ready_oneshot().await?;

        let f = async move {
            let sleep = rand::thread_rng().gen_range(0, 25);
            time::delay_for(Duration::from_millis(sleep)).await;

            let (len, uri) = gen_uri(&authority);
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
            async move {
                info!(target: "gen", "sending request");
                let rsp = match svc.call(req).await {
                    Err(e) => {
                        error!(target: "gen", error = %e, "request error!");
                        return Err(e);
                    }
                    Ok(rsp) => rsp,
                };

                let status = rsp.status();
                if status != StatusCode::OK {
                    error!(target: "gen", status = ?status, "error received from server!");
                }

                let body = match hyper::body::to_bytes(rsp).await {
                    Err(e) => {
                        error!(target: "gen", error = ?e, "body error!");
                        return Err(e.into());
                    }
                    Ok(body) => body,
                };
                let body = String::from_utf8(body.to_vec())?;
                info!(target: "gen", message = "response complete.", rsp.body = %body);
                Ok(())
            }
            .instrument(span)
            .await
        }
        .instrument(span!(target: "gen", Level::INFO, "generated_request", remote.addr=%addr));
        tokio::spawn(f);
    }

    Ok(())
}

fn req_span<A>(req: &Request<A>) -> Span {
    let span = tracing::span!(
        target: "gen",
        Level::INFO,
        "request",
        req.method = ?req.method(),
        req.path = ?req.uri().path(),
    );
    debug!(
        parent: &span,
        message = "received request.",
        req.headers = ?req.headers(),
        req.version = ?req.version(),
    );
    span
}
