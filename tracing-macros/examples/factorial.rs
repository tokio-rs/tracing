//! Compare to the example given in the documentation for the `std::dbg` macro.
#![deny(rust_2018_idioms)]

#[macro_use]
extern crate tracing;
#[macro_use]
extern crate tracing_macros;
use env_logger;
use tracing_log;

fn factorial(n: u32) -> u32 {
    if dbg!(n <= 1) {
        dbg!(1)
    } else {
        dbg!(n * factorial(n - 1))
    }
}

fn main() {
    env_logger::Builder::new().parse("trace").init();
    let subscriber = tracing_log::TraceLogger::new();

    tracing::subscriber::with_default(subscriber, || dbg!(factorial(4)));
}
