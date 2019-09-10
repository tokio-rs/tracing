//! A `Subscriber` for formatting and logging `tracing` data.
//!
//! ## Overview
//!
//! [`tracing`] is a framework for instrumenting Rust programs with context-aware,
//! structured, event-based diagnostic information. This crate provides an
//! implementation of the [`Subscriber`] trait that records `tracing`'s `Event`s
//! and `Span`s by formatting them as text and logging them to stdout.
//!
//!
//! [`tracing`]: https://crates.io/crates/tracing
//! [`Subscriber`]: https://docs.rs/tracing/latest/tracing/trait.Subscriber.html
use tracing_core::{field, subscriber::Interest, Event, Metadata};

use std::{any::TypeId, cell::RefCell, fmt, io};
pub mod format;
mod span;
pub mod time;
pub mod writer;

use crate::filter::LevelFilter;
use crate::layer::{self, Layer};

#[doc(inline)]
pub use self::{format::FormatEvent, span::Context, writer::MakeWriter};

/// A `Subscriber` that logs formatted representations of `tracing` events.
///
/// This consists of an inner`Formatter` wrapped in a layer that performs filtering.
#[derive(Debug)]
pub struct Subscriber<
    N = format::NewRecorder,
    E = format::Format<format::Full>,
    F = LevelFilter,
    W = fn() -> io::Stdout,
> {
    inner: layer::Layered<F, Formatter<N, E, W>>,
}

/// A `Subscriber` that logs formatted representations of `tracing` events.
/// This type only logs formatted events; it does not perform any filtering.
#[derive(Debug)]
pub struct Formatter<
    N = format::NewRecorder,
    E = format::Format<format::Full>,
    W = fn() -> io::Stdout,
> {
    new_visitor: N,
    fmt_event: E,
    spans: span::Store,
    settings: Settings,
    make_writer: W,
}

/// Configures and constructs `Subscriber`s.
#[derive(Debug)]
pub struct Builder<
    N = format::NewRecorder,
    E = format::Format<format::Full>,
    F = LevelFilter,
    W = fn() -> io::Stdout,
> {
    filter: F,
    new_visitor: N,
    fmt_event: E,
    settings: Settings,
    make_writer: W,
}

#[derive(Debug)]
struct Settings {
    inherit_fields: bool,
    initial_span_capacity: usize,
}

impl Subscriber {
    /// The maximum [verbosity level] that is enabled by a `Subscriber` by
    /// default.
    ///
    /// This can be overridden with the [`Builder::with_max_level`] method.
    ///
    /// [verbosity level]: https://docs.rs/tracing-core/0.1.5/tracing_core/struct.Level.html
    /// [`Builder::with_max_level`]: struct.Builder.html#method.with_max_level
    pub const DEFAULT_MAX_LEVEL: LevelFilter = LevelFilter::INFO;

    /// Returns a new `Builder` for configuring a format subscriber.
    pub fn builder() -> Builder {
        Builder::default()
    }

    /// Returns a new format subscriber with the default configuration.
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for Subscriber {
    fn default() -> Self {
        Builder::default().finish()
    }
}
// === impl Subscriber ===

impl<N, E, F, W> tracing_core::Subscriber for Subscriber<N, E, F, W>
where
    N: for<'a> NewVisitor<'a> + 'static,
    E: FormatEvent<N> + 'static,
    F: Layer<Formatter<N, E, W>> + 'static,
    W: MakeWriter + 'static,
    layer::Layered<F, Formatter<N, E, W>>: tracing_core::Subscriber,
{
    #[inline]
    fn register_callsite(&self, meta: &'static Metadata<'static>) -> Interest {
        self.inner.register_callsite(meta)
    }

    #[inline]
    fn enabled(&self, meta: &Metadata<'_>) -> bool {
        self.inner.enabled(meta)
    }

    #[inline]
    fn new_span(&self, attrs: &span::Attributes<'_>) -> span::Id {
        self.inner.new_span(attrs)
    }

    #[inline]
    fn record(&self, span: &span::Id, values: &span::Record<'_>) {
        self.inner.record(span, values)
    }

    #[inline]
    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {
        self.inner.record_follows_from(span, follows)
    }

    #[inline]
    fn event(&self, event: &Event<'_>) {
        self.inner.event(event);
    }

    #[inline]
    fn enter(&self, id: &span::Id) {
        // TODO: add on_enter hook
        self.inner.enter(id);
    }

    #[inline]
    fn exit(&self, id: &span::Id) {
        self.inner.exit(id);
    }

    #[inline]
    fn current_span(&self) -> span::Current {
        self.inner.current_span()
    }

    #[inline]
    fn clone_span(&self, id: &span::Id) -> span::Id {
        self.inner.clone_span(id)
    }

    #[inline]
    fn try_close(&self, id: span::Id) -> bool {
        self.inner.try_close(id)
    }

    unsafe fn downcast_raw(&self, id: TypeId) -> Option<*const ()> {
        if id == TypeId::of::<Self>() {
            Some(self as *const Self as *const ())
        } else {
            self.inner.downcast_raw(id)
        }
    }
}

// === impl Formatter ===
impl<N, E, W> Formatter<N, E, W>
where
    N: for<'a> NewVisitor<'a>,
{
    #[inline]
    fn ctx(&self) -> span::Context<'_, N> {
        span::Context::new(&self.spans, &self.new_visitor)
    }
}

