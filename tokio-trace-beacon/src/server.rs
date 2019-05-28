// Taken from tokio-rs/tokio/tokio-trace/examples/sloggish/main.rs

use tokio_trace::{field, Level};

pub(crate) fn incoming_connection() {
    span!(Level::TRACE, "", version = &field::display(5.0)).in_scope(|| {
        span!(Level::TRACE, "server", host = "localhost", port = 8080).in_scope(|| {
            info!("starting");
            info!("listening");
            let peer1 = span!(Level::TRACE, "conn", peer_addr = "82.9.9.9", port = 42381);
            peer1.in_scope(|| {
                debug!("connected");
                debug!({ length = 2 }, "message received");
            });
            let peer2 = span!(Level::TRACE, "conn", peer_addr = "8.8.8.8", port = 18230);
            peer2.in_scope(|| {
                debug!("connected");
            });
            peer1.in_scope(|| {
                warn!({ algo = "xor" }, "weak encryption requested");
                debug!({ length = 8 }, "response sent");
                debug!("disconnected");
            });
            peer2.in_scope(|| {
                debug!({ length = 5 }, "message received");
                debug!({ length = 8 }, "response sent");
                debug!("disconnected");
            });
            warn!("internal error");
            info!("exit");
        })
    })
}
