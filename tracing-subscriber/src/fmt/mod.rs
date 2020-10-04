//! A `Collector` for formatting and logging `tracing` data.
//!
//! ## Overview
//!
//! [`tracing`] is a framework for instrumenting Rust programs with context-aware,
//! structured, event-based diagnostic information. This crate provides an
//! implementation of the [`Collector`] trait that records `tracing`'s `Event`s
//! and `Span`s by formatting them as text and logging them to stdout.
//!
//! ## Usage
//!
//! First, add this to your `Cargo.toml` file:
//!
//! ```toml
//! [dependencies]
//! tracing-subscriber = "0.2"
//! ```
//!
//! *Compiler support: requires rustc 1.39+*
//!
//! Add the following to your executable to initialize the default collector:
//! ```rust
//! use tracing_subscriber;
//!
//! tracing_subscriber::fmt::init();
//! ```
//!
//! ## Filtering Events with Environment Variables
//!
//! The default subscriber installed by `init` enables you to filter events
//! at runtime using environment variables (using the [`EnvFilter`]).
//!
//! The filter syntax is a superset of the [`env_logger`] syntax.
//!
//! For example:
//! - Setting `RUST_LOG=debug` enables all `Span`s and `Event`s
//!     set to the log level `DEBUG` or higher
//! - Setting `RUST_LOG=my_crate=trace` enables `Span`s and `Event`s
//!     in `my_crate` at all log levels
//!
//! **Note**: This should **not** be called by libraries. Libraries should use
//! [`tracing`] to publish `tracing` `Event`s.
//!
//! ## Configuration
//!
//! You can configure a subscriber instead of using the defaults with
//! the following functions:
//!
//! ### Collector
//!
//! The [`FmtSubscriber`] formats and records `tracing` events as line-oriented logs.
//! You can create one by calling:
//!
//! ```rust
//! let subscriber = tracing_subscriber::fmt()
//!     // ... add configuration
//!     .finish();
//! ```
//!
//! You can find the configuration methods for [`FmtSubscriber`] in [`fmtBuilder`].
//!
//! ### Filters
//!
//! If you want to filter the `tracing` `Events` based on environment
//! variables, you can use the [`EnvFilter`] as follows:
//!
//! ```rust
//! use tracing_subscriber::EnvFilter;
//!
//! let filter = EnvFilter::from_default_env();
//! ```
//!
//! As mentioned above, the [`EnvFilter`] allows `Span`s and `Event`s to
//! be filtered at runtime by setting the `RUST_LOG` environment variable.
//!
//! You can find the other available [`filter`]s in the documentation.
//!
//! ### Using Your Collector
//!
//! Finally, once you have configured your `Collector`, you need to
//! configure your executable to use it.
//!
//! A subscriber can be installed globally using:
//! ```rust
//! use tracing;
//! use tracing_subscriber::FmtSubscriber;
//!
//! let subscriber = FmtSubscriber::new();
//!
//! tracing::collector::set_global_default(subscriber)
//!     .map_err(|_err| eprintln!("Unable to set global default subscriber"));
//! // Note this will only fail if you try to set the global default
//! // subscriber multiple times
//! ```
//!
//! ### Composing Layers
//!
//! Composing an [`EnvFilter`] `Subscriber` and a [format `Subscriber`](../fmt/struct.Subscriber.html):
//!
//! ```rust
//! use tracing_subscriber::{fmt, EnvFilter};
//! use tracing_subscriber::prelude::*;
//!
//! let fmt_layer = fmt::layer()
//!     .with_target(false);
//! let filter_layer = EnvFilter::try_from_default_env()
//!     .or_else(|_| EnvFilter::try_new("info"))
//!     .unwrap();
//!
//! tracing_subscriber::registry()
//!     .with(filter_layer)
//!     .with(fmt_layer)
//!     .init();
//! ```
//!
//! [`EnvFilter`]: ../filter/struct.EnvFilter.html
//! [`env_logger`]: https://docs.rs/env_logger/
//! [`filter`]: ../filter/index.html
//! [`fmtBuilder`]: ./struct.SubscriberBuilder.html
//! [`FmtSubscriber`]: ./struct.Collector.html
//! [`Collector`]:
//!     https://docs.rs/tracing/latest/tracing/trait.Subscriber.html
//! [`tracing`]: https://crates.io/crates/tracing
use std::{any::TypeId, error::Error, io};
use tracing_core::{collector::Interest, span, Event, Metadata};

