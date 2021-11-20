use crate::{
    field::RecordFields,
    fmt::{format, FormatEvent, FormatFields, MakeWriter, TestWriter},
    registry::{self, LookupSpan, SpanRef},
    subscribe::{self, Context},
};
use format::{FmtSpan, TimingDisplay};
use std::{
    any::TypeId, cell::RefCell, fmt, io, marker::PhantomData, ops::Deref, ptr::NonNull,
    time::Instant,
};
use tracing_core::{
    field,
    span::{Attributes, Current, Id, Record},
    Collect, Event, Metadata,
};

/// A [`Subscriber`] that logs formatted representations of `tracing` events.
///
/// ## Examples
///
/// Constructing a subscriber with the default configuration:
///
/// ```rust
/// use tracing_subscriber::{fmt, Registry};
/// use tracing_subscriber::subscribe::CollectExt;
///
/// let collector = Registry::default()
///     .with(fmt::Subscriber::default());
///
/// tracing::collect::set_global_default(collector).unwrap();
/// ```
///
/// Overriding the subscriber's behavior:
///
/// ```rust
/// use tracing_subscriber::{fmt, Registry};
/// use tracing_subscriber::subscribe::CollectExt;
///
/// let fmt_subscriber = fmt::subscriber()
///    .with_target(false) // don't include event targets when logging
///    .with_level(false); // don't include event levels when logging
///
/// let collector = Registry::default().with(fmt_subscriber);
/// # tracing::collect::set_global_default(collector).unwrap();
/// ```
///
/// Setting a custom event formatter:
///
/// ```rust
/// use tracing_subscriber::fmt::{self, format, time};
/// use tracing_subscriber::Subscribe;
///
/// let fmt = format().with_timer(time::Uptime::default());
/// let fmt_subscriber = fmt::subscriber()
///     .event_format(fmt)
///     .with_target(false);
/// # let subscriber = fmt_subscriber.with_collector(tracing_subscriber::registry::Registry::default());
/// # tracing::collect::set_global_default(subscriber).unwrap();
/// ```
///
/// [`Subscriber`]: subscribe::Subscribe
#[derive(Debug)]
#[cfg_attr(docsrs, doc(cfg(all(feature = "fmt", feature = "std"))))]
pub struct Subscriber<C, N = format::DefaultFields, E = format::Format, W = fn() -> io::Stdout> {
    make_writer: W,
    fmt_fields: N,
    fmt_event: E,
    fmt_span: format::FmtSpanConfig,
    is_ansi: bool,
    _inner: PhantomData<C>,
}

impl<C> Subscriber<C> {
    /// Returns a new [`Subscriber`] with the default configuration.
    pub fn new() -> Self {
        Self::default()
    }
}

// This needs to be a separate impl block because they place different bounds on the type parameters.
impl<C, N, E, W> Subscriber<C, N, E, W>
where
    C: Collect + for<'a> LookupSpan<'a>,
    N: for<'writer> FormatFields<'writer> + 'static,
    W: for<'writer> MakeWriter<'writer> + 'static,
{
    /// Sets the [event formatter][`FormatEvent`] that the subscriber will use to
    /// format events.
    ///
    /// The event formatter may be any type implementing the [`FormatEvent`]
    /// trait, which is implemented for all functions taking a [`FmtContext`], a
    /// [`Writer`], and an [`Event`].
    ///
    /// # Examples
    ///
    /// Setting a type implementing [`FormatEvent`] as the formatter:
    /// ```rust
    /// use tracing_subscriber::fmt::{self, format};
    ///
    /// let fmt_subscriber = fmt::subscriber()
    ///     .event_format(format().compact());
    /// # // this is necessary for type inference.
    /// # use tracing_subscriber::Subscribe as _;
    /// # let _ = fmt_subscriber.with_collector(tracing_subscriber::registry::Registry::default());
    /// ```
    /// [`FormatEvent`]: format::FormatEvent
    /// [`Event`]: tracing::Event
    /// [`Writer`]: crate::format::Writer
    pub fn event_format<E2>(self, e: E2) -> Subscriber<C, N, E2, W>
    where
        E2: FormatEvent<C, N> + 'static,
    {
        Subscriber {
            fmt_fields: self.fmt_fields,
            fmt_event: e,
            fmt_span: self.fmt_span,
            make_writer: self.make_writer,
            is_ansi: self.is_ansi,
            _inner: self._inner,
        }
    }
}

