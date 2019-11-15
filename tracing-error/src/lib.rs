mod ctx;
pub mod fmt;
mod layer;
use std::error::Error;
use tracing_core::{dispatcher, Metadata};

pub use self::ctx::*;
pub use self::layer::ErrorLayer;

pub struct ContextError<F = fmt::DefaultFields> {
    inner: Box<dyn Error + Send + Sync>,
    context: Option<Context<F>>,
}

impl<F> ContextError<F> {
    pub fn from_error(error: Box<dyn Error + Send + Sync + 'static>) -> Self
    where
        F: for<'writer> fmt::FormatFields<'writer> + 'static,
    {
        ContextError {
            inner: error,
            context: Context::<F>::current(),
        }
    }

    pub fn context(&self) -> Option<&Context<F>> {
        self.context.as_ref()
    }

    pub fn span_backtrace(&self) -> fmt::SpanBacktrace<&Self> {
        fmt::SpanBacktrace::new(self)
    }
}

pub trait TraceError: Error {
    fn in_context(self) -> ContextError
    where
        Self: Sized + Send + Sync + 'static,
    {
        ContextError::from_error(Box::new(self))
    }

    fn context<F>(&self) -> Option<&Context<F>>
    where
        F: for<'writer> fmt::FormatFields<'writer> + 'static,
        Self: Sized + 'static,
    {
        (self as &dyn Error)
            .downcast_ref::<ContextError<F>>()?
            .context()
    }

    fn span_backtrace(&self) -> fmt::SpanBacktrace<&(dyn Error + 'static)>
    where
        Self: Sized + 'static,
    {
        fmt::SpanBacktrace::new(self as &dyn Error)
    }
}

impl<T> TraceError for T where T: Error {}

impl<F> std::fmt::Display for ContextError<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl<F> std::fmt::Debug for ContextError<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextError")
            .field("inner", &self.inner)
            .field("context", &self.context)
            .finish()
    }
}

impl<F> Error for ContextError<F> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner.source()
    }
}