mod fmt_layer;
pub mod format;
pub mod time;
pub mod writer;
#[allow(deprecated)]
pub use fmt_layer::LayerBuilder;
pub use fmt_layer::{FmtContext, FormattedFields, Layer};

use crate::layer::Subscriber as _;
use crate::{
    filter::LevelFilter,
    layer,
    registry::{LookupSpan, Registry},
};

#[doc(inline)]
pub use self::{
    format::{format, FormatEvent, FormatFields},
    time::time,
    writer::{MakeWriter, TestWriter},
};

/// A `Collector` that logs formatted representations of `tracing` events.
///
/// This consists of an inner `Formatter` wrapped in a layer that performs filtering.
#[derive(Debug)]
pub struct Collector<
    N = format::DefaultFields,
    E = format::Format<format::Full>,
    F = LevelFilter,
    W = fn() -> io::Stdout,
> {
    inner: layer::Layered<F, Formatter<N, E, W>>,
}

/// A `Collector` that logs formatted representations of `tracing` events.
/// This type only logs formatted events; it does not perform any filtering.
pub type Formatter<
    N = format::DefaultFields,
    E = format::Format<format::Full>,
    W = fn() -> io::Stdout,
> = layer::Layered<fmt_layer::Layer<Registry, N, E, W>, Registry>;

/// Configures and constructs `Collector`s.
#[derive(Debug)]
pub struct CollectorBuilder<
    N = format::DefaultFields,
    E = format::Format<format::Full>,
    F = LevelFilter,
    W = fn() -> io::Stdout,
> {
    filter: F,
    inner: Layer<Registry, N, E, W>,
}

/// Returns a new [`CollectorBuilder`] for configuring a [formatting subscriber].
///
/// This is essentially shorthand for [`CollectorBuilder::default()]`.
///
/// # Examples
///
/// Using [`init`] to set the default subscriber:
///
/// ```rust
/// tracing_subscriber::fmt().init();
/// ```
///
/// Configuring the output format:
///
/// ```rust
///
/// tracing_subscriber::fmt()
///     // Configure formatting settings.
///     .with_target(false)
///     .with_timer(tracing_subscriber::fmt::time::uptime())
///     .with_level(true)
///     // Set the subscriber as the default.
///     .init();
/// ```
///
/// [`try_init`] returns an error if the default subscriber could not be set:
///
/// ```rust
/// use std::error::Error;
///
/// fn init_subscriber() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
///     tracing_subscriber::fmt()
///         // Configure the subscriber to emit logs in JSON format.
///         .json()
///         // Configure the subscriber to flatten event fields in the output JSON objects.
///         .flatten_event(true)
///         // Set the subscriber as the default, returning an error if this fails.
///         .try_init()?;
///
///     Ok(())
/// }
/// ```
///
/// Rather than setting the subscriber as the default, [`finish`] _returns_ the
/// constructed collector, which may then be passed to other functions:
///
/// ```rust
/// let collector = tracing_subscriber::fmt()
///     .with_max_level(tracing::Level::DEBUG)
///     .compact()
///     .finish();
///
/// tracing::collector::with_default(collector, || {
///     // the collector will only be set as the default
///     // inside this closure...
/// })
/// ```
///
/// [`SubscriberBuilder`]: struct.SubscriberBuilder.html
/// [formatting subscriber]: struct.Collector.html
/// [`SubscriberBuilder::default()`]: struct.SubscriberBuilder.html#method.default
/// [`init`]: struct.SubscriberBuilder.html#method.init
/// [`try_init`]: struct.SubscriberBuilder.html#method.try_init
/// [`finish`]: struct.SubscriberBuilder.html#method.finish
pub fn fmt() -> CollectorBuilder {
    CollectorBuilder::default()
}

/// Returns a new [formatting subscriber] that can be [composed] with other layers to
/// construct a [`Collector`].
///
/// This is a shorthand for the equivalent [`Subscriber::default`] function.
///
/// [formatting layer]: struct.Subscriber.html
/// [composed]: ../layer/index.html
/// [`Subscriber::default`]: struct.Subscriber.html#method.default
pub fn layer<S>() -> Layer<S> {
    Layer::default()
}

