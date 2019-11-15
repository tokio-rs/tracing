use super::fmt::{DefaultFields, FormatFields};
use super::{Context, ContextSpan};
use std::any::TypeId;
use std::marker::PhantomData;
use tracing_core::{span, Subscriber};
use tracing_subscriber::{
    fmt::FormattedFields,
    layer::{self, Layer},
    registry::LookupSpan,
};

pub struct ErrorLayer<S, F = DefaultFields> {
    format: F,
    get_context: GetContext<F>,
    _subscriber: PhantomData<fn(S)>,
}

pub(crate) struct GetContext<F>(fn(&tracing_core::Dispatch) -> Option<Context<F>>);

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
        span.extensions_mut()
            .insert(FormattedFields::<F>::new(fields));
    }

    unsafe fn downcast_raw(&self, id: TypeId) -> Option<*const ()> {
        match id {
            id if id == TypeId::of::<Self>() => Some(self as *const _ as *const ()),
            id if id == TypeId::of::<GetContext<F>>() => {
                Some(&self.get_context as *const _ as *const ())
            }
            _ => None,
        }
    }
}

impl<S, F> ErrorLayer<S, F>
where
    F: for<'writer> FormatFields<'writer> + 'static,
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    pub fn new(format: F) -> Self {
        Self {
            format,
            get_context: GetContext(Self::get_context),
            _subscriber: PhantomData,
        }
    }

    fn get_context(dispatch: &tracing_core::Dispatch) -> Option<Context<F>> {
        let subscriber = dispatch
            .downcast_ref::<S>()
            .expect("subscriber should downcast to expected type; this is a bug!");
        let curr_span = subscriber.current_span();
        let curr_id = curr_span.id()?;
        let span = subscriber
            .span(curr_id)
            .expect("registry should have a span for the current ID");
        let mut ctx = Context::new();
        let fields = span
            .extensions()
            .get::<FormattedFields<F>>()
            .map(|f| f.fmt_fields.clone())
            .unwrap_or_default();
        ctx.push(span.metadata(), fields);
        for span in span.parents() {
            let fields = span
                .extensions()
                .get::<FormattedFields<F>>()
                .map(|f| f.fmt_fields.clone())
                .unwrap_or_default();
            ctx.push(span.metadata(), fields);
        }
        Some(ctx)
    }
}

impl<F> GetContext<F>
where
    F: for<'writer> FormatFields<'writer> + 'static,
{
    pub(crate) fn get_context(&self, dispatch: &tracing_core::Dispatch) -> Option<Context<F>> {
        (self.0)(dispatch)
    }
}

impl<S> Default for ErrorLayer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn default() -> Self {
        Self::new(DefaultFields::default())
    }
}
