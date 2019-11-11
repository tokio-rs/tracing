use tracing_subscriber::{layer::{self, Layer}, fmt::{FormattedFields, format::{self, FormatFields}}, registry::LookupSpan};
use tracing_core::{span, Subscriber};
use std::marker::PhantomData;
use super::Context;

#[derive(Debug)]
pub struct ErrorLayer<S, F = format::DefaultFields> {
    format: F,
    _s: PhantomData<fn(S)>,
}

pub(crate) struct LayerMarker<F> {
    _f: PhantomData<fn(F)>
}

impl<S, F> Layer<S> for ErrorLayer<S, F>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    F: for<'writer> FormatFields<'writer> + 'static,
{
    /// Notifies this layer that a new span was constructed with the given
    /// `Attributes` and `Id`.
    fn new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: layer::Context<'_, S>) {
        let span = ctx.span(id).expect("span must already exist!");
        if span.extensions().get::<FormattedFields<F>>().is_some() {
            return;
        }
        let mut fields = String::new();
        self.format.format_fields(&mut fields, attrs);
        span.extensions_mut().insert(FormattedFields::<F>::new(fields));
    }


}

impl<S, F> ErrorLayer<S, F>
where
    F: for<'writer> FormatFields<'writer>,
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    pub(crate) fn current_context(&self, dispatch: &tracing_core::Dispatch) -> Option<Context> {
        let subscriber = dispatch.downcast_ref::<S>()?;
        let curr_id = subscriber.current_span().id()?;
        let span = subscriber.span(curr_id).expect("registry should have a span for the current ID");
        let mut ctx = Context::new();
        for span in span.parents() {
            unimplemented!("add to ctx")
        }
        Some(ctx)
    }
}