impl Collector {
    /// The maximum [verbosity level] that is enabled by a `Collector` by
    /// default.
    ///
    /// This can be overridden with the [`SubscriberBuilder::with_max_level`] method.
    ///
    /// [verbosity level]: https://docs.rs/tracing-core/0.1.5/tracing_core/struct.Level.html
    /// [`SubscriberBuilder::with_max_level`]: struct.SubscriberBuilder.html#method.with_max_level
    pub const DEFAULT_MAX_LEVEL: LevelFilter = LevelFilter::INFO;

    /// Returns a new `SubscriberBuilder` for configuring a format subscriber.
    pub fn builder() -> CollectorBuilder {
        CollectorBuilder::default()
    }

    /// Returns a new format subscriber with the default configuration.
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for Collector {
    fn default() -> Self {
        CollectorBuilder::default().finish()
    }
}

// === impl Collector ===

impl<N, E, F, W> tracing_core::Collector for Collector<N, E, F, W>
where
    N: for<'writer> FormatFields<'writer> + 'static,
    E: FormatEvent<Registry, N> + 'static,
    F: layer::Subscriber<Formatter<N, E, W>> + 'static,
    W: MakeWriter + 'static,
    layer::Layered<F, Formatter<N, E, W>>: tracing_core::Collector,
    fmt_layer::Layer<Registry, N, E, W>: layer::Subscriber<Registry>,
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

impl<'a, N, E, F, W> LookupSpan<'a> for Collector<N, E, F, W>
where
    layer::Layered<F, Formatter<N, E, W>>: LookupSpan<'a>,
{
    type Data = <layer::Layered<F, Formatter<N, E, W>> as LookupSpan<'a>>::Data;

    fn span_data(&'a self, id: &span::Id) -> Option<Self::Data> {
        self.inner.span_data(id)
    }
}

// ===== impl SubscriberBuilder =====

impl Default for CollectorBuilder {
    fn default() -> Self {
        CollectorBuilder {
            filter: Collector::DEFAULT_MAX_LEVEL,
            inner: Default::default(),
        }
    }
}

impl<N, E, F, W> CollectorBuilder<N, E, F, W>
where
    N: for<'writer> FormatFields<'writer> + 'static,
    E: FormatEvent<Registry, N> + 'static,
    W: MakeWriter + 'static,
    F: layer::Subscriber<Formatter<N, E, W>> + Send + Sync + 'static,
    fmt_layer::Layer<Registry, N, E, W>: layer::Subscriber<Registry> + Send + Sync + 'static,
{
    /// Finish the builder, returning a new `FmtSubscriber`.
    pub fn finish(self) -> Collector<N, E, F, W> {
        let subscriber = self.inner.with_subscriber(Registry::default());
        Collector {
            inner: self.filter.with_subscriber(subscriber),
        }
    }

    /// Install this Collector as the global default if one is
    /// not already set.
    ///
    /// If the `tracing-log` feature is enabled, this will also install
    /// the LogTracer to convert `Log` records into `tracing` `Event`s.
    ///
    /// # Errors
    /// Returns an Error if the initialization was unsuccessful, likely
    /// because a global subscriber was already installed by another
    /// call to `try_init`.
    pub fn try_init(self) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
        #[cfg(feature = "tracing-log")]
        tracing_log::LogTracer::init().map_err(Box::new)?;

        tracing_core::dispatcher::set_global_default(tracing_core::dispatcher::Dispatch::new(
            self.finish(),
        ))?;
        Ok(())
    }

    /// Install this Collector as the global default.
    ///
    /// If the `tracing-log` feature is enabled, this will also install
    /// the LogTracer to convert `Log` records into `tracing` `Event`s.
    ///
    /// # Panics
    /// Panics if the initialization was unsuccessful, likely because a
    /// global subscriber was already installed by another call to `try_init`.
    pub fn init(self) {
        self.try_init()
            .expect("Unable to install global subscriber")
    }
}

