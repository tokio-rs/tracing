#![deny(rust_2018_idioms)]
use std::error::Error;
use std::fmt::{self, Write as _};
use tracing_error::{ErrorLayer, SpanTrace};
use tracing_subscriber::{fmt::Layer as FmtLayer, prelude::*, registry::Registry};

#[tracing::instrument]
fn do_something(foo: &str) -> Result<&'static str, impl Error + Send + Sync + 'static> {
    match do_another_thing(42, false) {
        Ok(i) => Ok(i),
        Err(e) => Err(FooError::new("something broke, lol", e)),
    }
}
#[derive(Debug)]
struct FooError {
    message: &'static str,
    source: Box<dyn Error + Send + Sync + 'static>,
    context: SpanTrace,
}

impl FooError {
    fn new(
        message: &'static str,
        source: impl Into<Box<dyn Error + Send + Sync + 'static>>,
    ) -> Self {
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
        .with(ErrorLayer::default());
    tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");
    match do_something("hello world") {
        Ok(result) => println!("did something successfully: {}", result),
        Err(e) => eprintln!("error: {}", e),
    };
}
