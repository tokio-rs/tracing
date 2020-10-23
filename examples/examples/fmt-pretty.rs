#![deny(rust_2018_idioms)]

use tracing::{debug, info, span, warn, Level};
use tracing_subscriber::prelude::*;
fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::subscriber()
                .pretty()
                .with_thread_names(true)
                .with_thread_ids(true)
                .with_target(false),
        )
        .init();

    let app_span = span!(Level::TRACE, "app", version = %5.0);
    let _e = app_span.enter();

    let server_span = span!(Level::TRACE, "server", host = "localhost", port = 8080);
    let _e2 = server_span.enter();
    info!("starting");
    info!("listening");
    let peer1 = span!(Level::TRACE, "conn", peer_addr = "82.9.9.9", port = 42381);
    peer1.in_scope(|| {
        debug!("connected");
        debug!(length = 2, "message received");
    });
    let peer2 = span!(Level::TRACE, "conn", peer_addr = "8.8.8.8", port = 18230);
    peer2.in_scope(|| {
        debug!("connected");
    });
    peer1.in_scope(|| {
        warn!(algo = "xor", "weak encryption requested");
        debug!(length = 8, "response sent");
        debug!("disconnected");
    });
    peer2.in_scope(|| {
        debug!(length = 5, "message received");
        debug!(length = 8, "response sent");
        debug!("disconnected");
    });
    warn!("internal error");
    info!("exit");
}
