//! A "hello world" echo server [from Tokio][echo-example]
//!
//! This server will create a TCP listener, accept connections in a loop, and
//! write back everything that's read off of each TCP connection.
//!
//! Because the Tokio runtime uses a thread pool, each TCP connection is
//! processed concurrently with all other TCP connections across multiple
//! threads.
//!
//! To see this server in action, you can run this in one terminal:
//!
//!     cargo +nightly run --example echo
//!
//! and in another terminal you can run:
//!
//!     nc localhost 3000
//!
//! Each line you type in to the `netcat` terminal should be echo'd back to
//! you! If you open up multiple terminals with `netcat` instances connected
//! to the same address you should be able to see them all make progress simultaneously.
//!
//! [echo-example]: https://github.com/tokio-rs/tokio/blob/master/tokio/examples/echo.rs

#![warn(rust_2018_idioms)]

use futures::future::{FutureExt, TryFutureExt};
use tokio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use std::env;
use std::error::Error;
use std::net::SocketAddr;

use tracing::{debug, info, info_span, trace_span, warn};
use tracing_futures::Instrument;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    use tracing_subscriber::{EnvFilter, FmtSubscriber};

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env().add_directive("echo=trace".parse()?))
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Allow passing an address to listen on as the first argument of this
    // program, but otherwise we'll just set up our TCP listener on
    // 127.0.0.1:8080 for connections.
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:3000".to_string());
    let addr = addr.parse::<SocketAddr>()?;

    // Next up we create a TCP listener which will listen for incoming
    // connections. This TCP listener is bound to the address we determined
    // above and must be associated with an event loop.
    let mut listener = TcpListener::bind(&addr).await?;
    // Use `fmt::Debug` impl for `addr` using the `%` symbol
    info!(message = "Listening on", %addr);

    loop {
        // Asynchronously wait for an inbound socket.
        let (mut socket, peer_addr) = listener.accept().await?;

        info!(message = "Got connection from", %peer_addr);

        // And this is where much of the magic of this server happens. We
        // crucially want all clients to make progress concurrently, rather than
        // blocking one on completion of another. To achieve this we use the
        // `tokio::spawn` function to execute the work in the background.
        //
        // Essentially here we're executing a new task to run concurrently,
        // which will allow all of our clients to be processed concurrently.

        tokio::spawn(async move {
            let mut buf = [0; 1024];

            // In a loop, read data from the socket and write the data back.
            loop {
                let n: usize = socket
                    .read(&mut buf)
                    .map(|bytes| {
                        if let Ok(n) = bytes {
                            debug!(bytes_read = n);
                        }

                        bytes
                    })
                    .map_err(|error| {
                        warn!(%error);
                        error
                    })
                    .instrument(trace_span!("read"))
                    .await
                    .expect("failed to read data from socket");

                if n == 0 {
                    return;
                }

                socket
                    .write_all(&buf[0..n])
                    .map(|bytes| {
                        if let Ok(()) = bytes {
                            debug!(bytes_written = n);
                        }

                        bytes
                    })
                    .map_err(|error| {
                        warn!(%error);
                        error
                    })
                    .instrument(trace_span!("write"))
                    .await
                    .expect("failed to write data to socket");

                info!(message = "echo'd data", %peer_addr, size = n);
            }
        })
        .instrument(info_span!("echo", %peer_addr));
    }
}
