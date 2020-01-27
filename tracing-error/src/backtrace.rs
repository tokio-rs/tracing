use super::try_bool;
use crate::layer::WithContext;
use std::fmt;
use tracing::{Metadata, Span};

#[derive(Clone)]
pub struct SpanTrace {
    span: Span,
}

// === impl SpanTrace ===

impl SpanTrace {
    pub fn capture() -> Self {
        SpanTrace {
            span: Span::current(),
        }
    }

    pub fn with_spans(&self, f: impl FnMut(&'static Metadata<'static>, &str) -> bool) {
        self.span.with_subscriber(|(id, s)| {
            if let Some(getcx) = s.downcast_ref::<WithContext>() {
                getcx.with_context(s, id, f);
            }
        });
    }
}

impl fmt::Debug for SpanTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut err = None;
        let mut span = 0;

        writeln!(f, "span backtrace:")?;
        self.with_spans(|metadata, fields| {
            try_bool!(
                writeln!(f, "{:>4}: {}::{}", span, metadata.target(), metadata.name()),
                err
            );
            if !fields.is_empty() {
                try_bool!(writeln!(f, "           with {}", fields), err);
            }
            if let Some((file, line)) = metadata
                .file()
                .and_then(|file| metadata.line().map(|line| (file, line)))
            {
                try_bool!(writeln!(f, "             at {}:{}", file, line), err);
            }

            span += 1;
            true
        });

        match err {
            Some(e) => Err(e),
            _ => Ok(()),
        }
    }
}

impl fmt::Display for SpanTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}