// This needs to be a separate impl block because they place different bounds on the type parameters.
impl<C, N, E, W> Subscriber<C, N, E, W> {
    /// Sets the [`MakeWriter`] that the [`Subscriber`] being built will use to write events.
    ///
    /// # Examples
    ///
    /// Using `stderr` rather than `stdout`:
    ///
    /// ```rust
    /// use std::io;
    /// use tracing_subscriber::fmt;
    ///
    /// let fmt_subscriber = fmt::subscriber()
    ///     .with_writer(io::stderr);
    /// # // this is necessary for type inference.
    /// # use tracing_subscriber::Subscribe as _;
    /// # let _ = fmt_subscriber.with_collector(tracing_subscriber::registry::Registry::default());
    /// ```
    ///
    /// [`MakeWriter`]: super::writer::MakeWriter
    /// [`Subscriber`]: super::Subscriber
    pub fn with_writer<W2>(self, make_writer: W2) -> Subscriber<C, N, E, W2>
    where
        W2: for<'writer> MakeWriter<'writer> + 'static,
    {
        Subscriber {
            fmt_fields: self.fmt_fields,
            fmt_event: self.fmt_event,
            fmt_span: self.fmt_span,
            is_ansi: self.is_ansi,
            make_writer,
            _inner: self._inner,
        }
    }

    /// Configures the subscriber to support [`libtest`'s output capturing][capturing] when used in
    /// unit tests.
    ///
    /// See [`TestWriter`] for additional details.
    ///
    /// # Examples
    ///
    /// Using [`TestWriter`] to let `cargo test` capture test output:
    ///
    /// ```rust
    /// use std::io;
    /// use tracing_subscriber::fmt;
    ///
    /// let fmt_subscriber = fmt::subscriber()
    ///     .with_test_writer();
    /// # // this is necessary for type inference.
    /// # use tracing_subscriber::Subscribe as _;
    /// # let _ = fmt_subscriber.with_collector(tracing_subscriber::registry::Registry::default());
    /// ```
    /// [capturing]:
    /// https://doc.rust-lang.org/book/ch11-02-running-tests.html#showing-function-output
    /// [`TestWriter`]: super::writer::TestWriter
    pub fn with_test_writer(self) -> Subscriber<C, N, E, TestWriter> {
        Subscriber {
            fmt_fields: self.fmt_fields,
            fmt_event: self.fmt_event,
            fmt_span: self.fmt_span,
            is_ansi: self.is_ansi,
            make_writer: TestWriter::default(),
            _inner: self._inner,
        }
    }

    /// Enable ANSI terminal colors for formatted output.
    #[cfg(feature = "ansi")]
    #[cfg_attr(docsrs, doc(cfg(feature = "ansi")))]
    pub fn with_ansi(self, ansi: bool) -> Self {
        Subscriber {
            is_ansi: ansi,
            ..self
        }
    }
}

