use super::{Context, ContextError, ContextSpan, TraceError};
use std::{error::Error, fmt};
pub use tracing_subscriber::fmt::format::{DefaultFields, FormatFields};

#[derive(Debug)]
pub struct SpanBacktrace<T> {
    inner: T,
}

impl<T> SpanBacktrace<T> {
    pub(crate) fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<'a> fmt::Display for SpanBacktrace<&'a Context> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unimplemented!("jane, do this part however you want (or get rid of this thing entirely)!")
    }
}

impl<'a> fmt::Display for SpanBacktrace<&'a dyn Error> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unimplemented!("jane, do this part however you want (or get rid of this thing entirely)!")
    }
}
