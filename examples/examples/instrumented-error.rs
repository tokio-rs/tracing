//! This example demonstrates using the `tracing-error` crate's `SpanTrace` type
//! to attach a trace context to a custom error type.
#![deny(rust_2018_idioms)]
use std::error::Error;
use std::fmt;
use tracing_error::{prelude::*, ErrorLayer};
use tracing_subscriber::prelude::*;

#[derive(Debug)]
struct FooError {
    message: &'static str,
}

// Arbitrary user defined error type for demonstration purposes only
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
    // Results can be instrumented with a `SpanTrace` via the `InstrumentResult` trait
    do_another_thing(42, false).in_current_span()
}

#[tracing::instrument]
fn do_another_thing(
    answer: usize,
    will_succeed: bool,
) -> Result<&'static str, impl Error + Send + Sync + 'static> {
    // Errors can also be instrumented directly via the `InstrumentError` trait
    Err(FooError::new("something broke, lol").in_current_span())
}

#[tracing::instrument]
fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        // The `ErrorLayer` subscriber layer enables the use of `SpanTrace`.
        .with(ErrorLayer::default())
        .init();

    match do_something("hello world") {
        Ok(result) => println!("did something successfully: {}", result),
        Err(e) => {
            eprintln!("printing error chain naively");
            print_naive_spantraces(&e);

            eprintln!("\nprinting error with extract method");
            print_extracted_spantraces(&e);
        }
    };
}

// Iterate through the source errors and check if each error is actually the attached `SpanTrace`
// before printing it
fn print_extracted_spantraces(error: &(dyn Error + 'static)) {
    let mut error = Some(error);
    let mut ind = 0;

    while let Some(err) = error {
        if let Some(spantrace) = err.span_trace() {
            eprintln!("Span Backtrace:\n{}", spantrace);
        } else {
            eprintln!("Error {}: {}", ind, err);
        }

        error = err.source();
        ind += 1;
    }
}

// Iterate through source errors and print all sources as errors uniformly, regardless of whether
// or not that error has a `SpanTrace` attached or not.
fn print_naive_spantraces(error: &(dyn Error + 'static)) {
    let mut error = Some(error);
    let mut ind = 0;
    while let Some(err) = error {
        eprintln!("Error {}: {}", ind, err);
        error = err.source();
        ind += 1;
    }
}
