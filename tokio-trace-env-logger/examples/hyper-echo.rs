extern crate futures;
extern crate hyper;
#[macro_use]
extern crate tokio_trace;
extern crate tokio_trace_env_logger;

use futures::future;
use hyper::rt::{Future, Stream};
use hyper::service::service_fn;
use hyper::{Body, Method, Request, Response, Server, StatusCode};

#[path = "../../tokio-trace/examples/sloggish_subscriber.rs"]
mod sloggish;
use self::sloggish::SloggishSubscriber;

use tokio_trace::{
    Level,
    instrument::{Instrument, Instrumented},
};

type BoxFut = Box<dyn Future<Item = Response<Body>, Error = hyper::Error> + Send>;

fn echo(req: Request<Body>) -> Instrumented<BoxFut> {
    span!(
        "request",
        method = req.method().clone(),
        uri = req.uri().clone(),
        headers = req.headers().clone()
    ).enter( || {
        event!(Level::Info, {}, "received request");
        let mut response = Response::new(Body::empty());

        let (rsp_span, fut): (_, BoxFut) = match (req.method(), req.uri().path()) {
            // Serve some instructions at /
            (&Method::GET, "/") => {
                const BODY: &'static str = "Try POSTing data to /echo";
                *response.body_mut() = Body::from(BODY);
                (span!("response", body = BODY), Box::new(future::ok(response)))
            }

            // Simply echo the body back to the client.
            (&Method::POST, "/echo") => {
                let body = req.into_body();
                *response.body_mut() = body;
                (span!("response", body = "echo"), Box::new(future::ok(response)))
            }

            // Convert to uppercase before sending back to client.
            (&Method::POST, "/echo/uppercase") => {
                let mapping = req.into_body().map(|chunk| {
                    event!(Level::Debug, { chunk = format!("{:?}", chunk) }, "got chunk");
                    chunk
                        .iter()
                        .map(|byte| byte.to_ascii_uppercase())
                        .collect::<Vec<u8>>()
                });

                *response.body_mut() = Body::wrap_stream(mapping);
                (span!("response", body = "uppercase"), Box::new(future::ok(response)))
            }

            // Reverse the entire body before sending back to the client.
            //
            // Since we don't know the end yet, we can't simply stream
            // the chunks as they arrive. So, this returns a different
            // future, waiting on concatenating the full body, so that
            // it can be reversed. Only then can we return a `Response`.
            (&Method::POST, "/echo/reversed") => {
                let reversed = req.into_body().concat2().map(move |chunk| {
                    let body = chunk.iter().rev().cloned().collect::<Vec<u8>>();
                    *response.body_mut() = Body::from(body);
                    response
                });

                (span!("response", body = "reversed"), Box::new(reversed))
            }

            // The 404 Not Found route...
            _ => {
                *response.status_mut() = StatusCode::NOT_FOUND;
                (span!(
                    "response",
                    body = (),
                    status = StatusCode::NOT_FOUND
                ),
                Box::new(future::ok(response)))
            }
        };

        fut.instrument(rsp_span)
    })
}

fn main() {
    tokio_trace::Dispatcher::builder()
        .add_subscriber(SloggishSubscriber::new(2))
        .init();
    tokio_trace_env_logger::try_init().expect("init log adapter");

    let addr: ::std::net::SocketAddr = ([127, 0, 0, 1], 3000).into();
    let server_span = span!(
        "server",
        local_addr = addr.ip(),
        local_port = addr.port()
    );
    server_span.clone().enter(|| {
        let span2 = server_span.clone();
        let server = Server::bind(&addr)
            .serve(move || span2.enter(|| service_fn(echo)))
            .map_err(|e| event!(Level::Error, { error = e }, "server error"));
        event!(Level::Info, {}, "listening...");
        hyper::rt::run(server.instrument(server_span));
    });
}