impl<C, N, L, T, W> Subscriber<C, N, format::Format<L, T>, W>
where
    N: for<'writer> FormatFields<'writer> + 'static,
{
    /// Use the given [`timer`] for span and event timestamps.
    ///
    /// See the [`time` module] for the provided timer implementations.
    ///
    /// Note that using the `"time`"" feature flag enables the
    /// additional time formatters [`UtcTime`] and [`LocalTime`], which use the
    /// [`time` crate] to provide more sophisticated timestamp formatting
    /// options.
    ///
    /// [`timer`]: super::time::FormatTime
    /// [`time` module]: mod@super::time
    /// [`UtcTime`]: super::time::UtcTime
    /// [`LocalTime`]: super::time::LocalTime
    /// [`time` crate]: https://docs.rs/time/0.3
    pub fn with_timer<T2>(self, timer: T2) -> Subscriber<C, N, format::Format<L, T2>, W> {
        Subscriber {
            fmt_event: self.fmt_event.with_timer(timer),
            fmt_fields: self.fmt_fields,
            fmt_span: self.fmt_span,
            make_writer: self.make_writer,
            is_ansi: self.is_ansi,
            _inner: self._inner,
        }
    }

    /// Do not emit timestamps with spans and event.
    pub fn without_time(self) -> Subscriber<C, N, format::Format<L, ()>, W> {
        Subscriber {
            fmt_event: self.fmt_event.without_time(),
            fmt_fields: self.fmt_fields,
            fmt_span: self.fmt_span.without_time(),
            make_writer: self.make_writer,
            is_ansi: self.is_ansi,
            _inner: self._inner,
        }
    }

    /// Configures how synthesized events are emitted at points in the [span
    /// lifecycle][lifecycle].
    ///
    /// The following options are available:
    ///
    /// - `FmtSpan::NONE`: No events will be synthesized when spans are
    ///    created, entered, exited, or closed. Data from spans will still be
    ///    included as the context for formatted events. This is the default.
    /// - `FmtSpan::NEW`: An event will be synthesized when spans are created.
    /// - `FmtSpan::ENTER`: An event will be synthesized when spans are entered.
    /// - `FmtSpan::EXIT`: An event will be synthesized when spans are exited.
    /// - `FmtSpan::CLOSE`: An event will be synthesized when a span closes. If
    ///    [timestamps are enabled][time] for this formatter, the generated
    ///    event will contain fields with the span's _busy time_ (the total
    ///    time for which it was entered) and _idle time_ (the total time that
    ///    the span existed but was not entered).
    /// - `FmtSpan::ACTIVE`: Events will be synthesized when spans are entered
    ///    or exited.
    /// - `FmtSpan::FULL`: Events will be synthesized whenever a span is
    ///    created, entered, exited, or closed. If timestamps are enabled, the
    ///    close event will contain the span's busy and idle time, as
    ///    described above.
    ///
    /// The options can be enabled in any combination. For instance, the following
    /// will synthesize events whenever spans are created and closed:
    ///
    /// ```rust
    /// use tracing_subscriber::fmt;
    /// use tracing_subscriber::fmt::format::FmtSpan;
    ///
    /// let subscriber = fmt()
    ///     .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
    ///     .finish();
    /// ```
    ///
    /// Note that the generated events will only be part of the log output by
    /// this formatter; they will not be recorded by other `Collector`s or by
    /// `Subscriber`s added to this subscriber.
    ///
    /// [lifecycle]: mod@tracing::span#the-span-lifecycle
    /// [time]: Subscriber::without_time()
    pub fn with_span_events(self, kind: FmtSpan) -> Self {
        Subscriber {
            fmt_span: self.fmt_span.with_kind(kind),
            ..self
        }
    }

    /// Sets whether or not an event's target is displayed.
    pub fn with_target(self, display_target: bool) -> Subscriber<C, N, format::Format<L, T>, W> {
        Subscriber {
            fmt_event: self.fmt_event.with_target(display_target),
            ..self
        }
    }

    /// Sets whether or not an event's level is displayed.
    pub fn with_level(self, display_level: bool) -> Subscriber<C, N, format::Format<L, T>, W> {
        Subscriber {
            fmt_event: self.fmt_event.with_level(display_level),
            ..self
        }
    }

    /// Sets whether or not the [thread ID] of the current thread is displayed
    /// when formatting events
    ///
    /// [thread ID]: std::thread::ThreadId
    pub fn with_thread_ids(
        self,
        display_thread_ids: bool,
    ) -> Subscriber<C, N, format::Format<L, T>, W> {
        Subscriber {
            fmt_event: self.fmt_event.with_thread_ids(display_thread_ids),
            ..self
        }
    }

    /// Sets whether or not the [name] of the current thread is displayed
    /// when formatting events
    ///
    /// [name]: std::thread#naming-threads
    pub fn with_thread_names(
        self,
        display_thread_names: bool,
    ) -> Subscriber<C, N, format::Format<L, T>, W> {
        Subscriber {
            fmt_event: self.fmt_event.with_thread_names(display_thread_names),
            ..self
        }
    }

    /// Sets the subscriber being built to use a [less verbose formatter](format::Compact).
    pub fn compact(self) -> Subscriber<C, N, format::Format<format::Compact, T>, W>
    where
        N: for<'writer> FormatFields<'writer> + 'static,
    {
        Subscriber {
            fmt_event: self.fmt_event.compact(),
            fmt_fields: self.fmt_fields,
            fmt_span: self.fmt_span,
            make_writer: self.make_writer,
            is_ansi: self.is_ansi,
            _inner: self._inner,
        }
    }

    /// Sets the subscriber being built to use an [excessively pretty, human-readable formatter](crate::fmt::format::Pretty).
    #[cfg(feature = "ansi")]
    #[cfg_attr(docsrs, doc(cfg(feature = "ansi")))]
    pub fn pretty(self) -> Subscriber<C, format::Pretty, format::Format<format::Pretty, T>, W> {
        Subscriber {
            fmt_event: self.fmt_event.pretty(),
            fmt_fields: format::Pretty::default(),
            fmt_span: self.fmt_span,
            make_writer: self.make_writer,
            is_ansi: self.is_ansi,
            _inner: self._inner,
        }
    }

    /// Sets the subscriber being built to use a [JSON formatter](format::Json).
    ///
    /// The full format includes fields from all entered spans.
    ///
    /// # Example Output
    ///
    /// ```ignore,json
    /// {"timestamp":"Feb 20 11:28:15.096","level":"INFO","target":"mycrate","fields":{"message":"some message", "key": "value"}}
    /// ```
    ///
    /// # Options
    ///
    /// - [`Subscriber::flatten_event`] can be used to enable flattening event fields into the root
    /// object.
    ///
    #[cfg(feature = "json")]
    #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
    pub fn json(self) -> Subscriber<C, format::JsonFields, format::Format<format::Json, T>, W> {
        Subscriber {
            fmt_event: self.fmt_event.json(),
            fmt_fields: format::JsonFields::new(),
            fmt_span: self.fmt_span,
            make_writer: self.make_writer,
            // always disable ANSI escapes in JSON mode!
            is_ansi: false,
            _inner: self._inner,
        }
    }
}

#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
impl<C, T, W> Subscriber<C, format::JsonFields, format::Format<format::Json, T>, W> {
    /// Sets the JSON subscriber being built to flatten event metadata.
    ///
    /// See [`format::Json`]
    pub fn flatten_event(
        self,
        flatten_event: bool,
    ) -> Subscriber<C, format::JsonFields, format::Format<format::Json, T>, W> {
        Subscriber {
            fmt_event: self.fmt_event.flatten_event(flatten_event),
            fmt_fields: format::JsonFields::new(),
            ..self
        }
    }

