#![deny(rust_2018_idioms)]
use std::error::Error;
use tracing::debug;
use tracing_subscriber::{fmt::FmtLayer, registry::Registry, Layer};
use tracing_error::{ErrorLayer, TraceError};

#[tracing::instrument]
fn do_something(foo: &str) -> Result<&'static str, impl Error + Send + Sync> {
    do_another_thing(42, false)
}

#[tracing::instrument]
fn do_another_thing(answer: usize, will_succeed: bool) -> Result<&'static str, impl Error + Send + Sync> {
    Err(std::io::Error::new(std::io::ErrorKind::Other, "something broke lol").in_context())
}

#[tracing::instrument]
fn main() {
    let subscriber = ErrorLayer::default()
        .and_then(FmtLayer::default())
        .with_subscriber(Registry::default());
    tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");
    match do_something("hello world") {
        Ok(result) => println!("did something successfully: {}", result),
        Err(e) => eprintln!("ERROR: {}", e),
    };
}