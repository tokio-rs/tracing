use crate::{
    fmt::{format, FormatEvent, FormatFields, MakeWriter},
    layer::{Context, Layer},
    registry::{LookupSpan, Registry, SpanRef},
};
use std::{io, marker::PhantomData};
use tracing_core::{span::Id, Event, Subscriber};

pub struct FmtLayer<
    S = Registry,
    N = format::DefaultFields,
    E = format::Format<format::Full>,
    W = fn() -> io::Stdout,
> {
    is_interested: Box<dyn Fn(&Event<'_>) -> bool + Send + Sync + 'static>,
    inner: PhantomData<S>,
    make_writer: W,
    fmt_fields: N,
    fmt_event: E,
}

pub struct FmtLayerBuilder<
    S = Registry,
    N = format::DefaultFields,
    E = format::Format<format::Full>,
    W = fn() -> io::Stdout,
> {
    fmt_fields: N,
    fmt_event: E,
    make_writer: W,
    is_interested: Box<dyn Fn(&Event<'_>) -> bool + Send + Sync + 'static>,
    inner: PhantomData<S>,
}

impl FmtLayer {
    pub fn builder() -> FmtLayerBuilder {
        FmtLayerBuilder::default()
    }
}

impl<S, N, E, W> FmtLayerBuilder<S, N, E, W>
where
    S: Subscriber,
    N: for<'writer> FormatFields<'writer> + 'static,
    E: FormatEvent<N> + 'static,
    W: MakeWriter + 'static,
{
    pub fn with_interest<F>(self, f: F) -> Self
    where
        F: Fn(&Event<'_>) -> bool + Send + Sync + 'static,
    {
        Self {
            is_interested: Box::new(f),
            ..self
        }
    }
}

// this needs to be a seperate impl block because we're re-assigning the the W2 (make_writer)
// type paramater from the default.
impl<S, N, E, W> FmtLayerBuilder<S, N, E, W> {
    pub fn with_writer<W2>(self, make_writer: W2) -> FmtLayerBuilder<S, N, E, W2>
    where
        W2: MakeWriter + 'static,
    {
        FmtLayerBuilder {
            fmt_fields: self.fmt_fields,
            fmt_event: self.fmt_event,
            is_interested: self.is_interested,
            inner: self.inner,
            make_writer,
        }
    }
}

impl<S, N, E, W> FmtLayerBuilder<S, N, E, W>
where
    S: Subscriber,
    N: for<'writer> FormatFields<'writer> + 'static,
    E: FormatEvent<N> + 'static,
    W: MakeWriter + 'static,
{
    pub fn build(self) -> FmtLayer<S, N, E, W> {
        FmtLayer {
            is_interested: self.is_interested,
            inner: self.inner,
            make_writer: self.make_writer,
            fmt_fields: self.fmt_fields,
            fmt_event: self.fmt_event,
        }
    }
}

impl Default for FmtLayerBuilder {
    fn default() -> Self {
        Self {
            is_interested: Box::new(|_| true),
            inner: PhantomData,
            fmt_fields: format::DefaultFields::default(),
            fmt_event: format::Format::default(),
            make_writer: io::stdout,
        }
    }
}

// === impl Formatter ===

impl<S, N, E, W> Layer<S> for FmtLayer<S, N, E, W>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'writer> FormatFields<'writer> + 'static,
    E: FormatEvent<N> + 'static,
    W: MakeWriter + 'static,
{
    fn on_close(&self, id: Id, _: Context<'_, S>) {
        dbg!(id);
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        if let Some(parent) = event.parent() {
            let span = ctx.span(parent).unwrap();
            let ctx = FmtContext {
                ctx: &span,
                fmt_fields: &self.fmt_fields,
            };
            // self.fmt_event.format_event(&ctx, &mut String::new(), event);
        }
    }
}

/// Represents the `Subscriber`'s view of the current span context to a
/// formatter.
pub struct FmtContext<'a, F, S>
where
    S: LookupSpan<'a>,
{
    ctx: &'a SpanRef<'a, S>,
    fmt_fields: &'a F,
}