    /// Sets whether or not the formatter will include the current span in
    /// formatted events.
    ///
    /// See [`format::Json`]
    pub fn with_current_span(
        self,
        display_current_span: bool,
    ) -> Subscriber<C, format::JsonFields, format::Format<format::Json, T>, W> {
        Subscriber {
            fmt_event: self.fmt_event.with_current_span(display_current_span),
            fmt_fields: format::JsonFields::new(),
            ..self
        }
    }

    /// Sets whether or not the formatter will include a list (from root to leaf)
    /// of all currently entered spans in formatted events.
    ///
    /// See [`format::Json`]
    pub fn with_span_list(
        self,
        display_span_list: bool,
    ) -> Subscriber<C, format::JsonFields, format::Format<format::Json, T>, W> {
        Subscriber {
            fmt_event: self.fmt_event.with_span_list(display_span_list),
            fmt_fields: format::JsonFields::new(),
            ..self
        }
    }
}

impl<C, N, E, W> Subscriber<C, N, E, W> {
    /// Sets the field formatter that the subscriber being built will use to record
    /// fields.
    pub fn fmt_fields<N2>(self, fmt_fields: N2) -> Subscriber<C, N2, E, W>
    where
        N2: for<'writer> FormatFields<'writer> + 'static,
    {
        Subscriber {
            fmt_event: self.fmt_event,
            fmt_fields,
            fmt_span: self.fmt_span,
            make_writer: self.make_writer,
            is_ansi: self.is_ansi,
            _inner: self._inner,
        }
    }
}

impl<C> Default for Subscriber<C> {
    fn default() -> Self {
        Subscriber {
            fmt_fields: format::DefaultFields::default(),
            fmt_event: format::Format::default(),
            fmt_span: format::FmtSpanConfig::default(),
            make_writer: io::stdout,
            is_ansi: cfg!(feature = "ansi"),
            _inner: PhantomData,
        }
    }
}

impl<C, N, E, W> Subscriber<C, N, E, W>
where
    C: Collect + for<'a> LookupSpan<'a>,
    N: for<'writer> FormatFields<'writer> + 'static,
    E: FormatEvent<C, N> + 'static,
    W: for<'writer> MakeWriter<'writer> + 'static,
{
    #[inline]
    fn make_ctx<'a>(&'a self, ctx: Context<'a, C>, event: &'a Event<'a>) -> FmtContext<'a, C, N> {
        FmtContext {
            ctx,
            fmt_fields: &self.fmt_fields,
            event,
        }
    }
}

/// A formatted representation of a span's fields stored in its [extensions].
///
/// Because `FormattedFields` is generic over the type of the formatter that
/// produced it, multiple versions of a span's formatted fields can be stored in
/// the [`Extensions`][extensions] type-map. This means that when multiple
/// formatters are in use, each can store its own formatted representation
/// without conflicting.
///
/// [extensions]: crate::registry::Extensions
#[derive(Default)]
pub struct FormattedFields<E: ?Sized> {
    _format_fields: PhantomData<fn(E)>,
    was_ansi: bool,
    /// The formatted fields of a span.
    pub fields: String,
}

impl<E: ?Sized> FormattedFields<E> {
    /// Returns a new `FormattedFields`.
    pub fn new(fields: String) -> Self {
        Self {
            fields,
            was_ansi: false,
            _format_fields: PhantomData,
        }
    }

    /// Returns a new [`format::Writer`] for writing to this `FormattedFields`.
    ///
    /// The returned [`format::Writer`] can be used with the
    /// [`FormatFields::format_fields`] method.
    pub fn as_writer(&mut self) -> format::Writer<'_> {
        format::Writer::new(&mut self.fields).with_ansi(self.was_ansi)
    }
}

impl<E: ?Sized> fmt::Debug for FormattedFields<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FormattedFields")
            .field("fields", &self.fields)
            .field("formatter", &format_args!("{}", std::any::type_name::<E>()))
            .field("was_ansi", &self.was_ansi)
            .finish()
    }
}

impl<E: ?Sized> fmt::Display for FormattedFields<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.fields, f)
    }
}

impl<E: ?Sized> Deref for FormattedFields<E> {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.fields
    }
}

// === impl FmtSubscriber ===

macro_rules! with_event_from_span {
    ($id:ident, $span:ident, $($field:literal = $value:expr),*, |$event:ident| $code:block) => {
        let meta = $span.metadata();
        let cs = meta.callsite();
        let fs = field::FieldSet::new(&[$($field),*], cs);
        #[allow(unused)]
        let mut iter = fs.iter();
        let v = [$(
            (&iter.next().unwrap(), Some(&$value as &dyn field::Value)),
        )*];
        let vs = fs.value_set(&v);
        let $event = Event::new_child_of($id, meta, &vs);
        $code
    };
}

