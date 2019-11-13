use crate::{
    field::RecordFields,
    fmt::{format, FormatEvent, FormatFields, MakeWriter},
    layer::{self, Context},
    registry::{LookupSpan, SpanRef},
};
use std::{any::TypeId, cell::RefCell, fmt, io, marker::PhantomData, ops::Deref};
use tracing_core::{
    span::{Attributes, Id, Record},
    Event, Subscriber,
};

/// A [`Layer`] that logs formatted representations of `tracing` events.
///
/// [`Layer`]: ../layer/trait.Layer.html
#[derive(Debug)]
pub struct Layer<
    S,
    N = format::DefaultFields,
    E = format::Format<format::Full>,
    W = fn() -> io::Stdout,
> {
    make_writer: W,
    fmt_fields: N,
    fmt_event: E,
    _inner: PhantomData<S>,
}

/// A builder for [`Layer`](struct.Layer.html) that logs formatted representations of `tracing`
/// events and spans.
#[derive(Debug)]
pub struct LayerBuilder<
    S,
    N = format::DefaultFields,
    E = format::Format<format::Full>,
    W = fn() -> io::Stdout,
> {
    fmt_fields: N,
    fmt_event: E,
    make_writer: W,
    _inner: PhantomData<S>,
}

impl<S> Layer<S> {
    /// Returns a new [`LayerBuilder`](struct.LayerBuilder.html) for configuring a `Layer`.
    pub fn builder() -> LayerBuilder<S> {
        LayerBuilder::default()
    }
}

// This, like the MakeWriter block, needs to be a seperate impl block because we're
// overriding the `E` type parameter with `E2`.
impl<S, N, E, W> LayerBuilder<S, N, E, W>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'writer> FormatFields<'writer> + 'static,
    W: MakeWriter + 'static,
{
    /// Sets the [event formatter][`FormatEvent`] that the layer will use to
    /// format events.
    ///
    /// The event formatter may be any type implementing the [`FormatEvent`]
    /// trait, which is implemented for all functions taking a [`FmtContext`], a
    /// `&mut dyn Write`, and an [`Event`].
    ///
    /// # Examples
    ///
    /// Setting a type implementing [`FormatEvent`] as the formatter:
    /// ```rust
    /// use tracing_subscriber::fmt::{self, format};
    ///
    /// let layer = fmt::Layer::builder()
    ///     .event_format(format::Format::default().compact())
    ///     .finish();
    /// # // this is necessary for type inference.
    /// # use tracing_subscriber::Layer as _;
    /// # let _ = layer.with_subscriber(tracing_subscriber::registry::Registry::default());
    /// ```
    /// [event formatter]: ../format/trait.FormatEvent.html
    /// [`FmtContext`]: ../struct.FmtContext.html
    /// [`Event`]: https://docs.rs/tracing/latest/tracing/struct.Event.html
    pub fn event_format<E2>(self, e: E2) -> LayerBuilder<S, N, E2, W>
    where
        E2: FormatEvent<S, N> + 'static,
    {
        LayerBuilder {
            fmt_fields: self.fmt_fields,
            fmt_event: e,
            make_writer: self.make_writer,
            _inner: self._inner,
        }
    }
}

// this needs to be a seperate impl block because we're re-assigning the the W2 (make_writer)
// type paramater from the default.
impl<S, N, E, W> LayerBuilder<S, N, E, W> {
    /// Sets the [`MakeWriter`] that the [`Layer`] being built will use to write events.
    ///
    /// # Examples
    ///
    /// Using `stderr` rather than `stdout`:
    ///
    /// ```rust
    /// use std::io;
    /// use tracing_subscriber::fmt;
    ///
    /// let layer = fmt::Layer::builder()
    ///     .with_writer(io::stderr)
    ///     .finish();
    /// # // this is necessary for type inference.
    /// # use tracing_subscriber::Layer as _;
    /// # let _ = layer.with_subscriber(tracing_subscriber::registry::Registry::default());
    /// ```
    ///
    /// [`MakeWriter`]: ../fmt/trait.MakeWriter.html
    /// [`Layer`]: ../layer/trait.Layer.html
    pub fn with_writer<W2>(self, make_writer: W2) -> LayerBuilder<S, N, E, W2>
    where
        W2: MakeWriter + 'static,
    {
        LayerBuilder {
            fmt_fields: self.fmt_fields,
            fmt_event: self.fmt_event,
            make_writer,
            _inner: self._inner,
        }
    }
}