impl<N, E, F, W> Into<tracing_core::Dispatch> for CollectorBuilder<N, E, F, W>
where
    N: for<'writer> FormatFields<'writer> + 'static,
    E: FormatEvent<Registry, N> + 'static,
    W: MakeWriter + 'static,
    F: layer::Subscriber<Formatter<N, E, W>> + Send + Sync + 'static,
    fmt_layer::Layer<Registry, N, E, W>: layer::Subscriber<Registry> + Send + Sync + 'static,
{
    fn into(self) -> tracing_core::Dispatch {
        tracing_core::Dispatch::new(self.finish())
    }
}

impl<N, L, T, F, W> CollectorBuilder<N, format::Format<L, T>, F, W>
where
    N: for<'writer> FormatFields<'writer> + 'static,
{
    /// Use the given [`timer`] for log message timestamps.
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
    pub fn with_timer<T2>(self, timer: T2) -> CollectorBuilder<N, format::Format<L, T2>, F, W> {
        CollectorBuilder {
            filter: self.filter,
            inner: self.inner.with_timer(timer),
        }
    }

    /// Do not emit timestamps with log messages.
    pub fn without_time(self) -> CollectorBuilder<N, format::Format<L, ()>, F, W> {
        CollectorBuilder {
            filter: self.filter,
            inner: self.inner.without_time(),
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
    /// - `FmtSpan::ACTIVE`: Events will be synthesized when spans are entered
    ///    or exited.
    /// - `FmtSpan::CLOSE`: An event will be synthesized when a span closes. If
    ///    [timestamps are enabled][time] for this formatter, the generated
    ///    event will contain fields with the span's _busy time_ (the total
    ///    time for which it was entered) and _idle time_ (the total time that
    ///    the span existed but was not entered).
    /// - `FmtSpan::FULL`: Events will be synthesized whenever a span is
    ///    created, entered, exited, or closed. If timestamps are enabled, the
    ///    close event will contain the span's busy and idle time, as
    ///    described above.
    ///
    /// Note that the generated events will only be part of the log output by
    /// this formatter; they will not be recorded by other `Collector`s or by
    /// `Subscriber`s added to this subscriber.
    ///
    /// [lifecycle]: https://docs.rs/tracing/latest/tracing/span/index.html#the-span-lifecycle
    /// [time]: #method.without_time
    pub fn with_span_events(self, kind: format::FmtSpan) -> Self {
        CollectorBuilder {
            filter: self.filter,
            inner: self.inner.with_span_events(kind),
        }
    }

    /// Enable ANSI encoding for formatted events.
    #[cfg(feature = "ansi")]
    #[cfg_attr(docsrs, doc(cfg(feature = "ansi")))]
    pub fn with_ansi(self, ansi: bool) -> CollectorBuilder<N, format::Format<L, T>, F, W> {
        CollectorBuilder {
            filter: self.filter,
            inner: self.inner.with_ansi(ansi),
        }
    }

    /// Sets whether or not an event's target is displayed.
    pub fn with_target(
        self,
        display_target: bool,
    ) -> CollectorBuilder<N, format::Format<L, T>, F, W> {
        CollectorBuilder {
            filter: self.filter,
            inner: self.inner.with_target(display_target),
        }
    }

    /// Sets whether or not an event's level is displayed.
    pub fn with_level(
        self,
        display_level: bool,
    ) -> CollectorBuilder<N, format::Format<L, T>, F, W> {
        CollectorBuilder {
            filter: self.filter,
            inner: self.inner.with_level(display_level),
        }
    }

    /// Sets whether or not the [name] of the current thread is displayed
    /// when formatting events
    ///
    /// [name]: https://doc.rust-lang.org/stable/std/thread/index.html#naming-threads
    pub fn with_thread_names(
        self,
        display_thread_names: bool,
    ) -> CollectorBuilder<N, format::Format<L, T>, F, W> {
        CollectorBuilder {
            filter: self.filter,
            inner: self.inner.with_thread_names(display_thread_names),
        }
    }

    /// Sets whether or not the [thread ID] of the current thread is displayed
    /// when formatting events
    ///
    /// [thread ID]: https://doc.rust-lang.org/stable/std/thread/struct.ThreadId.html
    pub fn with_thread_ids(
        self,
        display_thread_ids: bool,
    ) -> CollectorBuilder<N, format::Format<L, T>, F, W> {
        CollectorBuilder {
            filter: self.filter,
            inner: self.inner.with_thread_ids(display_thread_ids),
        }
    }

    /// Sets the subscriber being built to use a less verbose formatter.
    ///
    /// See [`format::Compact`](../fmt/format/struct.Compact.html).
    pub fn compact(self) -> CollectorBuilder<N, format::Format<format::Compact, T>, F, W>
    where
        N: for<'writer> FormatFields<'writer> + 'static,
    {
        CollectorBuilder {
            filter: self.filter,
            inner: self.inner.compact(),
        }
    }

    /// Sets the subscriber being built to use a JSON formatter.
    ///
    /// See [`format::Json`](../fmt/format/struct.Json.html)
    #[cfg(feature = "json")]
    #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
    pub fn json(self) -> CollectorBuilder<format::JsonFields, format::Format<format::Json, T>, F, W>
    where
        N: for<'writer> FormatFields<'writer> + 'static,
    {
        CollectorBuilder {
            filter: self.filter,
            inner: self.inner.json(),
        }
    }
}

#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
impl<T, F, W> CollectorBuilder<format::JsonFields, format::Format<format::Json, T>, F, W> {
    /// Sets the json subscriber being built to flatten event metadata.
    ///
    /// See [`format::Json`](../fmt/format/struct.Json.html)
    pub fn flatten_event(
        self,
        flatten_event: bool,
    ) -> CollectorBuilder<format::JsonFields, format::Format<format::Json, T>, F, W> {
        CollectorBuilder {
            filter: self.filter,
            inner: self.inner.flatten_event(flatten_event),
        }
    }

    /// Sets whether or not the JSON layer being built will include the current span
    /// in formatted events.
    ///
    /// See [`format::Json`](../fmt/format/struct.Json.html)
    pub fn with_current_span(
        self,
        display_current_span: bool,
    ) -> CollectorBuilder<format::JsonFields, format::Format<format::Json, T>, F, W> {
        CollectorBuilder {
            filter: self.filter,
            inner: self.inner.with_current_span(display_current_span),
        }
    }

    /// Sets whether or not the JSON layer being built will include a list (from
    /// root to leaf) of all currently entered spans in formatted events.
    ///
    /// See [`format::Json`](../fmt/format/struct.Json.html)
    pub fn with_span_list(
        self,
        display_span_list: bool,
    ) -> CollectorBuilder<format::JsonFields, format::Format<format::Json, T>, F, W> {
        CollectorBuilder {
            filter: self.filter,
            inner: self.inner.with_span_list(display_span_list),
        }
    }
}

#[cfg(feature = "env-filter")]
#[cfg_attr(docsrs, doc(cfg(feature = "env-filter")))]
impl<N, E, W> CollectorBuilder<N, E, crate::EnvFilter, W>
where
    Formatter<N, E, W>: tracing_core::Collector + 'static,
{
    /// Configures the subscriber being built to allow filter reloading at
    /// runtime.
    pub fn with_filter_reloading(
        self,
    ) -> CollectorBuilder<N, E, crate::reload::Layer<crate::EnvFilter, Formatter<N, E, W>>, W> {
        let (filter, _) = crate::reload::Layer::new(self.filter);
        CollectorBuilder {
            filter,
            inner: self.inner,
        }
    }
}

#[cfg(feature = "env-filter")]
#[cfg_attr(docsrs, doc(cfg(feature = "env-filter")))]
impl<N, E, W> CollectorBuilder<N, E, crate::reload::Layer<crate::EnvFilter, Formatter<N, E, W>>, W>
where
    Formatter<N, E, W>: tracing_core::Collector + 'static,
{
    /// Returns a `Handle` that may be used to reload the constructed subscriber's
    /// filter.
    pub fn reload_handle(&self) -> crate::reload::Handle<crate::EnvFilter, Formatter<N, E, W>> {
        self.filter.handle()
    }
}

impl<N, E, F, W> CollectorBuilder<N, E, F, W> {
    /// Sets the Visitor that the subscriber being built will use to record
    /// fields.
    ///
    /// For example:
    /// ```rust
    /// use tracing_subscriber::fmt::format;
    /// use tracing_subscriber::prelude::*;
    ///
    /// let formatter =
    ///     // Construct a custom formatter for `Debug` fields
    ///     format::debug_fn(|writer, field, value| write!(writer, "{}: {:?}", field, value))
    ///         // Use the `tracing_subscriber::MakeFmtExt` trait to wrap the
    ///         // formatter so that a delimiter is added between fields.
    ///         .delimited(", ");
    ///
    /// let subscriber = tracing_subscriber::fmt()
    ///     .fmt_fields(formatter)
    ///     .finish();
    /// # drop(subscriber)
    /// ```
    pub fn fmt_fields<N2>(self, fmt_fields: N2) -> CollectorBuilder<N2, E, F, W>
    where
        N2: for<'writer> FormatFields<'writer> + 'static,
    {
        CollectorBuilder {
            filter: self.filter,
            inner: self.inner.fmt_fields(fmt_fields),
        }
    }

    /// Sets the [`EnvFilter`] that the subscriber will use to determine if
    /// a span or event is enabled.
    ///
    /// Note that this method requires the "env-filter" feature flag to be enabled.
    ///
    /// If a filter was previously set, or a maximum level was set by the
    /// [`with_max_level`] method, that value is replaced by the new filter.
    ///
    /// # Examples
    ///
    /// Setting a filter based on the value of the `RUST_LOG` environment
    /// variable:
    /// ```rust
    /// use tracing_subscriber::{fmt, EnvFilter};
    ///
    /// fmt()
    ///     .with_env_filter(EnvFilter::from_default_env())
    ///     .init();
    /// ```
    ///
    /// Setting a filter based on a pre-set filter directive string:
    /// ```rust
    /// use tracing_subscriber::fmt;
    ///
    /// fmt()
    ///     .with_env_filter("my_crate=info,my_crate::my_mod=debug,[my_span]=trace")
    ///     .init();
    /// ```
    ///
    /// Adding additional directives to a filter constructed from an env var:
    /// ```rust
    /// use tracing_subscriber::{fmt, filter::{EnvFilter, LevelFilter}};
    ///
    /// # fn filter() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    /// let filter = EnvFilter::try_from_env("MY_CUSTOM_FILTER_ENV_VAR")?
    ///     // Set the base level when not matched by other directives to WARN.
    ///     .add_directive(LevelFilter::WARN.into())
    ///     // Set the max level for `my_crate::my_mod` to DEBUG, overriding
    ///     // any directives parsed from the env variable.
    ///     .add_directive("my_crate::my_mod=debug".parse()?);
    ///
    /// fmt()
    ///     .with_env_filter(filter)
    ///     .try_init()?;
    /// # Ok(())}
    /// ```
    /// [`EnvFilter`]: ../filter/struct.EnvFilter.html
    /// [`with_max_level`]: #method.with_max_level
    #[cfg(feature = "env-filter")]
    #[cfg_attr(docsrs, doc(cfg(feature = "env-filter")))]
    pub fn with_env_filter(
        self,
        filter: impl Into<crate::EnvFilter>,
    ) -> CollectorBuilder<N, E, crate::EnvFilter, W>
    where
        Formatter<N, E, W>: tracing_core::Collector + 'static,
    {
        let filter = filter.into();
        CollectorBuilder {
            filter,
            inner: self.inner,
        }
    }

    /// Sets the maximum [verbosity level] that will be enabled by the
    /// subscriber.
    ///
    /// If the max level has already been set, or a [`EnvFilter`] was added by
    /// [`with_filter`], this replaces that configuration with the new
    /// maximum level.
    ///
    /// # Examples
    ///
    /// Enable up to the `DEBUG` verbosity level:
    /// ```rust
    /// use tracing_subscriber::fmt;
    /// use tracing::Level;
    ///
    /// fmt()
    ///     .with_max_level(Level::DEBUG)
    ///     .init();
    /// ```
    /// This subscriber won't record any spans or events!
    /// ```rust
    /// use tracing_subscriber::{fmt, filter::LevelFilter};
    ///
    /// let subscriber = fmt()
    ///     .with_max_level(LevelFilter::OFF)
    ///     .finish();
    /// ```
    /// [verbosity level]: https://docs.rs/tracing-core/0.1.5/tracing_core/struct.Level.html
    /// [`EnvFilter`]: ../filter/struct.EnvFilter.html
    /// [`with_filter`]: #method.with_filter
    pub fn with_max_level(
        self,
        filter: impl Into<LevelFilter>,
    ) -> CollectorBuilder<N, E, LevelFilter, W> {
        let filter = filter.into();
        CollectorBuilder {
            filter,
            inner: self.inner,
        }
    }

    /// Sets the function that the subscriber being built should use to format
    /// events that occur.
    pub fn event_format<E2>(self, fmt_event: E2) -> CollectorBuilder<N, E2, F, W>
    where
        E2: FormatEvent<Registry, N> + 'static,
        N: for<'writer> FormatFields<'writer> + 'static,
        W: MakeWriter + 'static,
    {
        CollectorBuilder {
            filter: self.filter,
            inner: self.inner.event_format(fmt_event),
        }
    }

    /// Sets whether or not spans inherit their parents' field values (disabled
    /// by default).
    #[deprecated(since = "0.2.0", note = "this no longer does anything")]
    pub fn inherit_fields(self, inherit_fields: bool) -> Self {
        let _ = inherit_fields;
        self
    }

    /// Sets the function that the subscriber being built should use to format
    /// events that occur.
    #[deprecated(since = "0.2.0", note = "renamed to `event_format`.")]
    pub fn on_event<E2>(self, fmt_event: E2) -> CollectorBuilder<N, E2, F, W>
    where
        E2: FormatEvent<Registry, N> + 'static,
        N: for<'writer> FormatFields<'writer> + 'static,
        W: MakeWriter + 'static,
    {
        self.event_format(fmt_event)
    }

    /// Sets the [`MakeWriter`] that the subscriber being built will use to write events.
    ///
    /// # Examples
    ///
    /// Using `stderr` rather than `stdout`:
    ///
    /// ```rust
    /// use tracing_subscriber::fmt;
    /// use std::io;
    ///
    /// fmt()
    ///     .with_writer(io::stderr)
    ///     .init();
    /// ```
    ///
    /// [`MakeWriter`]: trait.MakeWriter.html
    pub fn with_writer<W2>(self, make_writer: W2) -> CollectorBuilder<N, E, F, W2>
    where
        W2: MakeWriter + 'static,
    {
        CollectorBuilder {
            filter: self.filter,
            inner: self.inner.with_writer(make_writer),
        }
    }

    /// Configures the subscriber to support [`libtest`'s output capturing][capturing] when used in
    /// unit tests.
    ///
    /// See [`TestWriter`] for additional details.
    ///
    /// # Examples
    ///
    /// Using [`TestWriter`] to let `cargo test` capture test output. Note that we do not install it
    /// globally as it may cause conflicts.
    ///
    /// ```rust
    /// use tracing_subscriber::fmt;
    /// use tracing::collector;
    ///
    /// collector::set_default(
    ///     fmt()
    ///         .with_test_writer()
    ///         .finish()
    /// );
    /// ```
    ///
    /// [capturing]:
    /// https://doc.rust-lang.org/book/ch11-02-running-tests.html#showing-function-output
    /// [`TestWriter`]: writer/struct.TestWriter.html
    pub fn with_test_writer(self) -> CollectorBuilder<N, E, F, TestWriter> {
        CollectorBuilder {
            filter: self.filter,
            inner: self.inner.with_writer(TestWriter::default()),
        }
    }
}

/// Install a global tracing subscriber that listens for events and
/// filters based on the value of the [`RUST_LOG` environment variable],
/// if one is not already set.
///
/// If the `tracing-log` feature is enabled, this will also install
/// the [`LogTracer`] to convert `log` records into `tracing` `Event`s.
///
/// This is shorthand for
///
/// ```rust
/// # fn doc() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
/// tracing_subscriber::fmt().try_init()
/// # }
/// ```
///
///
/// # Errors
///
/// Returns an Error if the initialization was unsuccessful,
/// likely because a global subscriber was already installed by another
/// call to `try_init`.
///
/// [`LogTracer`]:
///     https://docs.rs/tracing-log/0.1.0/tracing_log/struct.LogTracer.html
/// [`RUST_LOG` environment variable]:
///     ../filter/struct.EnvFilter.html#associatedconstant.DEFAULT_ENV
pub fn try_init() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let builder = Collector::builder();

    #[cfg(feature = "env-filter")]
    let builder = builder.with_env_filter(crate::EnvFilter::from_default_env());

    builder.try_init()
}

/// Install a global tracing subscriber that listens for events and
/// filters based on the value of the [`RUST_LOG` environment variable].
///
/// If the `tracing-log` feature is enabled, this will also install
/// the LogTracer to convert `Log` records into `tracing` `Event`s.
///
/// This is shorthand for
///
/// ```rust
/// tracing_subscriber::fmt().init()
/// ```
///
/// # Panics
/// Panics if the initialization was unsuccessful, likely because a
/// global subscriber was already installed by another call to `try_init`.
///
/// [`RUST_LOG` environment variable]:
///     ../filter/struct.EnvFilter.html#associatedconstant.DEFAULT_ENV
pub fn init() {
    try_init().expect("Unable to install global subscriber")
}

#[cfg(test)]
mod test {
    use crate::{
        filter::LevelFilter,
        fmt::{
            format::{self, Format},
            time,
            writer::MakeWriter,
            Collector,
        },
    };
    use std::{
        io,
        sync::{Mutex, MutexGuard, TryLockError},
    };
    use tracing_core::dispatcher::Dispatch;

    pub(crate) struct MockWriter<'a> {
        buf: &'a Mutex<Vec<u8>>,
    }

    impl<'a> MockWriter<'a> {
        pub(crate) fn new(buf: &'a Mutex<Vec<u8>>) -> Self {
            Self { buf }
        }

        pub(crate) fn map_error<Guard>(err: TryLockError<Guard>) -> io::Error {
            match err {
                TryLockError::WouldBlock => io::Error::from(io::ErrorKind::WouldBlock),
                TryLockError::Poisoned(_) => io::Error::from(io::ErrorKind::Other),
            }
        }

        pub(crate) fn buf(&self) -> io::Result<MutexGuard<'a, Vec<u8>>> {
            self.buf.try_lock().map_err(Self::map_error)
        }
    }

    impl<'a> io::Write for MockWriter<'a> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buf()?.write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.buf()?.flush()
        }
    }

    pub(crate) struct MockMakeWriter<'a> {
        buf: &'a Mutex<Vec<u8>>,
    }

    impl<'a> MockMakeWriter<'a> {
        pub(crate) fn new(buf: &'a Mutex<Vec<u8>>) -> Self {
            Self { buf }
        }
    }

    impl<'a> MakeWriter for MockMakeWriter<'a> {
        type Writer = MockWriter<'a>;

        fn make_writer(&self) -> Self::Writer {
            MockWriter::new(self.buf)
        }
    }

    #[test]
    fn impls() {
        let f = Format::default().with_timer(time::Uptime::default());
        let subscriber = Collector::builder().event_format(f).finish();
        let _dispatch = Dispatch::new(subscriber);

        let f = format::Format::default();
        let subscriber = Collector::builder().event_format(f).finish();
        let _dispatch = Dispatch::new(subscriber);

        let f = format::Format::default().compact();
        let subscriber = Collector::builder().event_format(f).finish();
        let _dispatch = Dispatch::new(subscriber);
    }

    #[test]
    fn subscriber_downcasts() {
        let subscriber = Collector::builder().finish();
        let dispatch = Dispatch::new(subscriber);
        assert!(dispatch.downcast_ref::<Collector>().is_some());
    }

    #[test]
    fn subscriber_downcasts_to_parts() {
        let subscriber = Collector::new();
        let dispatch = Dispatch::new(subscriber);
        assert!(dispatch.downcast_ref::<format::DefaultFields>().is_some());
        assert!(dispatch.downcast_ref::<LevelFilter>().is_some());
        assert!(dispatch.downcast_ref::<format::Format>().is_some())
    }

    #[test]
    fn is_lookup_span() {
        fn assert_lookup_span<T: for<'a> crate::registry::LookupSpan<'a>>(_: T) {}
        let subscriber = Collector::new();
        assert_lookup_span(subscriber)
    }
}
