#![deny(rust_2018_idioms)]

use http::{Method, Request, Response, StatusCode};
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Server,
};
use std::str;
use tracing::{debug, info, span, Instrument as _, Level};

async fn echo(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let span = span!(
        Level::INFO,
        "request",
        method = ?req.method(),
        uri = ?req.uri(),
        headers = ?req.headers()
    );
    async move {
        info!("received request");
        let mut response = Response::new(Body::empty());

        match (req.method(), req.uri().path()) {
            // Serve some instructions at /
            (&Method::GET, "/") => {
                const BODY: &str = "Try POSTing data to /echo";
                *response.body_mut() = Body::from(BODY);
                info!(body = %(&BODY), "response",);
                Ok(response)
            }

            // Simply echo the body back to the client.
            (&Method::POST, "/echo") => {
                info!(response_kind = %"echo", "response");
                *response.body_mut() = req.into_body();
                Ok(response)
            }

            // Convert to uppercase before sending back to client.
            (&Method::POST, "/echo/uppercase") => {
                let body = hyper::body::to_bytes(req).await?;
                let upper = body
                    .iter()
                    .map(|byte| byte.to_ascii_uppercase())
                    .collect::<Vec<u8>>();
                debug!(
                    body = ?str::from_utf8(&body[..]),
                    uppercased = ?str::from_utf8(&upper[..]),
                    "uppercased request body"
                );

                info!(response_kind = %"uppercase", "response");
                *response.body_mut() = Body::from(upper);
                Ok(response)
            }

            // Reverse the entire body before sending back to the client.
            (&Method::POST, "/echo/reversed") => {
                async move {
                    let body = hyper::body::to_bytes(req).await?;
                    let reversed = body.iter().rev().cloned().collect::<Vec<u8>>();
                    debug!(
                        body = ?str::from_utf8(&body[..]),
                        "reversed request body"
                    );
                    *response.body_mut() = Body::from(reversed);
                    info!( body = ?(&response.body()), "response");
                    Ok(response)
                }
                .instrument(span!(Level::TRACE, "response", response_kind = %"reversed"))
                .await
            }

            // The 404 Not Found route...
            _ => {
                *response.status_mut() = StatusCode::NOT_FOUND;
                info!(
                    body = ?(),
                    status = ?StatusCode::NOT_FOUND,
                    "response",
                );
                Ok(response)
            }
        }
    }
    .instrument(span)
    .await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use tracing_log::env_logger::BuilderExt;

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .finish();
    let mut builder = env_logger::Builder::new();
    builder
        .filter(Some("hyper_echo"), log::LevelFilter::Off)
        .filter(Some("hyper"), log::LevelFilter::Trace)
        .emit_traces() // from `tracing_log::env_logger::BuilderExt`
        .try_init()?;
    tracing::subscriber::set_global_default(subscriber)?;

    let local_addr: std::net::SocketAddr = ([127, 0, 0, 1], 3000).into();
    let server_span = span!(Level::TRACE, "server", %local_addr);

    let service = make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(echo)) });
    let server = Server::bind(&local_addr)
        .serve(service)
        .instrument(server_span.clone());

    info!(parent: &server_span, "listening...");
    server.await?;

    Ok(())
}