impl<S, N, L, T, W> LayerBuilder<S, N, format::Format<L, T>, W>
where
    N: for<'writer> FormatFields<'writer> + 'static,
{
    /// Use the given [`timer`] for span and event timestamps.
    ///
    /// See [`time`] for the provided timer implementations.
    ///
    /// Note that using the `chrono` feature flag enables the
    /// additional time formatters [`ChronoUtc`] and [`ChronoLocal`].
    ///
    /// [`time`]: ./time/index.html
    /// [`timer`]: ./time/trait.FormatTime.html
    /// [`ChronoUtc`]: ./time/struct.ChronoUtc.html
    /// [`ChronoLocal`]: ./time/struct.ChronoLocal.html
    pub fn with_timer<T2>(self, timer: T2) -> LayerBuilder<S, N, format::Format<L, T2>, W> {
        LayerBuilder {
            fmt_event: self.fmt_event.with_timer(timer),
            fmt_fields: self.fmt_fields,
            make_writer: self.make_writer,
            _inner: self._inner,
        }
    }

    /// Do not emit timestamps with spans and event.
    pub fn without_time(self) -> LayerBuilder<S, N, format::Format<L, ()>, W> {
        LayerBuilder {
            fmt_event: self.fmt_event.without_time(),
            fmt_fields: self.fmt_fields,
            make_writer: self.make_writer,
            _inner: self._inner,
        }
    }

    /// Enable ANSI encoding for formatted events.
    #[cfg(feature = "ansi")]
    pub fn with_ansi(self, ansi: bool) -> LayerBuilder<S, N, format::Format<L, T>, W> {
        LayerBuilder {
            fmt_event: self.fmt_event.with_ansi(ansi),
            fmt_fields: self.fmt_fields,
            make_writer: self.make_writer,
            _inner: self._inner,
        }
    }

    /// Sets whether or not an event's target is displayed.
    pub fn with_target(self, display_target: bool) -> LayerBuilder<S, N, format::Format<L, T>, W> {
        LayerBuilder {
            fmt_event: self.fmt_event.with_target(display_target),
            fmt_fields: self.fmt_fields,
            make_writer: self.make_writer,
            _inner: self._inner,
        }
    }

    /// Sets the layer being built to use a [less verbose formatter](../fmt/format/struct.Compact.html).
    pub fn compact(self) -> LayerBuilder<S, N, format::Format<format::Compact, T>, W>
    where
        N: for<'writer> FormatFields<'writer> + 'static,
    {
        LayerBuilder {
            fmt_event: self.fmt_event.compact(),
            fmt_fields: self.fmt_fields,
            make_writer: self.make_writer,
            _inner: self._inner,
        }
    }

    /// Sets the layer being built to use a [JSON formatter](../fmt/format/struct.Json.html).
    #[cfg(feature = "json")]
    pub fn json(self) -> LayerBuilder<S, format::JsonFields, format::Format<format::Json, T>, W> {
        LayerBuilder {
            fmt_event: self.fmt_event.json(),
            fmt_fields: format::JsonFields::new(),
            make_writer: self.make_writer,
            _inner: self._inner,
        }
    }
}

impl<S, N, E, W> LayerBuilder<S, N, E, W> {
    /// Sets the field formatter that the layer being built will use to record
    /// fields.
    pub fn fmt_fields<N2>(self, fmt_fields: N2) -> LayerBuilder<S, N2, E, W>
    where
        N2: for<'writer> FormatFields<'writer> + 'static,
    {
        LayerBuilder {
            fmt_event: self.fmt_event,
            fmt_fields,
            make_writer: self.make_writer,
            _inner: self._inner,
        }
    }
}

impl<S, N, E, W> LayerBuilder<S, N, E, W>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'writer> FormatFields<'writer> + 'static,
    E: FormatEvent<S, N> + 'static,
    W: MakeWriter + 'static,
{
    /// Builds a [`Layer`].
    ///
    /// [`Layer`]: struct.Layer.html
    pub fn finish(self) -> Layer<S, N, E, W> {
        Layer {
            make_writer: self.make_writer,
            fmt_fields: self.fmt_fields,
            fmt_event: self.fmt_event,
            _inner: self._inner,
        }
    }
}

impl<S> Default for Layer<S>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn default() -> Self {
        LayerBuilder::default().finish()
    }
}

impl<S> Default for LayerBuilder<S> {
    fn default() -> Self {
        LayerBuilder {
            fmt_fields: format::DefaultFields::default(),
            fmt_event: format::Format::default(),
            make_writer: io::stdout,
            _inner: PhantomData,
        }
    }
}

impl<S, N, E, W> Layer<S, N, E, W>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'writer> FormatFields<'writer> + 'static,
    E: FormatEvent<S, N> + 'static,
    W: MakeWriter + 'static,
{
    #[inline]
    fn make_ctx<'a>(&'a self, ctx: Context<'a, S>) -> FmtContext<'a, S, N> {
        FmtContext {
            ctx,
            fmt_fields: &self.fmt_fields,
        }
    }
}

/// A formatted representation of a span's fields stored in its [extensions].
///
/// Because `FormattedFields` is generic over the type of the formatter
/// that produced it, multiple versions of a span's formatted fields can be
/// stored in the [`Extensions`][extensions] type-map. This means that when
/// multiple formatters are in use, each can store its own formatted
/// representation without conflicting.
///
/// [extensions]: ../registry/extensions/index.html
#[derive(Default)]
pub struct FormattedFields<E> {
    _format_event: PhantomData<fn(E)>,
    /// The formatted fields of a span.
    fields: String,
}

