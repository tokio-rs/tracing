#![deny(rust_2018_idioms)]
use std::io;
use tracing::error;
use tracing_subscriber::fmt;

fn main() {
    let subscriber = fmt::Subscriber::builder()
        .with_writer(io::stderr)
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        error!("This event will be printed to `stderr`.");
    });
}
