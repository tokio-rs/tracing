//! This example demonstrates using the `tracing-error` crate's `SpanTrace` type
//! to attach a trace context to a custom error type.
#![deny(rust_2018_idioms)]
use std::error::Error;
use std::fmt;
use tracing_error::{ErrorLayer, Instrument as _, SpanTrace, SpanTraceExt as _};
use tracing_subscriber::{fmt::Layer as FmtLayer, prelude::*, registry::Registry};

#[derive(Debug)]
struct FooError {
    message: &'static str,
}

impl FooError {
    fn new(message: &'static str) -> Self {
        Self { message }
    }
}

impl Error for FooError {}

impl fmt::Display for FooError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(self.message)
    }
}

#[tracing::instrument]
fn do_something(foo: &str) -> Result<&'static str, impl Error + Send + Sync + 'static> {
    do_another_thing(42, false)
}

#[tracing::instrument]
fn do_another_thing(
    answer: usize,
    will_succeed: bool,
) -> Result<&'static str, impl Error + Send + Sync + 'static> {
    Err(FooError::new("something broke, lol")).in_current_span()
}

#[tracing::instrument]
fn main() {
    let subscriber = Registry::default()
        .with(FmtLayer::default())
        // The `ErrorLayer` subscriber layer enables the use of `SpanTrace`.
        .with(ErrorLayer::default());
    tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");
    match do_something("hello world") {
        Ok(result) => println!("did something successfully: {}", result),
        Err(e) => {
            let trait_object: Box<dyn Error + 'static> = Box::new(e);
            let mut error = Some(trait_object.as_ref());
            while let Some(err) = error {
                if let Some(spantrace) = err.spantrace() {
                    eprintln!("found a spantrace!:\n{}", spantrace);
                }
                eprintln!("error: {}", err);
                error = err.source();
            }
        }
    };
}
