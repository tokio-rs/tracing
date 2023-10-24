//! Compare to the example given in the documentation for the `std::dbg` macro.
#![deny(rust_2018_idioms)]

use tracing_macros::dbg;
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter};

fn factorial(n: u32) -> u32 {
    if dbg!(n <= 1) {
        dbg!(1)
    } else {
        dbg!(n * factorial(n - 1))
    }
}

fn main() {
    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::TRACE.into()))
        .with(fmt::Layer::new());

    tracing::subscriber::set_global_default(subscriber).expect("Unable to set a global subscriber");
    dbg!(factorial(4));
}
