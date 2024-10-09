//! NOTE: This is pre-release documentation for the upcoming tracing 0.2.0 ecosystem. For the
//! release examples, please see the `v0.1.x` branch instead.
#![deny(rust_2018_idioms)]
use std::io;
use tracing::error;

fn main() {
    let collector = tracing_subscriber::fmt().with_writer(io::stderr).finish();

    tracing::collect::with_default(collector, || {
        error!("This event will be printed to `stderr`.");
    });
}
