//! A proxy that forwards data to another server and forwards that server's
//! responses back to clients.
//!
//! Because the Tokio runtime uses a thread pool, each TCP connection is
//! processed concurrently with all other TCP connections across multiple
//! threads.
//!
//! You can showcase this by running this in one terminal:
//!
//!     cargo run --example proxy_server -- --log_format=(plain|json)
//!
//! This in another terminal
//!
//!     cargo run --example echo
//!
//! And finally this in another terminal
//!
//!     nc localhost 8081
//!
//! This final terminal will connect to our proxy, which will in turn connect to
//! the echo server, and you'll be able to see data flowing between them.
//!
//! This example has been taken and modified from here :
//! https://raw.githubusercontent.com/tokio-rs/tokio/master/tokio/examples/proxy.rs

#![deny(rust_2018_idioms)]

use argh::FromArgs;
use futures::{future::try_join, prelude::*};
use std::net::SocketAddr;
use tokio::{
    self, io,
    net::{TcpListener, TcpStream},
};
use tracing::{debug, debug_span, info, instrument, warn, Instrument as _};

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[instrument]
async fn transfer(mut inbound: TcpStream, proxy_addr: SocketAddr) -> Result<(), Error> {
    let mut outbound = TcpStream::connect(&proxy_addr).await?;

    let (mut ri, mut wi) = inbound.split();
    let (mut ro, mut wo) = outbound.split();

    let client_to_server = io::copy(&mut ri, &mut wo)
        .map_ok(|bytes_copied| {
            info!(bytes_copied);
            bytes_copied
        })
        .map_err(|error| {
            warn!(%error);
            error
        })
        .instrument(debug_span!("client_to_server"));
    let server_to_client = io::copy(&mut ro, &mut wi)
        .map_ok(|bytes_copied| {
            info!(bytes_copied);
            bytes_copied
        })
        .map_err(|error| {
            warn!(%error);
            error
        })
        .instrument(debug_span!("server_to_client"));

    let (client_to_server, server_to_client) = try_join(client_to_server, server_to_client).await?;
    info!(client_to_server, server_to_client, "transfer completed",);

    Ok(())
}

#[derive(FromArgs)]
#[argh(description = "Proxy server example")]
pub struct Args {
    /// how to format the logs.
    #[argh(option, default = "LogFormat::Plain")]
    log_format: LogFormat,

    /// address to listen on.
    #[argh(option, default = "default_listen_addr()")]
    listen_addr: SocketAddr,

    /// address to proxy to.
    #[argh(option, default = "default_server_addr()")]
    server_addr: SocketAddr,
}

#[derive(Eq, PartialEq, Debug)]
pub enum LogFormat {
    Plain,
    Json,
}

fn default_listen_addr() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 8081))
}

fn default_server_addr() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 3000))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args: Args = argh::from_env();
    set_global_default(args.log_format)?;

    let listener = TcpListener::bind(&args.listen_addr).await?;

    info!("Listening on: {}", args.listen_addr);
    info!("Proxying to: {}", args.server_addr);

    while let Ok((inbound, client_addr)) = listener.accept().await {
        info!(client.addr = %client_addr, "client connected");

        let transfer = transfer(inbound, args.server_addr).map(|r| {
            if let Err(err) = r {
                // Don't panic, maybe the client just disconnected too soon
                debug!(error = %err);
            }
        });

        tokio::spawn(transfer);
    }

    Ok(())
}

fn set_global_default(format: LogFormat) -> Result<(), Error> {
    let filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive(concat!(module_path!(), "=trace").parse()?);
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    match format {
        LogFormat::Json => {
            builder.json().try_init()?;
        }
        LogFormat::Plain => {
            builder.try_init()?;
        }
    }
    Ok(())
}

impl std::str::FromStr for LogFormat {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            s if s.eq_ignore_ascii_case("plain") => Ok(Self::Plain),
            s if s.eq_ignore_ascii_case("json") => Ok(Self::Json),
            _ => Err("expected either `plain` or `json`"),
        }
    }
}