impl<E> fmt::Debug for FormattedFields<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FormattedFields")
            .field("fields", &self.fields)
            .finish()
    }
}

impl<E> fmt::Display for FormattedFields<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.fields)
    }
}

impl<E> Deref for FormattedFields<E> {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.fields
    }
}

// === impl FmtLayer ===

impl<S, N, E, W> layer::Layer<S> for Layer<S, N, E, W>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'writer> FormatFields<'writer> + 'static,
    E: FormatEvent<S, N> + 'static,
    W: MakeWriter + 'static,
{
    fn new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();

        let mut buf = String::new();
        if self.fmt_fields.format_fields(&mut buf, attrs).is_ok() {
            let fmt_fields = FormattedFields {
                fields: buf,
                _format_event: PhantomData::<fn(N)>,
            };
            extensions.insert(fmt_fields);
        }
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();

        let mut buf = String::new();
        if self.fmt_fields.format_fields(&mut buf, values).is_ok() {
            let buf = match extensions.get_mut::<FormattedFields<Self>>() {
                Some(fields) => format!("{}{}", fields.fields, buf),
                None => buf,
            };
            let fmt_fields = FormattedFields {
                fields: buf,
                _format_event: PhantomData::<fn(N)>,
            };
            extensions.insert(fmt_fields);
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        thread_local! {
            static BUF: RefCell<String> = RefCell::new(String::new());
        }

        BUF.with(|buf| {
            let borrow = buf.try_borrow_mut();
            let mut a;
            let mut b;
            let mut buf = match borrow {
                Ok(buf) => {
                    a = buf;
                    &mut *a
                }
                _ => {
                    b = String::new();
                    &mut b
                }
            };

            let ctx = self.make_ctx(ctx);
            if self.fmt_event.format_event(&ctx, &mut buf, event).is_ok() {
                let mut writer = self.make_writer.make_writer();
                let _ = io::Write::write_all(&mut writer, buf.as_bytes());
            }

            buf.clear();
        });
    }

    unsafe fn downcast_raw(&self, id: TypeId) -> Option<*const ()> {
        // This `downcast_raw` impl allows downcasting a `fmt` layer to any of
        // its components (event formatter, field formatter, and `MakeWriter`)
        // as well as to the layer's type itself. The potential use-cases for
        // this *may* be somewhat niche, though...
        match () {
            _ if id == TypeId::of::<Self>() => Some(self as *const Self as *const ()),
            _ if id == TypeId::of::<E>() => Some(&self.fmt_event as *const E as *const ()),
            _ if id == TypeId::of::<N>() => Some(&self.fmt_fields as *const N as *const ()),
            _ if id == TypeId::of::<W>() => Some(&self.make_writer as *const W as *const ()),
            _ => None,
        }
    }
}

/// Provides the current span context to a formatter.
pub struct FmtContext<'a, S, N> {
    pub(crate) ctx: Context<'a, S>,
    pub(crate) fmt_fields: &'a N,
}

impl<'a, S, N> fmt::Debug for FmtContext<'a, S, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FmtContext").finish()
    }
}

impl<'a, S, N> FormatFields<'a> for FmtContext<'a, S, N>
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_fields<R: RecordFields>(
        &self,
        writer: &'a mut dyn fmt::Write,
        fields: R,
    ) -> fmt::Result {
        self.fmt_fields.format_fields(writer, fields)
    }
}

impl<'a, S, N> FmtContext<'a, S, N>
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    /// Visits every span in the current context with a closure.

    /// The provided closure will be called first with the current span,
    /// and then with that span's parent, and then that span's parent,
    /// and so on until a root span is reached.
    pub fn visit_spans<E, F>(&self, mut f: F) -> Result<(), E>
    where
        F: FnMut(&SpanRef<'_, S>) -> Result<(), E>,
    {
        let current_span = self.ctx.current_span();
        let id = match current_span.id() {
            Some(id) => id,
            None => return Ok(()),
        };
        let span = match self.ctx.span(id) {
            Some(span) => span,
            None => return Ok(()),
        };

        #[cfg(feature = "smallvec")]
        type SpanRefVec<'span, S> = smallvec::SmallVec<[SpanRef<'span, S>; 16]>;
        #[cfg(not(feature = "smallvec"))]
        type SpanRefVec<'span, S> = Vec<SpanRef<'span, S>>;

        // an alternative way to handle this would be to the recursive approach that
        // `fmt` uses that _does not_ entail any allocation in this fmt'ing
        // spans path.
        let parents = span.parents().collect::<SpanRefVec<'_, _>>();
        let mut iter = parents.iter().rev();
        // visit all the parent spans...
        while let Some(parent) = iter.next() {
            f(parent)?;
        }
        // and finally, print out the current span.
        f(&span)?;
        Ok(())
    }
}
