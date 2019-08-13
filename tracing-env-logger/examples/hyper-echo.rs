#![deny(rust_2018_idioms)]

use futures::future;
use hyper::rt::{Future, Stream};
use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::{Body, Method, Request, Response, StatusCode};

use std::str;

use tracing::{debug, error, field, info, span, Level};
use tracing_futures::{Instrument, Instrumented};

type BoxFut = Box<dyn Future<Item = Response<Body>, Error = hyper::Error> + Send>;

fn echo(req: Request<Body>) -> Instrumented<BoxFut> {
    let span = span!(
        Level::TRACE,
        "request",
        method = &field::debug(req.method()),
        uri = &field::debug(req.uri()),
        headers = &field::debug(req.headers())
    );
    let _enter = span.enter();
    info!("received request");
    let mut response = Response::new(Body::empty());

    let (rsp_span, fut): (_, BoxFut) = match (req.method(), req.uri().path()) {
        // Serve some instructions at /
        (&Method::GET, "/") => {
            const BODY: &'static str = "Try POSTing data to /echo";
            *response.body_mut() = Body::from(BODY);
            (
                span!(Level::TRACE, "response", body = %(&BODY)),
                Box::new(future::ok(response)),
            )
        }

        // Simply echo the body back to the client.
        (&Method::POST, "/echo") => {
            let body = req.into_body();
            let span = span!(Level::TRACE, "response", response_kind = &"echo");
            *response.body_mut() = body;
            (span, Box::new(future::ok(response)))
        }

        // Convert to uppercase before sending back to client.
        (&Method::POST, "/echo/uppercase") => {
            let mapping = req.into_body().map(|chunk| {
                let upper = chunk
                    .iter()
                    .map(|byte| byte.to_ascii_uppercase())
                    .collect::<Vec<u8>>();
                debug!(
                    {
                        chunk = field::debug(str::from_utf8(&chunk[..])),
                        uppercased = field::debug(str::from_utf8(&upper[..]))
                    },
                    "uppercased request body"
                );
                upper
            });

            *response.body_mut() = Body::wrap_stream(mapping);
            (
                span!(Level::TRACE, "response", response_kind = "uppercase"),
                Box::new(future::ok(response)),
            )
        }

        // Reverse the entire body before sending back to the client.
        //
        // Since we don't know the end yet, we can't simply stream
        // the chunks as they arrive. So, this returns a different
        // future, waiting on concatenating the full body, so that
        // it can be reversed. Only then can we return a `Response`.
        (&Method::POST, "/echo/reversed") => {
            let span = span!(Level::TRACE, "response", response_kind = "reversed");
            let _enter = span.enter();
            let reversed = req.into_body().concat2().map(move |chunk| {
                let body = chunk.iter().rev().cloned().collect::<Vec<u8>>();
                debug!(
                    {
                        chunk = ?str::from_utf8(&chunk[..]),
                        body = ?str::from_utf8(&body[..])
                    },
                    "reversed request body");
                *response.body_mut() = Body::from(body);
                response
            });
            (span.clone(), Box::new(reversed))
        }

        // The 404 Not Found route...
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
            (
                span!(
                    Level::TRACE,
                    "response",
                    body = &field::debug(()),
                    status = &field::debug(&StatusCode::NOT_FOUND)
                ),
                Box::new(future::ok(response)),
            )
        }
    };

    fut.instrument(rsp_span)
}

fn main() {
    let subscriber = tracing_fmt::FmtSubscriber::builder().finish();
    tracing_env_logger::try_init().expect("init log adapter");

    let _ = tracing::subscriber::set_global_default(subscriber);
    let addr: ::std::net::SocketAddr = ([127, 0, 0, 1], 3000).into();
    let server_span = span!(Level::TRACE, "server", local = %addr);
    let _enter = server_span.enter();
    let server = tokio::net::TcpListener::bind(&addr)
        .expect("bind")
        .incoming()
        .fold(Http::new(), move |http, sock| {
            let span = span!(
                Level::TRACE,
                "connection",
                remote = ?sock.peer_addr().unwrap()
            );
            hyper::rt::spawn(
                http.serve_connection(sock, service_fn(echo))
                    .map_err(|e| {
                        error!(message = "serve error", error = %&e);
                    })
                    .instrument(span),
            );
            Ok::<_, ::std::io::Error>(http)
        })
        .map(|_| ())
        .map_err(|e| {
            error!(message = "server error", error = %&e);
        })
        .instrument(server_span.clone());
    info!("listening...");
    hyper::rt::run(server);
}