impl<C, N, E, W> subscribe::Subscribe<C> for Subscriber<C, N, E, W>
where
    C: Collect + for<'a> LookupSpan<'a>,
    N: for<'writer> FormatFields<'writer> + 'static,
    E: FormatEvent<C, N> + 'static,
    W: for<'writer> MakeWriter<'writer> + 'static,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, C>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();

        if extensions.get_mut::<FormattedFields<N>>().is_none() {
            let mut fields = FormattedFields::<N>::new(String::new());
            if self
                .fmt_fields
                .format_fields(fields.as_writer().with_ansi(self.is_ansi), attrs)
                .is_ok()
            {
                fields.was_ansi = self.is_ansi;
                extensions.insert(fields);
            }
        }

        if self.fmt_span.fmt_timing
            && self.fmt_span.trace_close()
            && extensions.get_mut::<Timings>().is_none()
        {
            extensions.insert(Timings::new());
        }

        if self.fmt_span.trace_new() {
            with_event_from_span!(id, span, "message" = "new", |event| {
                drop(extensions);
                drop(span);
                self.on_event(&event, ctx);
            });
        }
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, C>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();
        if let Some(fields) = extensions.get_mut::<FormattedFields<N>>() {
            let _ = self.fmt_fields.add_fields(fields, values);
            return;
        }

        let mut fields = FormattedFields::<N>::new(String::new());
        if self
            .fmt_fields
            .format_fields(fields.as_writer().with_ansi(self.is_ansi), values)
            .is_ok()
        {
            fields.was_ansi = self.is_ansi;
            extensions.insert(fields);
        }
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, C>) {
        if self.fmt_span.trace_enter() || self.fmt_span.trace_close() && self.fmt_span.fmt_timing {
            let span = ctx.span(id).expect("Span not found, this is a bug");
            let mut extensions = span.extensions_mut();
            if let Some(timings) = extensions.get_mut::<Timings>() {
                let now = Instant::now();
                timings.idle += (now - timings.last).as_nanos() as u64;
                timings.last = now;
            }

            if self.fmt_span.trace_enter() {
                with_event_from_span!(id, span, "message" = "enter", |event| {
                    drop(extensions);
                    drop(span);
                    self.on_event(&event, ctx);
                });
            }
        }
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, C>) {
        if self.fmt_span.trace_exit() || self.fmt_span.trace_close() && self.fmt_span.fmt_timing {
            let span = ctx.span(id).expect("Span not found, this is a bug");
            let mut extensions = span.extensions_mut();
            if let Some(timings) = extensions.get_mut::<Timings>() {
                let now = Instant::now();
                timings.busy += (now - timings.last).as_nanos() as u64;
                timings.last = now;
            }

            if self.fmt_span.trace_exit() {
                with_event_from_span!(id, span, "message" = "exit", |event| {
                    drop(extensions);
                    drop(span);
                    self.on_event(&event, ctx);
                });
            }
        }
    }

    fn on_close(&self, id: Id, ctx: Context<'_, C>) {
        if self.fmt_span.trace_close() {
            let span = ctx.span(&id).expect("Span not found, this is a bug");
            let extensions = span.extensions();
            if let Some(timing) = extensions.get::<Timings>() {
                let Timings {
                    busy,
                    mut idle,
                    last,
                } = *timing;
                idle += (Instant::now() - last).as_nanos() as u64;

                let t_idle = field::display(TimingDisplay(idle));
                let t_busy = field::display(TimingDisplay(busy));

                with_event_from_span!(
                    id,
                    span,
                    "message" = "close",
                    "time.busy" = t_busy,
                    "time.idle" = t_idle,
                    |event| {
                        drop(extensions);
                        drop(span);
                        self.on_event(&event, ctx);
                    }
                );
            } else {
                with_event_from_span!(id, span, "message" = "close", |event| {
                    drop(extensions);
                    drop(span);
                    self.on_event(&event, ctx);
                });
            }
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, C>) {
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

            let ctx = self.make_ctx(ctx, event);
            if self
                .fmt_event
                .format_event(
                    &ctx,
                    format::Writer::new(&mut buf).with_ansi(self.is_ansi),
                    event,
                )
                .is_ok()
            {
                let mut writer = self.make_writer.make_writer_for(event.metadata());
                let _ = io::Write::write_all(&mut writer, buf.as_bytes());
            }

            buf.clear();
        });
    }

    unsafe fn downcast_raw(&self, id: TypeId) -> Option<NonNull<()>> {
        // This `downcast_raw` impl allows downcasting a `fmt` subscriber to any of
        // its components (event formatter, field formatter, and `MakeWriter`)
        // as well as to the subscriber's type itself. The potential use-cases for
        // this *may* be somewhat niche, though...
        match () {
            _ if id == TypeId::of::<Self>() => Some(NonNull::from(self).cast()),
            _ if id == TypeId::of::<E>() => Some(NonNull::from(&self.fmt_event).cast()),
            _ if id == TypeId::of::<N>() => Some(NonNull::from(&self.fmt_fields).cast()),
            _ if id == TypeId::of::<W>() => Some(NonNull::from(&self.make_writer).cast()),
            _ => None,
        }
    }
}

/// Provides the current span context to a formatter.
pub struct FmtContext<'a, C, N> {
    pub(crate) ctx: Context<'a, C>,
    pub(crate) fmt_fields: &'a N,
    pub(crate) event: &'a Event<'a>,
}

