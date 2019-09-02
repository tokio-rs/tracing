#![deny(rust_2018_idioms)]

//! A proxy that forwards data to another server and forwards that server's
//! responses back to clients.
//!
//! Because the Tokio runtime uses a thread pool, each TCP connection is
//! processed concurrently with all other TCP connections across multiple
//! threads.
//!
//! You can showcase this by running this in one terminal:
//!
//!     cargo +nightly run --example proxy_server
//!
//! This in another terminal
//!
//!     cargo +nightly run --example echo
//!
//! And finally this in another terminal
//!
//!     nc localhost 8081
//!
//! This final terminal will connect to our proxy, which will in turn connect to
//! the echo server, and you'll be able to see data flowing between them.

use futures::{future::try_join, TryFutureExt};
use tokio::{
    self,
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
    prelude::*,
};

use tracing::{debug, debug_span, info, warn};
use tracing_attributes::instrument;
use tracing_futures::Instrument;

use std::{env, net::SocketAddr};

#[instrument]
async fn transfer(
    mut inbound: TcpStream,
    proxy_addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut outbound = TcpStream::connect(&proxy_addr).await?;

    let (mut ri, mut wi) = inbound.split();
    let (mut ro, mut wo) = outbound.split();

    let client_to_server = ri
        .copy(&mut wo)
        .map(|bytes| {
            if let Ok(n) = bytes {
                debug!(bytes_copied = n);
            }

            bytes
        })
        .map_err(|error| {
            warn!(%error);
            error
        })
        .instrument(debug_span!("client_to_server"));
    let server_to_client = ro
        .copy(&mut wi)
        .map(|bytes| {
            if let Ok(n) = bytes {
                debug!(bytes_copied = n);
            }

            bytes
        })
        .map_err(|error| {
            warn!(%error);
            error
        })
        .instrument(debug_span!("server_to_client"));

    let (client_to_server, server_to_client) = try_join(client_to_server, server_to_client).await?;
    info!(
        message = "transfer completed",
        client_to_server, server_to_client
    );

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_filter("proxy_server=trace")
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let listen_addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8081".to_string());
    let listen_addr = listen_addr.parse::<SocketAddr>()?;

    let server_addr = env::args()
        .nth(2)
        .unwrap_or_else(|| "127.0.0.1:3000".to_string());
    let server_addr = server_addr.parse::<SocketAddr>()?;

    let mut listener = TcpListener::bind(&listen_addr).await?.incoming();

    info!("Listening on: {}", listen_addr);
    info!("Proxying to: {}", server_addr);

    while let Some(Ok(inbound)) = listener.next().await {
        match inbound.peer_addr() {
            Ok(addr) => {
                info!(message = "client connected", client_addr = %addr);
            }
            Err(error) => warn!(
                message = "Could not get client information",
                %error
            ),
        }

        let transfer = transfer(inbound, server_addr).map(|r| {
            if let Err(err) = r {
                // Don't panic, maybe the client just disconnected too soon
                debug!(error = %err);
            }
        });

        tokio::spawn(transfer);
    }

    Ok(())
}