impl<N, E, W> tracing_core::Subscriber for Formatter<N, E, W>
where
    N: for<'a> NewVisitor<'a> + 'static,
    E: FormatEvent<N> + 'static,
    W: MakeWriter + 'static,
{
    fn register_callsite(&self, _: &'static Metadata<'static>) -> Interest {
        Interest::always()
    }

    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    #[inline]
    fn new_span(&self, attrs: &span::Attributes<'_>) -> span::Id {
        self.spans.new_span(attrs, &self.new_visitor)
    }

    #[inline]
    fn record(&self, span: &span::Id, values: &span::Record<'_>) {
        self.spans.record(span, values, &self.new_visitor)
    }

    fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {
        // TODO: implement this please
    }

    fn event(&self, event: &Event<'_>) {
        thread_local! {
            static BUF: RefCell<String> = RefCell::new(String::new());
        }

        BUF.with(|buf| {
            let borrow = buf.try_borrow_mut();
            let mut a;
            let mut b;
            let buf = match borrow {
                Ok(buf) => {
                    a = buf;
                    &mut *a
                }
                _ => {
                    b = String::new();
                    &mut b
                }
            };

            if self.fmt_event.format_event(&self.ctx(), buf, event).is_ok() {
                let mut writer = self.make_writer.make_writer();
                let _ = io::Write::write_all(&mut writer, buf.as_bytes());
            }

            buf.clear();
        });
    }

    fn enter(&self, id: &span::Id) {
        // TODO: add on_enter hook
        self.spans.push(id);
    }

    fn exit(&self, id: &span::Id) {
        self.spans.pop(id);
    }

    fn current_span(&self) -> span::Current {
        if let Some(id) = self.spans.current() {
            if let Some(meta) = self.spans.get(&id).map(|span| span.metadata()) {
                return span::Current::new(id, meta);
            }
        }
        span::Current::none()
    }

    #[inline]
    fn clone_span(&self, id: &span::Id) -> span::Id {
        self.spans.clone_span(id)
    }

    #[inline]
    fn try_close(&self, id: span::Id) -> bool {
        self.spans.drop_span(id)
    }

    unsafe fn downcast_raw(&self, id: TypeId) -> Option<*const ()> {
        match () {
            _ if id == TypeId::of::<Self>() => Some(self as *const Self as *const ()),
            // _ if id == TypeId::of::<F>() => Some(&self.filter as *const F as *const ()),
            _ if id == TypeId::of::<E>() => Some(&self.fmt_event as *const E as *const ()),
            _ if id == TypeId::of::<N>() => Some(&self.new_visitor as *const N as *const ()),
            _ => None,
        }
    }
}

/// A type that can construct a new field visitor for formatting the fields on a
/// span or event.
pub trait NewVisitor<'a> {
    /// The type of the returned `Visitor`.
    type Visitor: field::Visit + 'a;
    /// Returns a new `Visitor` that writes to the provided `writer`.
    fn make(&self, writer: &'a mut dyn fmt::Write, is_empty: bool) -> Self::Visitor;
}

