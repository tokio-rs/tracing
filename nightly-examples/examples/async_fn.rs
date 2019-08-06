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

    write(&mut stream).await;

    Ok(())
}
