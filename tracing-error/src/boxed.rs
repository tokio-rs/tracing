//! Dynamic wrapper type and accompanying traits for instrumenting arbitrary error types

use crate::SpanTrace;
use std::error::Error;
use std::fmt::{self, Debug, Display};

/// A wrapper type for Errors that bundles a `SpanTrace` with an inner `Error` type.
pub struct TracedError {
    inner: ErrorImpl,
}

pub(crate) struct ErrorImpl {
    pub(crate) span_trace: SpanTrace,
    error: Box<dyn Error + Send + Sync + 'static>,
}

impl TracedError {
    fn new<E>(error: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self {
            inner: ErrorImpl {
                span_trace: SpanTrace::capture(),
                error: Box::new(error),
            },
        }
    }
}

impl Error for TracedError {
    fn source<'a>(&'a self) -> Option<&'a (dyn Error + 'static)> {
        Some(&self.inner)
    }
}

impl Debug for TracedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.inner.error, f)
    }
}

impl Display for TracedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.inner.error, f)
    }
}

impl Error for ErrorImpl {
    fn source<'a>(&'a self) -> Option<&'a (dyn Error + 'static)> {
        self.error.source()
    }
}

impl Debug for ErrorImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("span backtrace:\n")?;
        Debug::fmt(&self.span_trace, f)
    }
}

impl Display for ErrorImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("span backtrace:\n")?;
        Display::fmt(&self.span_trace, f)
    }
}

/// Extension trait for instrumenting errors with `SpanTrace`s
pub trait InstrumentError {
    /// The type of the wrapped error after instrumentation
    type Instrumented;

    /// Instrument an Error by bundling it with a SpanTrace
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tracing_error::boxed::{TracedError, InstrumentError};
    ///
    /// fn wrap_error(e: impl std::error::Error + Send + Sync + 'static) -> TracedError {
    ///     e.in_current_span()
    /// }
    /// ```
    fn in_current_span(self) -> Self::Instrumented;
}

/// Extension trait for instrumenting errors in `Result`s with `SpanTrace`s
pub trait InstrumentResult<T> {
    /// The type of the wrapped error after instrumentation
    type Instrumented;

    /// Instrument an Error by bundling it with a SpanTrace
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use std::{io, fs};
    /// use tracing_error::boxed::{TracedError, InstrumentResult};
    ///
    /// # fn fallible_fn() -> io::Result<()> { fs::read_dir("......").map(drop) };
    ///
    /// fn do_thing() -> Result<(), TracedError> {
    ///     fallible_fn().in_current_span()
    /// }
    /// ```
    fn in_current_span(self) -> Result<T, Self::Instrumented>;
}

impl<T, E> InstrumentResult<T> for Result<T, E>
where
    E: InstrumentError,
{
    type Instrumented = <E as InstrumentError>::Instrumented;

    fn in_current_span(self) -> Result<T, Self::Instrumented> {
        self.map_err(E::in_current_span)
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
