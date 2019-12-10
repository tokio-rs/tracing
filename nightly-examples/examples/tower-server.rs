use futures::future;
use http::{Request, Response};
use hyper::{Body, Server};
use std::task::{Context, Poll};
use std::time::Duration;
use tower::{Service, ServiceBuilder};
use tracing::info;
use tracing_tower::request_span::make;

type Err = Box<dyn std::error::Error + Send + Sync + 'static>;

fn req_span<A>(req: &Request<A>) -> tracing::Span {
    let span = tracing::info_span!(
        "service",
        req.method = ?req.method(),
        req.uri = ?req.uri(),
        req.version = ?req.version(),
        headers = ?req.headers()
    );
    {
        // TODO: this is a workaround because tracing_subscriber::fmt::Layer doesn't honor
        // overridden span parents.
        let _enter = span.enter();
        tracing::info!(parent: &span, "accepted request");
    }
    span
}

const ROOT: &'static str = "/";

#[derive(Debug)]
pub struct Svc;

impl Service<Request<Body>> for Svc {
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let rsp = Response::builder();

        let uri = req.uri();
        if uri.path() != ROOT {
            let body = Body::from(Vec::new());
            let rsp = rsp.status(404).body(body).unwrap();
            return future::ok(rsp);
        }

        let body = Body::from(Vec::from(&b"heyo!"[..]));
        let rsp = rsp.status(200).body(body).unwrap();
        future::ok(rsp)
    }
}

pub struct MakeSvc;

impl<T> Service<T> for MakeSvc {
    type Response = Svc;
    type Error = std::io::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, _: T) -> Self::Future {
        future::ok(Svc)
    }
}

#[tokio::main]
async fn main() -> Result<(), Err> {
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter("tower=trace")
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let svc = ServiceBuilder::new()
        .timeout(Duration::from_millis(250))
        .layer(make::layer::<_, Svc, _>(req_span))
        .service(MakeSvc);

    let addr = "127.0.0.1:3000".parse()?;
    let server = Server::bind(&addr).serve(svc);
    info!(message = "listening", addr = ?addr);
    server.await?;

    Ok(())
}