impl<'a, F, R> NewVisitor<'a> for F
where
    F: Fn(&'a mut dyn fmt::Write, bool) -> R,
    R: field::Visit + 'a,
{
    type Visitor = R;

    #[inline]
    fn make(&self, writer: &'a mut dyn fmt::Write, is_empty: bool) -> Self::Visitor {
        (self)(writer, is_empty)
    }
}

// ===== impl Builder =====

impl Default for Builder {
    fn default() -> Self {
        Builder {
            filter: Subscriber::DEFAULT_MAX_LEVEL,
            new_visitor: format::NewRecorder::new(),
            fmt_event: format::Format::default(),
            settings: Settings::default(),
            make_writer: io::stdout,
        }
    }
}

impl<N, E, F, W> Builder<N, E, F, W>
where
    N: for<'a> NewVisitor<'a> + 'static,
    E: FormatEvent<N> + 'static,
    W: MakeWriter + 'static,
    F: Layer<Formatter<N, E, W>> + 'static,
{
    /// Finish the builder, returning a new `FmtSubscriber`.
    pub fn finish(self) -> Subscriber<N, E, F, W> {
        let subscriber = Formatter {
            new_visitor: self.new_visitor,
            fmt_event: self.fmt_event,
            spans: span::Store::with_capacity(self.settings.initial_span_capacity),
            settings: self.settings,
            make_writer: self.make_writer,
        };
        Subscriber {
            inner: self.filter.with_subscriber(subscriber),
        }
    }
}

impl<N, L, T, F, W> Builder<N, format::Format<L, T>, F, W>
where
    N: for<'a> NewVisitor<'a> + 'static,
{
    /// Use the given `timer` for log message timestamps.
    pub fn with_timer<T2>(self, timer: T2) -> Builder<N, format::Format<L, T2>, F, W> {
        Builder {
            new_visitor: self.new_visitor,
            fmt_event: self.fmt_event.with_timer(timer),
            filter: self.filter,
            settings: self.settings,
            make_writer: self.make_writer,
        }
    }

    /// Do not emit timestamps with log messages.
    pub fn without_time(self) -> Builder<N, format::Format<L, ()>, F, W> {
        Builder {
            new_visitor: self.new_visitor,
            fmt_event: self.fmt_event.without_time(),
            filter: self.filter,
            settings: self.settings,
            make_writer: self.make_writer,
        }
    }

    /// Enable ANSI encoding for formatted events.
    #[cfg(feature = "ansi")]
    pub fn with_ansi(self, ansi: bool) -> Builder<N, format::Format<L, T>, F, W> {
        Builder {
            fmt_event: self.fmt_event.with_ansi(ansi),
            ..self
        }
    }

    /// Sets whether or not an event's target is displayed.
    pub fn with_target(self, display_target: bool) -> Builder<N, format::Format<L, T>, F, W> {
        Builder {
            fmt_event: self.fmt_event.with_target(display_target),
            ..self
        }
    }
}

#[cfg(feature = "filter")]
impl<N, E, W> Builder<N, E, crate::Filter, W>
where
    Formatter<N, E, W>: tracing_core::Subscriber + 'static,
{
    /// Configures the subscriber being built to allow filter reloading at
    /// runtime.
    pub fn with_filter_reloading(
        self,
    ) -> Builder<N, E, crate::reload::Layer<crate::Filter, Formatter<N, E, W>>, W> {
        let (filter, _) = crate::reload::Layer::new(self.filter);
        Builder {
            new_visitor: self.new_visitor,
            fmt_event: self.fmt_event,
            filter,
            settings: self.settings,
            make_writer: self.make_writer,
        }
    }
}

#[cfg(feature = "filter")]
impl<N, E, W> Builder<N, E, crate::reload::Layer<crate::Filter, Formatter<N, E, W>>, W>
where
    Formatter<N, E, W>: tracing_core::Subscriber + 'static,
{
    /// Returns a `Handle` that may be used to reload the constructed subscriber's
    /// filter.
    pub fn reload_handle(&self) -> crate::reload::Handle<crate::Filter, Formatter<N, E, W>> {
        self.filter.handle()
    }
}

impl<N, E, F, W> Builder<N, E, F, W> {
    /// Sets the Visitor that the subscriber being built will use to record
    /// fields.
    pub fn with_visitor<N2>(self, new_visitor: N2) -> Builder<N2, E, F, W>
    where
        N2: for<'a> NewVisitor<'a> + 'static,
    {
        Builder {
            new_visitor,
            fmt_event: self.fmt_event,
            filter: self.filter,
            settings: self.settings,
            make_writer: self.make_writer,
        }
    }

    /// Sets the [`Filter`] that the subscriber will use to determine if
    /// a span or event is enabled.
    ///
    /// Note that this method requires the "filter" feature flag to be enabled.
    ///
    /// If a filter was previously set, or a maximum level was set by the
    /// [`with_max_level`] method, that value is replaced by the new filter.
    ///
    /// # Examples
    ///
    /// Setting a filter based on the value of the `RUST_LOG` environment
    /// variable:
    /// ```rust
    /// use tracing_subscriber::{FmtSubscriber, Filter};
    ///
    /// let subscriber = FmtSubscriber::builder()
    ///     .with_filter(Filter::from_default_env())
    ///     .finish();
    /// ```
    ///
    /// Setting a filter based on a pre-set filter directive string:
    /// ```rust
    /// use tracing_subscriber::FmtSubscriber;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let subscriber = FmtSubscriber::builder()
    ///     .with_filter("my_crate=info,my_crate::my_mod=debug,[my_span]=trace")
    ///     .finish();
    /// # Ok(()) }
    /// ```
    ///
    /// Adding additional directives to a filter constructed from an env var:
    /// ```rust
    /// use tracing_subscriber::{
    ///     FmtSubscriber,
    ///     filter::{Filter, LevelFilter},
    /// };
    ///
    /// # fn filter() -> Result<(), Box<dyn std::error::Error>> {
    /// let filter = Filter::try_from_env("MY_CUSTOM_FILTER_ENV_VAR")?
    ///     // Set the base level when not matched by other directives to WARN.
    ///     .add_directive(LevelFilter::WARN.into())
    ///     // Set the max level for `my_crate::my_mod` to DEBUG, overriding
    ///     // any directives parsed from the env variable.
    ///     .add_directive("my_crate::my_mod=debug".parse()?);
    ///
    /// let subscriber = FmtSubscriber::builder()
    ///     .with_filter(filter)
    ///     .finish();
    /// # Ok(())}
    /// ```
    /// [`Filter`]: ../filter/struct.Filter.html
    /// [`with_max_level`]: #method.with_max_level
    #[cfg(feature = "filter")]
    pub fn with_filter(self, filter: impl Into<crate::Filter>) -> Builder<N, E, crate::Filter, W>
    where
        Formatter<N, E, W>: tracing_core::Subscriber + 'static,
    {
        let filter = filter.into();
        Builder {
            new_visitor: self.new_visitor,
            fmt_event: self.fmt_event,
            filter,
            settings: self.settings,
            make_writer: self.make_writer,
        }
    }

    /// Sets the maximum [verbosity level] that will be enabled by the
    /// subscriber.
    ///
    /// If the max level has already been set, or a [`Filter`] was added by
    /// [`with_filter`], this replaces that configuration with the new
    /// maximum level.
    ///
    /// # Examples
    ///
    /// Enable up to the `DEBUG` verbosity level:
    /// ```rust
    /// use tracing_subscriber::FmtSubscriber;
    /// use tracing::Level;
    ///
    /// let subscriber = FmtSubscriber::builder()
    ///     .with_max_level(Level::DEBUG)
    ///     .finish();
    /// ```
    /// This subscriber won't record any spans or events!
    /// ```rust
    /// use tracing_subscriber::{
    ///     FmtSubscriber,
    ///     filter::LevelFilter,
    /// };
    ///
    /// let subscriber = FmtSubscriber::builder()
    ///     .with_max_level(LevelFilter::OFF)
    ///     .finish();
    /// ```
    /// [verbosity level]: https://docs.rs/tracing-core/0.1.5/tracing_core/struct.Level.html
    /// [`Filter`]: ../filter/struct.Filter.html
    /// [`with_filter`]: #method.with_filter
    pub fn with_max_level(self, filter: impl Into<LevelFilter>) -> Builder<N, E, LevelFilter, W> {
        let filter = filter.into();
        Builder {
            new_visitor: self.new_visitor,
            fmt_event: self.fmt_event,
            filter,
            settings: self.settings,
            make_writer: self.make_writer,
        }
    }

    /// Sets the subscriber being built to use a less verbose formatter.
    ///
    /// See [`format::Compact`].
    pub fn compact(self) -> Builder<N, format::Format<format::Compact>, F, W>
    where
        N: for<'a> NewVisitor<'a> + 'static,
    {
        Builder {
            fmt_event: format::Format::default().compact(),
            filter: self.filter,
            new_visitor: self.new_visitor,
            settings: self.settings,
            make_writer: self.make_writer,
        }
    }

    /// Sets the function that the subscriber being built should use to format
    /// events that occur.
    pub fn on_event<E2>(self, fmt_event: E2) -> Builder<N, E2, F, W>
    where
        E2: FormatEvent<N> + 'static,
    {
        Builder {
            new_visitor: self.new_visitor,
            fmt_event,
            filter: self.filter,
            settings: self.settings,
            make_writer: self.make_writer,
        }
    }

    /// Sets whether or not spans inherit their parents' field values (disabled
    /// by default).
    pub fn inherit_fields(self, inherit_fields: bool) -> Self {
        Builder {
            settings: Settings {
                inherit_fields,
                ..self.settings
            },
            ..self
        }
    }

    // TODO(eliza): should this be publicly exposed?
    // /// Configures the initial capacity for the span slab used to store
    // /// in-progress span data. This may be used for tuning the subscriber's
    // /// allocation performance, but in general does not need to be manually configured..
    // pub fn initial_span_capacity(self, initial_span_capacity: usize) -> Self {
    //     Builder {
    //         settings: Settings {
    //             initial_span_capacity,
    //             ..self.settings
    //         },
    //         ..self
    //     }
    // }
}

impl<N, E, F, W> Builder<N, E, F, W> {
    /// Sets the [`MakeWriter`] that the subscriber being built will use to write events.
    ///
    /// # Examples
    ///
    /// Using `stderr` rather than `stdout`:
    ///
    /// ```rust
    /// use std::io;
    ///
    /// let subscriber = tracing_subscriber::fmt::Subscriber::builder()
    ///     .with_writer(io::stderr)
    ///     .finish();
    /// ```
    ///
    /// [`MakeWriter`]: trait.MakeWriter.html
    pub fn with_writer<W2>(self, make_writer: W2) -> Builder<N, E, F, W2>
    where
        W2: MakeWriter + 'static,
    {
        Builder {
            new_visitor: self.new_visitor,
            fmt_event: self.fmt_event,
            filter: self.filter,
            settings: self.settings,
            make_writer,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            inherit_fields: false,
            initial_span_capacity: 32,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tracing_core::dispatcher::Dispatch;

    #[test]
    fn impls() {
        let f = format::Format::default().with_timer(time::Uptime::default());
        let subscriber = Subscriber::builder().on_event(f).finish();
        let _dispatch = Dispatch::new(subscriber);

        let f = format::Format::default();
        let subscriber = Subscriber::builder().on_event(f).finish();
        let _dispatch = Dispatch::new(subscriber);

        let f = format::Format::default().compact();
        let subscriber = Subscriber::builder().on_event(f).finish();
        let _dispatch = Dispatch::new(subscriber);
    }

    #[test]
    fn subscriber_downcasts() {
        let subscriber = Subscriber::new();
        let dispatch = Dispatch::new(subscriber);
        assert!(dispatch.downcast_ref::<Subscriber>().is_some());
    }

    #[test]
    fn subscriber_downcasts_to_parts() {
        let subscriber = Subscriber::builder().finish();
        let dispatch = Dispatch::new(subscriber);
        assert!(dispatch.downcast_ref::<format::NewRecorder>().is_some());
        assert!(dispatch.downcast_ref::<LevelFilter>().is_some());
        assert!(dispatch.downcast_ref::<format::Format>().is_some())
    }
}
