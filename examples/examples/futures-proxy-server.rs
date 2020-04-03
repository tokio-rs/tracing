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

use clap::{arg_enum, value_t, App, Arg, ArgMatches};
use futures::{future::try_join, prelude::*};
use std::net::SocketAddr;
use tokio::{
    self, io,
    net::{TcpListener, TcpStream},
};
use tracing::{debug, debug_span, info, warn};
use tracing_attributes::instrument;
use tracing_futures::Instrument;

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

arg_enum! {
    #[derive(PartialEq, Debug)]
    pub enum LogFormat {
        Plain,
        Json,
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let matches = App::new("Proxy Server Example")
        .version("1.0")
        .arg(
            Arg::with_name("log_format")
                .possible_values(&LogFormat::variants())
                .case_insensitive(true)
                .long("log_format")
                .value_name("log_format")
                .help("Formatting of the logs")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("listen_addr")
                .long("listen_addr")
                .help("Address to listen on")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("server_addr")
                .long("server_addr")
                .help("Address to proxy to")
                .takes_value(false)
                .required(false),
        )
        .get_matches();

    set_global_default(&matches)?;

    let listen_addr = matches.value_of("listen_addr").unwrap_or("127.0.0.1:8081");
    let listen_addr = listen_addr.parse::<SocketAddr>()?;

    let server_addr = matches.value_of("server_addr").unwrap_or("127.0.0.1:3000");
    let server_addr = server_addr.parse::<SocketAddr>()?;

    let mut listener = TcpListener::bind(&listen_addr).await?;

    info!("Listening on: {}", listen_addr);
    info!("Proxying to: {}", server_addr);

    while let Ok((inbound, client_addr)) = listener.accept().await {
        info!(client.addr = %client_addr, "client connected");

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

fn set_global_default(matches: &ArgMatches<'_>) -> Result<(), Error> {
    let filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive("proxy_server=trace".parse()?);
    let subscriber = tracing_subscriber::fmt().with_env_filter(filter);
    match value_t!(matches, "log_format", LogFormat).unwrap_or(LogFormat::Plain) {
        LogFormat::Json => {
            subscriber.json().try_init()?;
        }
        LogFormat::Plain => {
            subscriber.try_init()?;
        }
    }
    Ok(())
}
