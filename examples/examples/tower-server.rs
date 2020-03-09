use futures::future;
use http::{Request, Response};
use hyper::{Body, Server};
use std::task::{Context, Poll};
use std::time::Duration;
use tower::{Service, ServiceBuilder};
use tracing::dispatcher;
use tracing::info;
use tracing_tower::request_span::make;

type Err = Box<dyn std::error::Error + Send + Sync + 'static>;

fn req_span<A>(req: &Request<A>) -> tracing::Span {
    let span = tracing::info_span!(
        "request",
        req.method = ?req.method(),
        req.uri = ?req.uri(),
        req.version = ?req.version(),
        req.headers = ?req.headers()
    );
    {
        // TODO: this is a workaround because tracing_subscriber::fmt::Layer doesn't honor
        // overridden span parents.
        let _enter = span.enter();
    }
    span
}

const ROOT: &str = "/";

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
        let rsp = if uri.path() != ROOT {
            let body = Body::from(Vec::new());
            rsp.status(404).body(body).unwrap()
        } else {
            let body = Body::from(Vec::from(&b"heyo!"[..]));
            rsp.status(200).body(body).unwrap()
        };
        let span = tracing::info_span!(
            "response",
            rsp.status = ?rsp.status(),
            rsp.version = ?rsp.version(),
            rsp.headers = ?rsp.headers()
        );

        dispatcher::get_default(|dispatch| {
            let id = span.id().expect("Missing ID; this is a bug");
            if let Some(current) = dispatch.current_span().id() {
                dispatch.record_follows_from(&id, current)
            }
        });
        let _guard = span.enter();
        info!("sending response");
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
