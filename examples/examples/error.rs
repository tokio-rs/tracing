//! This example demonstrates using the `tracing-error` crate's `SpanTrace` type
//! to attach a trace context to a custom error type.
#![deny(rust_2018_idioms)]
use std::error::Error;
use std::fmt;
use tracing_error::{ErrorLayer, SpanTrace};
use tracing_subscriber::{fmt::Layer as FmtLayer, prelude::*, registry::Registry};

type BoxError = Box<dyn Error + Send + Sync + Static>;
#[derive(Debug)]
struct FooError {
    message: &'static str,
    source: BoxError,
    // This struct captures the current `tracing` span context when it is
    // constructed. Later, when we display this error, we will format this
    // captured span trace.
    context: SpanTrace,
}

impl FooError {
    fn new(message: &'static str, source: impl Into<BoxError>) -> Self {
        Self {
            message,
            source: source.into(),
            context: SpanTrace::capture(),
        }
    }
}

impl Error for FooError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}

impl fmt::Display for FooError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(self.message)?;
        write!(f, "\n{}", self.context)
    }
}

#[tracing::instrument]
fn do_something(foo: &str) -> Result<&'static str, impl Error + Send + Sync + 'static> {
    match do_another_thing(42, false) {
        Ok(i) => Ok(i),
        Err(e) => Err(FooError::new("something broke, lol", e)),
    }
}

#[tracing::instrument]
fn do_another_thing(
    answer: usize,
    will_succeed: bool,
) -> Result<&'static str, impl Error + Send + Sync + 'static> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "something else broke!",
    ))
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
        Err(e) => eprintln!("error: {}", e),
    };
}
