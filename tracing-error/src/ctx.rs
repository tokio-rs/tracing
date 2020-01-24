use crate::{fmt, layer::WithContext};
use std::marker::PhantomData;
use tracing::{Span, Metadata};

#[derive(Clone)]
pub struct Context {
    span: Span,
}

#[derive(Clone, Debug)]
pub struct ContextSpan {
    metadata: &'static Metadata<'static>,
    fields: String,
}

// === impl Context ===

impl Context {
    pub fn current() -> Self {
        Context {
            span: Span::current(),
        }
    }

    pub fn span_backtrace(&self) -> fmt::SpanBacktrace<&Self> {
        unimplemented!()
    }

    pub fn with_spans<T>(&self, f: impl FnMut(&'static Metadata<'static>, &str) -> bool) {
        self.span.with_subscriber(|(id, s)| {
            if let Some(getcx) = s.downcast_ref::<WithContext>() {
                getcx.with_context(s, id, f);
            }
        });

    }
}

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unimplemented!("jane, do whatever you like with this :)")
    }
}