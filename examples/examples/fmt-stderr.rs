#![deny(rust_2018_idioms)]
use std::io;
use tracing::error;

fn main() {
    let subscriber = tracing_subscriber::fmt().with_writer(io::stderr).finish();

    tracing::collector::with_default(subscriber, || {
        error!("This event will be printed to `stderr`.");
    });
}
