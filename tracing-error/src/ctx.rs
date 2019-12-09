use crate::{fmt, layer::GetContext};
use std::marker::PhantomData;
use tracing_core::{dispatcher, Metadata};

#[derive(Clone)]
pub struct Context<F = fmt::DefaultFields> {
    context: Vec<ContextSpan>,
    _fmt: PhantomData<fn(F)>,
}

pub struct ContextIter<'a>(std::slice::Iter<'a, ContextSpan>);

#[derive(Clone, Debug)]
pub struct ContextSpan {
    metadata: &'static Metadata<'static>,
    fields: String,
}

// === impl Context ===

impl<F> Context<F> {
    pub fn current() -> Option<Self>
    where
        F: for<'writer> fmt::FormatFields<'writer> + 'static,
    {
        dispatcher::get_default(|curr| curr.downcast_ref::<GetContext<F>>()?.get_context(&curr))
    }

    pub(crate) fn new() -> Self {
        Self {
            context: Vec::new(),
            _fmt: PhantomData,
        }
    }

    pub(crate) fn push(
        &mut self,
        metadata: &'static Metadata<'static>,
        fields: String,
    ) -> &mut Self {
        self.context.push(ContextSpan { metadata, fields });
        self
    }

    pub fn iter(&self) -> ContextIter<'_> {
        ContextIter(self.context[..].iter())
    }

    pub fn span_backtrace(&self) -> fmt::SpanBacktrace<&Self> {
        fmt::SpanBacktrace::new(self)
    }
}

impl<F> std::fmt::Debug for Context<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut map = f.debug_map();
        for ContextSpan {
            ref metadata,
            ref fields,
        } in self.context.iter()
        {
            map.entry(&metadata.name(), &format_args!("{}", fields));
        }
        map.finish()
    }
}

impl<'a, F> IntoIterator for &'a Context<F> {
    type IntoIter = ContextIter<'a>;
    type Item = &'a ContextSpan;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// === impl ContextSpan ===

impl ContextSpan {
    pub fn metadata(&self) -> &'static Metadata<'static> {
        self.metadata
    }

    pub fn name(&self) -> &'static str {
        self.metadata.name()
    }

    pub fn fields(&self) -> Option<&str> {
        if self.fields == "" {
            return None;
        }

        Some(self.fields.as_ref())
    }
}

// === impl ContextIter ===

impl<'a> Iterator for ContextIter<'a> {
    type Item = &'a ContextSpan;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}