impl<'a, C, N> fmt::Debug for FmtContext<'a, C, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FmtContext").finish()
    }
}

impl<'cx, 'writer, C, N> FormatFields<'writer> for FmtContext<'cx, C, N>
where
    C: Collect + for<'lookup> LookupSpan<'lookup>,
    N: FormatFields<'writer> + 'static,
{
    fn format_fields<R: RecordFields>(
        &self,
        writer: format::Writer<'writer>,
        fields: R,
    ) -> fmt::Result {
        self.fmt_fields.format_fields(writer, fields)
    }
}

impl<'a, C, N> FmtContext<'a, C, N>
where
    C: Collect + for<'lookup> LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    /// Visits every span in the current context with a closure.
    ///
    /// The provided closure will be called first with the current span,
    /// and then with that span's parent, and then that span's parent,
    /// and so on until a root span is reached.
    pub fn visit_spans<E, F>(&self, mut f: F) -> Result<(), E>
    where
        F: FnMut(&SpanRef<'_, C>) -> Result<(), E>,
    {
        // visit all the current spans
        if let Some(scope) = self.event_scope() {
            for span in scope.from_root() {
                f(&span)?;
            }
        }
        Ok(())
    }

    /// Returns metadata for the span with the given `id`, if it exists.
    ///
    /// If this returns `None`, then no span exists for that ID (either it has
    /// closed or the ID is invalid).
    #[inline]
    pub fn metadata(&self, id: &Id) -> Option<&'static Metadata<'static>>
    where
        C: for<'lookup> LookupSpan<'lookup>,
    {
        self.ctx.metadata(id)
    }

    /// Returns [stored data] for the span with the given `id`, if it exists.
    ///
    /// If this returns `None`, then no span exists for that ID (either it has
    /// closed or the ID is invalid).
    ///
    /// [stored data]: SpanRef
    #[inline]
    pub fn span(&self, id: &Id) -> Option<SpanRef<'_, C>>
    where
        C: for<'lookup> LookupSpan<'lookup>,
    {
        self.ctx.span(id)
    }

    /// Returns `true` if an active span exists for the given `Id`.
    #[inline]
    pub fn exists(&self, id: &Id) -> bool
    where
        C: for<'lookup> LookupSpan<'lookup>,
    {
        self.ctx.exists(id)
    }

    /// Returns [stored data] for the span that the wrapped subscriber considers
    /// to be the current.
    ///
    /// If this returns `None`, then we are not currently within a span.
    ///
    /// [stored data]: SpanRef
    #[inline]
    pub fn lookup_current(&self) -> Option<SpanRef<'_, C>>
    where
        C: for<'lookup> LookupSpan<'lookup>,
    {
        self.ctx.lookup_current()
    }

    /// Returns the current span for this formatter.
    pub fn current_span(&self) -> Current {
        self.ctx.current_span()
    }

    /// Returns [stored data] for the parent span of the event currently being
    /// formatted.
    ///
    /// If the event has a contextual parent, this will return the current span. If
    /// the event has an explicit parent span, this will return that span. If
    /// the event does not have a parent span, this will return `None`.
    ///
    /// [stored data]: SpanRef
    pub fn parent_span(&self) -> Option<SpanRef<'_, C>> {
        self.ctx.event_span(self.event)
    }

    /// Returns an iterator over the [stored data] for all the spans in the
    /// current context, starting with the specified span and ending with the
    /// root of the trace tree and ending with the current span.
    ///
    /// This is equivalent to the [`Context::span_scope`] method.
    ///
    /// <div class="information">
    ///     <div class="tooltip ignore" style="">ⓘ<span class="tooltiptext">Note</span></div>
    /// </div>
    /// <div class="example-wrap" style="display:inline-block">
    /// <pre class="ignore" style="white-space:normal;font:inherit;">
    /// <strong>Note</strong>: Compared to <a href="#method.scope"><code>scope</code></a> this
    /// returns the spans in reverse order (from leaf to root). Use
    /// <a href="../registry/struct.Scope.html#method.from_root"><code>Scope::from_root</code></a>
    /// in case root-to-leaf ordering is desired.
    /// </pre></div>
    ///
    /// <div class="example-wrap" style="display:inline-block">
    /// <pre class="ignore" style="white-space:normal;font:inherit;">
    /// <strong>Note</strong>: This requires the wrapped subscriber to implement the
    /// <a href="../registry/trait.LookupSpan.html"><code>LookupSpan</code></a> trait.
    /// See the documentation on <a href="./struct.Context.html"><code>Context</code>'s
    /// declaration</a> for details.
    /// </pre></div>
    ///
    /// [stored data]: crate::registry::SpanRef
    pub fn span_scope(&self, id: &Id) -> Option<registry::Scope<'_, C>>
    where
        C: for<'lookup> LookupSpan<'lookup>,
    {
        self.ctx.span_scope(id)
    }

    /// Returns an iterator over the [stored data] for all the spans in the
    /// event's span context, starting with its parent span and ending with the
    /// root of the trace tree.
    ///
    /// This is equivalent to calling the [`Context::event_scope`] method and
    /// passing the event currently being formatted.
    ///
    /// <div class="example-wrap" style="display:inline-block">
    /// <pre class="ignore" style="white-space:normal;font:inherit;">
    /// <strong>Note</strong>: Compared to <a href="#method.scope"><code>scope</code></a> this
    /// returns the spans in reverse order (from leaf to root). Use
    /// <a href="../registry/struct.Scope.html#method.from_root"><code>Scope::from_root</code></a>
    /// in case root-to-leaf ordering is desired.
    /// </pre></div>
    ///
    /// <div class="example-wrap" style="display:inline-block">
    /// <pre class="ignore" style="white-space:normal;font:inherit;">
    /// <strong>Note</strong>: This requires the wrapped subscriber to implement the
    /// <a href="../registry/trait.LookupSpan.html"><code>LookupSpan</code></a> trait.
    /// See the documentation on <a href="./struct.Context.html"><code>Context</code>'s
    /// declaration</a> for details.
    /// </pre></div>
    ///
    /// [stored data]: crate::registry::SpanRef
    pub fn event_scope(&self) -> Option<registry::Scope<'_, C>>
    where
        C: for<'lookup> registry::LookupSpan<'lookup>,
    {
        self.ctx.event_scope(self.event)
    }

    /// Returns the [field formatter] configured by the subscriber invoking
    /// `format_event`.
    ///
    /// The event formatter may use the returned field formatter to format the
    /// fields of any events it records.
    ///
    /// [field formatter]: FormatFields
    pub fn field_format(&self) -> &N {
        self.fmt_fields
    }
}

