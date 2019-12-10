use http::{Method, Response, Request, Uri};
use hyper::{
    client::Client,
    Body,
};
use tower::{Service, ServiceBuilder};
use std::time::Duration;
use tracing::info;

type Err = Box<dyn std::error::Error + Send + Sync + 'static>;

fn req_span<A>(req: &Request<A>) -> tracing::Span {
    let span = tracing::info_span!(
        "request",
        req.method = ?req.method(),
        req.uri = ?req.uri(),
        req.version = ?req.version(),
        headers = ?req.headers()
    );
    {
        // TODO: this is a workaround because tracing_subscriber::fmt::Layer doesn't honor
        // overridden span parents.
        let _enter = span.enter();
        tracing::info!(parent: &span, "sending request");
    }
    span
}

#[tokio::main]
async fn main() -> Result<(), Err> {
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter("tower=trace")
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let mut svc = ServiceBuilder::new()
        .timeout(Duration::from_millis(250))
        .layer(tracing_tower::request_span::layer(req_span))
        .service(Client::new());

    // let mut svc = svc.trace_requests(req_span);
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
