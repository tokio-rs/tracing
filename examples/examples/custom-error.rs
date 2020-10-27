//! This example demonstrates using the `tracing-error` crate's `SpanTrace` type
//! to attach a trace context to a custom error type.
#![deny(rust_2018_idioms)]
use std::error::Error;
use std::fmt;
use tracing_error::{ErrorSubscriber, SpanTrace};
use tracing_subscriber::prelude::*;
#[derive(Debug)]
struct FooError {
    message: &'static str,
    // This struct captures the current `tracing` span context when it is
    // constructed. Later, when we display this error, we will format this
    // captured span trace.
    context: SpanTrace,
}

impl FooError {
    fn new(message: &'static str) -> Self {
        Self {
            message,
            context: SpanTrace::capture(),
        }
    }
}

impl Error for FooError {}

impl fmt::Display for FooError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(self.message)?;
        write!(f, "\n\nspan backtrace:\n{}", self.context)?;
        write!(f, "\n\ndebug span backtrace: {:?}", self.context)?;
        write!(f, "\n\nalt debug span backtrace: {:#?}", self.context)?;
        Ok(())
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
    Err(FooError::new("something broke, lol"))
}

#[tracing::instrument]
fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::subscriber())
        // The `ErrorSubscriber` subscriber layer enables the use of `SpanTrace`.
        .with(ErrorSubscriber::default())
        .init();
    match do_something("hello world") {
        Ok(result) => println!("did something successfully: {}", result),
        Err(e) => eprintln!("error: {}", e),
    };
}
