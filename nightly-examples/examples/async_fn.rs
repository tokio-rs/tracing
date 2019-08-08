//! Demonstrates using the `trace` attribute macro to instrument `async`
//! functions.
//!
//! This is based on the [`hello_world`] example from `tokio`. and implements a
//! simple client that opens a TCP stream, writes "hello world\n", and closes
//! the connection.
//!
//! You can test this out by running:
//!
//!     ncat -l 6142
//!
//! And then in another terminal run:
//!
//!     cargo +nightly run --example async_fn
//!
//! [`hello_world`]: https://github.com/tokio-rs/tokio/blob/132e9f1da5965530b63554d7a1c59824c3de4e30/tokio/examples/hello_world.rs
#![feature(async_await)]
use tokio;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use tracing::info;
use tracing_attributes::trace;

use std::{io, error::Error, net::SocketAddr};

#[instrument]
async fn connect(addr: &SocketAddr) -> io::Result<TcpStream> {
    let stream = TcpStream::connect(&addr).await;
    tracing::info!("created stream");
    stream
}

#[instrument]
async fn write(stream: &mut TcpStream) -> io::Result<usize> {
    let result = stream.write(b"hello world\n").await;
    info!("wrote to stream; success={:?}", result.is_ok());
    result
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "127.0.0.1:6142".parse()?;

    let subscriber = tracing_fmt::FmtSubscriber::builder()
        .with_filter(tracing_fmt::filter::EnvFilter::from("async_fn=trace"))
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    // Open a TCP stream to the socket address.
    //
    // Note that this is the Tokio TcpStream, which is fully async.
    let mut stream = connect(&addr).await?;

    write(&mut stream).await?;

    Ok(())
}
