#![deny(rust_2018_idioms)]
use std::io;
use tracing::error;

fn main() {
    let subscriber = tracing_fmt::FmtSubscriber::builder()
        .with_writer(io::stderr)
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        error!("This event will be printed to `stderr`.");
    });
}
