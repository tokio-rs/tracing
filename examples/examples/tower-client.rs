use http::{Method, Request, Uri};
use hyper::{client::Client, Body};
use std::time::Duration;
use tower::{Service, ServiceBuilder};
use tracing::info;
use tracing_tower::request_span;

type Err = Box<dyn std::error::Error + Send + Sync + 'static>;

fn req_span<A>(req: &Request<A>) -> tracing::Span {
    let span = tracing::info_span!(
        "request",
        req.method = ?req.method(),
        req.uri = ?req.uri(),
        req.version = ?req.version(),
        headers = ?req.headers()
    );
    tracing::info!(parent: &span, "sending request");
    span
}

#[tokio::main]
async fn main() -> Result<(), Err> {
    tracing_subscriber::fmt()
        .with_env_filter("tower=trace")
        .try_init()?;

    let mut svc = ServiceBuilder::new()
        .timeout(Duration::from_millis(250))
        .layer(request_span::layer(req_span))
        .service(Client::new());

    let uri = Uri::from_static("http://httpbin.org");

    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .expect("Unable to build request; this is a bug.");

    let res = svc.call(req).await?;
    info!(message = "got a response", res.headers = ?res.headers());

    Ok(())
}
