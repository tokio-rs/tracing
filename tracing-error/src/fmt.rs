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

impl<'a> fmt::Display for SpanBacktrace<&'a ContextSpan> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let metadata = self.inner.metadata();
        write!(f, "   in {}::{}", metadata.target(), metadata.name())?;

        if let Some(fields) = self.inner.fields() {
            write!(f, ", {}", fields)?;
        }

        if let (Some(file), Some(line)) = (metadata.file(), metadata.line()) {
            write!(f, "\n\tat {}:{}", file, line)?;
        }

        Ok(())
    }
}

impl<'a, F> fmt::Display for SpanBacktrace<&'a Context<F>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut spans = self.inner.iter();
        if let Some(span) = spans.next() {
            write!(f, "{}", SpanBacktrace::new(span))?;

            for span in spans {
                write!(f, "\n{}", SpanBacktrace::new(span))?;
            }
        }

        Ok(())
    }
}

impl<'a, F> fmt::Display for SpanBacktrace<&'a ContextError<F>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.inner)?;
        if let Some(ctx) = self.inner.context() {
            writeln!(f, "{}", ctx.span_backtrace())?;
        }

        let mut source = self.inner.source();
        while let Some(err) = source {
            writeln!(f, "caused by: {}", err)?;
            if let Some(ctx) = err
                .downcast_ref::<ContextError>()
                .and_then(ContextError::context)
            {
                writeln!(f, "{}", ctx.span_backtrace())?;
            }
            source = err.source();
        }

        Ok(())
    }
}

impl<'a> fmt::Display for SpanBacktrace<&'a (dyn Error + 'static)> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.inner)?;
        if let Some(ctx) = self
            .inner
            .downcast_ref::<ContextError>()
            .and_then(ContextError::context)
        {
            writeln!(f, "{}", ctx.span_backtrace())?;
        }

        let mut source = self.inner.source();
        while let Some(err) = source {
            writeln!(f, "caused by: {}", err)?;
            if let Some(ctx) = err
                .downcast_ref::<ContextError>()
                .and_then(ContextError::context)
            {
                writeln!(f, "{}", ctx.span_backtrace())?;
            }
            source = err.source();
        }

        Ok(())
    }
}
