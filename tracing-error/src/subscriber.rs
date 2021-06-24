use std::fmt;
use std::marker::PhantomData;
use std::{
    any::{type_name, TypeId},
    ptr::NonNull,
};
use tracing::{span, Collect, Dispatch, Metadata};
use tracing_subscriber::fmt::format::{DefaultFields, FormatFields};
use tracing_subscriber::{
    fmt::FormattedFields,
    registry::LookupSpan,
    subscribe::{self, Subscribe},
};

/// A [subscriber] that enables capturing [`SpanTrace`]s.
///
/// Optionally, this type may be constructed with a [field formatter] to use
/// when formatting the fields of each span in a trace. When no formatter is
/// provided, the [default format] is used instead.
///
/// [subscriber]: tracing_subscriber::subscribe::Subscribe
/// [`SpanTrace`]: super::SpanTrace
/// [field formatter]: tracing_subscriber::fmt::FormatFields
/// [default format]: tracing_subscriber::fmt::format::DefaultFields
pub struct ErrorSubscriber<C, F = DefaultFields> {
    format: F,

    get_context: WithContext,
    _collector: PhantomData<fn(C)>,
}

// this function "remembers" the types of the subscriber and the formatter,
// so that we can downcast to something aware of them without knowing those
// types at the callsite.
pub(crate) struct WithContext(
    fn(&Dispatch, &span::Id, f: &mut dyn FnMut(&'static Metadata<'static>, &str) -> bool),
);

impl<C, F> Subscribe<C> for ErrorSubscriber<C, F>
where
    C: Collect + for<'span> LookupSpan<'span>,
    F: for<'writer> FormatFields<'writer> + 'static,
{
    /// Notifies this subscriber that a new span was constructed with the given
    /// `Attributes` and `Id`.
    fn new_span(
        &self,
        attrs: &span::Attributes<'_>,
        id: &span::Id,
        ctx: subscribe::Context<'_, C>,
    ) {
        let span = ctx.span(id).expect("span must already exist!");
        if span.extensions().get::<FormattedFields<F>>().is_some() {
            return;
        }
        let mut fields = String::new();
        if self.format.format_fields(&mut fields, attrs).is_ok() {
            span.extensions_mut()
                .insert(FormattedFields::<F>::new(fields));
        }
    }

    unsafe fn downcast_raw(&self, id: TypeId) -> Option<NonNull<()>> {
        match id {
            id if id == TypeId::of::<Self>() => Some(NonNull::from(self).cast()),
            id if id == TypeId::of::<WithContext>() => {
                Some(NonNull::from(&self.get_context).cast())
            }
            _ => None,
        }
    }
}

impl<C, F> ErrorSubscriber<C, F>
where
    F: for<'writer> FormatFields<'writer> + 'static,
    C: Collect + for<'span> LookupSpan<'span>,
{
    /// Returns a new `ErrorSubscriber` with the provided [field formatter].
    ///
    /// [field formatter]: tracing_subscriber::fmt::FormatFields
    pub fn new(format: F) -> Self {
        Self {
            format,
            get_context: WithContext(Self::get_context),
            _collector: PhantomData,
        }
    }

    fn get_context(
        dispatch: &Dispatch,
        id: &span::Id,
        f: &mut dyn FnMut(&'static Metadata<'static>, &str) -> bool,
    ) {
        let collector = dispatch
            .downcast_ref::<C>()
            .expect("collector should downcast to expected type; this is a bug!");
        let span = collector
            .span(id)
            .expect("registry should have a span for the current ID");
        for span in span.scope() {
            let cont = if let Some(fields) = span.extensions().get::<FormattedFields<F>>() {
                f(span.metadata(), fields.fields.as_str())
            } else {
                f(span.metadata(), "")
            };
            if !cont {
                break;
            }
        }
    }
}

impl WithContext {
    pub(crate) fn with_context<'a>(
        &self,
        dispatch: &'a Dispatch,
        id: &span::Id,
        mut f: impl FnMut(&'static Metadata<'static>, &str) -> bool,
    ) {
        (self.0)(dispatch, id, &mut f)
    }
}

impl<C> Default for ErrorSubscriber<C>
where
    C: Collect + for<'span> LookupSpan<'span>,
{
    fn default() -> Self {
        Self::new(DefaultFields::default())
    }
}

impl<C, F: fmt::Debug> fmt::Debug for ErrorSubscriber<C, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ErrorSubscriber")
            .field("format", &self.format)
            .field("collector", &format_args!("{}", type_name::<C>()))
            .finish()
    }
}
