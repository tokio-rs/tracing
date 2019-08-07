//! Demonstrates using the `trace` attribute macro to instrument `async`
//! functions.
//!
//! This is based on the [`hello_world`] example from `tokio`.
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
//!     cargo run --example echo
//!
//! and in another terminal you can run:
//!
//!     cargo run --example connect 127.0.0.1:8080
//!
//! Each line you type in to the `connect` terminal should be echo'd back to
//! you! If you open up multiple terminals running the `connect` example you
//! should be able to see them all make progress simultaneously.
//!
//! [`hello_world`]: https://github.com/tokio-rs/tokio/blob/132e9f1da5965530b63554d7a1c59824c3de4e30/tokio/examples/hello_world.rs
#![feature(async_await)]
use tokio;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use std::error::Error;

#[tracing_proc_macros::trace]
async fn connect(addr: &std::net::SocketAddr) -> Result<TcpStream, std::io::Error> {
    let stream = TcpStream::connect(&addr).await;
    tracing::info!("created stream");
    stream
}

#[tracing_proc_macros::trace]
async fn write(stream: &mut TcpStream) -> Result<usize, std::io::Error> {
    let result = stream.write(b"hello world\n").await;
    tracing::info!("wrote to stream; success={:?}", result.is_ok());
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

    write(&mut stream).await
}
