use super::fmt::{DefaultFields, FormatFields};
use super::{Context, ContextSpan};
use std::marker::PhantomData;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};
use tracing_core::{span, Subscriber};
use tracing_subscriber::{
    fmt::FormattedFields,
    layer::{self, Layer},
    registry::LookupSpan,
};

pub struct ErrorLayer<F = DefaultFields> {
    format: F,
    get_context: AtomicPtr<()>,
}

impl<S, F> Layer<S> for ErrorLayer<F>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    F: for<'writer> FormatFields<'writer> + 'static,
{
    fn register_callsite(
        &self,
        _: &'static tracing_core::Metadata<'static>,
    ) -> tracing_core::subscriber::Interest {
        self.get_context.compare_exchange(
            ptr::null_mut(),
            (Self::get_context::<S>) as fn(&tracing_core::Dispatch) -> Option<Context<F>>
                as *const () as *mut _,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
        tracing_core::subscriber::Interest::always()
    }

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
}

impl<F> ErrorLayer<F>
where
    F: for<'writer> FormatFields<'writer> + 'static,
{
    pub fn new(format: F) -> Self {
        Self {
            format,
            get_context: AtomicPtr::new(ptr::null_mut()),
        }
    }

    fn get_context<S>(dispatch: &tracing_core::Dispatch) -> Option<Context<F>>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
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

    pub(crate) fn current_context(&self, dispatch: &tracing_core::Dispatch) -> Option<Context<F>> {
        let get_context = unsafe {
            self.get_context
                .load(Ordering::Acquire)
                .cast::<fn(&tracing_core::Dispatch) -> Option<Context<F>>>()
                .as_ref()
                .expect("should have been set!")
        };
        (get_context)(dispatch)
    }
}

impl Default for ErrorLayer {
    fn default() -> Self {
        Self::new(DefaultFields::default())
    }
}
