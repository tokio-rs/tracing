mod ctx;
pub mod fmt;
mod layer;
use std::error::Error;
use tracing::{dispatcher, Metadata};

pub use self::ctx::*;
pub use self::layer::ErrorLayer;

pub struct ContextError {
    inner: Box<dyn Error + Send + Sync>,
    context: Context,
}

impl ContextError {
    pub fn from_error(error: Box<dyn Error + Send + Sync + 'static>) -> Self {
        ContextError {
            inner: error,
            context: Context::current(),
        }
    }

    pub fn context(&self) -> &Context {
        &self.context
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

    fn context(&self) -> Option<&Context>
    where
        Self: Sized + 'static,
    {
        let cx = (self as &dyn Error)
            .downcast_ref::<ContextError>()?
            .context();
        Some(cx)
    }

    fn span_backtrace(&self) -> fmt::SpanBacktrace<&(dyn Error + 'static)>
    where
        Self: Sized + 'static,
    {
        fmt::SpanBacktrace::new(self as &dyn Error)
    }
}

impl<T> TraceError for T where T: Error {}

impl std::fmt::Display for ContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl std::fmt::Debug for ContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextError")
            .field("inner", &self.inner)
            .field("context", &self.context)
            .finish()
    }
}

impl Error for ContextError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner.source()
    }
}
