use crate::SpanTrace;
use crate::{InstrumentError, InstrumentResult, SpanTraceExtract};
use std::error::Error;
use std::fmt::{self, Debug, Display};

/// A wrapper type for Errors that bundles a SpanTrace with an inner `Error` type.
///
/// # Notes
///
/// This type does not print the wrapped `SpanTrace` in either its `Debug` or `Display`
/// implementations. The `SpanTrace` must be extracted via the `SpanTraceExtract` trait in order to
/// be printed.
pub struct TracedError {
    spantrace: SpanTrace,
    inner: Box<dyn Error + Send + Sync + 'static>,
}

impl TracedError {
    fn new<E>(error: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self {
            spantrace: SpanTrace::capture(),
            inner: Box::new(error),
        }
    }
}

impl Error for TracedError {
    fn source<'a>(&'a self) -> Option<&'a (dyn Error + 'static)> {
        self.inner.source()
    }
}

impl Debug for TracedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}

impl Display for TracedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl<E> InstrumentError for E
where
    E: Error + Send + Sync + 'static,
{
    type Instrumented = TracedError;

    fn in_current_span(self) -> Self::Instrumented {
        TracedError::new(self)
    }
}

impl<T, E> InstrumentResult<T> for Result<T, E>
where
    E: Error + Send + Sync + 'static,
{
    type Instrumented = TracedError;

    fn in_current_span(self) -> Result<T, Self::Instrumented> {
        self.map_err(TracedError::new)
    }
}

impl SpanTraceExtract for &(dyn Error + 'static) {
    fn span_trace(&self) -> Option<&SpanTrace> {
        self.downcast_ref::<TracedError>().map(|e| &e.spantrace)
    }
}