struct Timings {
    idle: u64,
    busy: u64,
    last: Instant,
}

impl Timings {
    fn new() -> Self {
        Self {
            idle: 0,
            busy: 0,
            last: Instant::now(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::fmt::{
        self,
        format::{self, test::MockTime, Format},
        subscribe::Subscribe as _,
        test::{MockMakeWriter, MockWriter},
        time,
    };
    use crate::Registry;
    use format::FmtSpan;
    use regex::Regex;
    use tracing::collect::with_default;
    use tracing_core::dispatch::Dispatch;

    #[test]
    fn impls() {
        let f = Format::default().with_timer(time::Uptime::default());
        let fmt = fmt::Subscriber::default().event_format(f);
        let subscriber = fmt.with_collector(Registry::default());
        let _dispatch = Dispatch::new(subscriber);

        let f = format::Format::default();
        let fmt = fmt::Subscriber::default().event_format(f);
        let subscriber = fmt.with_collector(Registry::default());
        let _dispatch = Dispatch::new(subscriber);

        let f = format::Format::default().compact();
        let fmt = fmt::Subscriber::default().event_format(f);
        let subscriber = fmt.with_collector(Registry::default());
        let _dispatch = Dispatch::new(subscriber);
    }

    #[test]
    fn fmt_subscriber_downcasts() {
        let f = format::Format::default();
        let fmt = fmt::Subscriber::default().event_format(f);
        let subscriber = fmt.with_collector(Registry::default());

        let dispatch = Dispatch::new(subscriber);
        assert!(dispatch
            .downcast_ref::<fmt::Subscriber<Registry>>()
            .is_some());
    }

    #[test]
    fn fmt_subscriber_downcasts_to_parts() {
        let f = format::Format::default();
        let fmt = fmt::Subscriber::default().event_format(f);
        let subscriber = fmt.with_collector(Registry::default());
        let dispatch = Dispatch::new(subscriber);
        assert!(dispatch.downcast_ref::<format::DefaultFields>().is_some());
        assert!(dispatch.downcast_ref::<format::Format>().is_some())
    }

    #[test]
    fn is_lookup_span() {
        fn assert_lookup_span<T: for<'a> crate::registry::LookupSpan<'a>>(_: T) {}
        let fmt = fmt::Subscriber::default();
        let subscriber = fmt.with_collector(Registry::default());
        assert_lookup_span(subscriber)
    }

    fn sanitize_timings(s: String) -> String {
        let re = Regex::new("time\\.(idle|busy)=([0-9.]+)[mµn]s").unwrap();
        re.replace_all(s.as_str(), "timing").to_string()
    }

    #[test]
    fn synthesize_span_none() {
        let make_writer = MockMakeWriter::default();
        let subscriber = crate::fmt::Collector::builder()
            .with_writer(make_writer.clone())
            .with_level(false)
            .with_ansi(false)
            .with_timer(MockTime)
            // check that FmtSpan::NONE is the default
            .finish();

        with_default(subscriber, || {
            let span1 = tracing::info_span!("span1", x = 42);
            let _e = span1.enter();
        });
        let actual = sanitize_timings(make_writer.get_string());
        assert_eq!("", actual.as_str());
    }

    #[test]
    fn synthesize_span_active() {
        let make_writer = MockMakeWriter::default();
        let subscriber = crate::fmt::Collector::builder()
            .with_writer(make_writer.clone())
            .with_level(false)
            .with_ansi(false)
            .with_timer(MockTime)
            .with_span_events(FmtSpan::ACTIVE)
            .finish();

        with_default(subscriber, || {
            let span1 = tracing::info_span!("span1", x = 42);
            let _e = span1.enter();
        });
        let actual = sanitize_timings(make_writer.get_string());
        assert_eq!(
            "fake time span1{x=42}: tracing_subscriber::fmt::fmt_subscriber::test: enter\n\
             fake time span1{x=42}: tracing_subscriber::fmt::fmt_subscriber::test: exit\n",
            actual.as_str()
        );
    }

    #[test]
    fn synthesize_span_close() {
        let make_writer = MockMakeWriter::default();
        let subscriber = crate::fmt::Collector::builder()
            .with_writer(make_writer.clone())
            .with_level(false)
            .with_ansi(false)
            .with_timer(MockTime)
            .with_span_events(FmtSpan::CLOSE)
            .finish();

        with_default(subscriber, || {
            let span1 = tracing::info_span!("span1", x = 42);
            let _e = span1.enter();
        });
        let actual = sanitize_timings(make_writer.get_string());
        assert_eq!(
            "fake time span1{x=42}: tracing_subscriber::fmt::fmt_subscriber::test: close timing timing\n",
            actual.as_str()
        );
    }

    #[test]
    fn synthesize_span_close_no_timing() {
        let make_writer = MockMakeWriter::default();
        let subscriber = crate::fmt::Collector::builder()
            .with_writer(make_writer.clone())
            .with_level(false)
            .with_ansi(false)
            .with_timer(MockTime)
            .without_time()
            .with_span_events(FmtSpan::CLOSE)
            .finish();

        with_default(subscriber, || {
            let span1 = tracing::info_span!("span1", x = 42);
            let _e = span1.enter();
        });
        let actual = sanitize_timings(make_writer.get_string());
        assert_eq!(
            "span1{x=42}: tracing_subscriber::fmt::fmt_subscriber::test: close\n",
            actual.as_str()
        );
    }

    #[test]
    fn synthesize_span_full() {
        let make_writer = MockMakeWriter::default();
        let subscriber = crate::fmt::Collector::builder()
            .with_writer(make_writer.clone())
            .with_level(false)
            .with_ansi(false)
            .with_timer(MockTime)
            .with_span_events(FmtSpan::FULL)
            .finish();

        with_default(subscriber, || {
            let span1 = tracing::info_span!("span1", x = 42);
            let _e = span1.enter();
        });
        let actual = sanitize_timings(make_writer.get_string());
        assert_eq!(
            "fake time span1{x=42}: tracing_subscriber::fmt::fmt_subscriber::test: new\n\
             fake time span1{x=42}: tracing_subscriber::fmt::fmt_subscriber::test: enter\n\
             fake time span1{x=42}: tracing_subscriber::fmt::fmt_subscriber::test: exit\n\
             fake time span1{x=42}: tracing_subscriber::fmt::fmt_subscriber::test: close timing timing\n",
            actual.as_str()
        );
    }

    #[test]
    fn make_writer_based_on_meta() {
        struct MakeByTarget {
            make_writer1: MockMakeWriter,
            make_writer2: MockMakeWriter,
        }

        impl<'a> MakeWriter<'a> for MakeByTarget {
            type Writer = MockWriter;

            fn make_writer(&'a self) -> Self::Writer {
                self.make_writer1.make_writer()
            }

            fn make_writer_for(&'a self, meta: &Metadata<'_>) -> Self::Writer {
                if meta.target() == "writer2" {
                    return self.make_writer2.make_writer();
                }
                self.make_writer()
            }
        }

        let make_writer1 = MockMakeWriter::default();
        let make_writer2 = MockMakeWriter::default();

        let make_writer = MakeByTarget {
            make_writer1: make_writer1.clone(),
            make_writer2: make_writer2.clone(),
        };

        let subscriber = crate::fmt::Collector::builder()
            .with_writer(make_writer)
            .with_level(false)
            .with_target(false)
            .with_ansi(false)
            .with_timer(MockTime)
            .with_span_events(FmtSpan::CLOSE)
            .finish();

        with_default(subscriber, || {
            let span1 = tracing::info_span!("writer1_span", x = 42);
            let _e = span1.enter();
            tracing::info!(target: "writer2", "hello writer2!");
            let span2 = tracing::info_span!(target: "writer2", "writer2_span");
            let _e = span2.enter();
            tracing::warn!(target: "writer1", "hello writer1!");
        });

        let actual = sanitize_timings(make_writer1.get_string());
        assert_eq!(
            "fake time writer1_span{x=42}:writer2_span: hello writer1!\n\
             fake time writer1_span{x=42}: close timing timing\n",
            actual.as_str()
        );
        let actual = sanitize_timings(make_writer2.get_string());
        assert_eq!(
            "fake time writer1_span{x=42}: hello writer2!\n\
             fake time writer1_span{x=42}:writer2_span: close timing timing\n",
            actual.as_str()
        );
    }
}
