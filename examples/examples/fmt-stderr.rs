#![deny(rust_2018_idioms)]
use std::io;
use tracing::error;
use tracing_subscriber::{FmtLayer, Layer, Registry};

fn main() {
    let subscriber = FmtLayer::builder()
        .with_writer(io::stderr)
        .build()
        .with_subscriber(Registry::default());

    tracing::subscriber::with_default(subscriber, || {
        error!("This event will be printed to `stderr`.");
    });
}
