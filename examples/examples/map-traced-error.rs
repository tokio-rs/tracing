//! An example on composing errors inside of a `TracedError`, such that the
//! SpanTrace captured is captured when creating the inner error, but still wraps
//! the outer error.
#![deny(rust_2018_idioms)]
#![allow(clippy::try_err)]
use std::error::Error;
use std::fmt;
use tracing_error::{prelude::*, ErrorSubscriber, TracedError};
use tracing_subscriber::prelude::*;

fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::subscriber())
        // The `ErrorSubscriber` subscriber enables the use of `SpanTrace`.
        .with(ErrorSubscriber::default())
        .init();

    let e = do_something().unwrap_err();
    print_extracted_spantraces(&e);
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

#[tracing::instrument]
fn do_something() -> Result<(), TracedError<OuterError>> {
    do_the_real_stuff().map_err(TracedError::err_into)
}

#[tracing::instrument]
fn do_the_real_stuff() -> Result<(), TracedError<InnerError>> {
    Err(InnerError).map_err(TracedError::from)
}

#[derive(Debug)]
struct OuterError {
    source: InnerError,
}

impl Error for OuterError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

impl fmt::Display for OuterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "outer error message")
    }
}

impl From<InnerError> for OuterError {
    fn from(source: InnerError) -> Self {
        Self { source }
    }
}

#[derive(Debug)]
struct InnerError;

impl Error for InnerError {}

impl fmt::Display for InnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "inner error message")
    }
}
