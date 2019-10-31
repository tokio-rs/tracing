//! A `Subscriber` for formatting and logging `tracing` data.
//!
//! ## Overview
//!
//! [`tracing`] is a framework for instrumenting Rust programs with context-aware,
//! structured, event-based diagnostic information. This crate provides an
//! implementation of the [`Subscriber`] trait that records `tracing`'s `Event`s
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
//! Add the following to your executable to initialize the default subscriber:
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
//! ### Subscriber
//!
//! The [`FmtSubscriber`] formats and records `tracing` events as line-oriented logs.
//! You can create one by calling:
//!
//! ```rust
//! use tracing_subscriber::FmtSubscriber;
//!
//! let subscriber = FmtSubscriber::builder()
//!     // ... add configuration
//!     .finish();
//! ```
//!
//! You can find the configuration methods for [`FmtSubscriber`] in [`fmt::Builder`].
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
//! ### Using Your Subscriber
//!
//! Finally, once you have configured your `Subscriber`, you need to
//! configure your executable to use it.
//!
//! A subscriber can be installed globally using:
//! ```rust
//! use tracing;
//! use tracing_subscriber::FmtSubscriber;
//!
//! let subscriber = FmtSubscriber::new();
//!
//! tracing::subscriber::set_global_default(subscriber)
//!     .map_err(|_err| eprintln!("Unable to set global default subscriber"));
//! // Note this will only fail if you try to set the global default
//! // subscriber multiple times
//! ```
//! [`EnvFilter`]: ../filter/struct.EnvFilter.html
//! [`env_logger`]: https://docs.rs/env_logger/
//! [`filter`]: ../filter/index.html
//! [`fmt::Builder`]: ./struct.Builder.html
//! [`FmtSubscriber`]: ./struct.Subscriber.html
//! [`Subscriber`]:
//!     https://docs.rs/tracing/latest/tracing/trait.Subscriber.html
//! [`tracing`]: https://crates.io/crates/tracing
use std::{cell::RefCell, error::Error, io};
use tracing_core::{subscriber::Interest, Event, Metadata};

pub mod format;
pub(crate) mod span;
pub mod time;
pub mod writer;

use crate::{
    filter::LevelFilter,
    layer::{self, Layer},
    registry::{LookupMetadata, LookupSpan},
};

#[doc(inline)]
pub use self::{
    format::{FormatEvent, FormatFields},
    span::Context,
    writer::MakeWriter,
};

/// A `Subscriber` that logs formatted representations of `tracing` events.
///
/// This consists of an inner `Formatter` wrapped in a layer that performs filtering.
#[derive(Debug)]
pub struct Subscriber<
    N = format::DefaultFields,
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
    N = format::DefaultFields,
    E = format::Format<format::Full>,
    W = fn() -> io::Stdout,
> {
    fmt_fields: N,
    fmt_event: E,
    spans: span::Store,
    settings: Settings,
    make_writer: W,
}

/// Configures and constructs `Subscriber`s.
#[derive(Debug)]
pub struct Builder<
    N = format::DefaultFields,
    E = format::Format<format::Full>,
    F = LevelFilter,
    W = fn() -> io::Stdout,
> {
    filter: F,
    fmt_fields: N,
    fmt_event: E,
    settings: Settings,
    make_writer: W,
}

#[derive(Debug)]
struct Settings {
    inherit_fields: bool,
    initial_span_capacity: usize,
}

// impl Subscriber {
//     /// The maximum [verbosity level] that is enabled by a `Subscriber` by
//     /// default.
//     ///
//     /// This can be overridden with the [`Builder::with_max_level`] method.
//     ///
//     /// [verbosity level]: https://docs.rs/tracing-core/0.1.5/tracing_core/struct.Level.html
//     /// [`Builder::with_max_level`]: struct.Builder.html#method.with_max_level
//     pub const DEFAULT_MAX_LEVEL: LevelFilter = LevelFilter::INFO;

//     /// Returns a new `Builder` for configuring a format subscriber.
//     pub fn builder() -> Builder {
//         Builder::default()
//     }

//     /// Returns a new format subscriber with the default configuration.
//     pub fn new() -> Self {
//         Default::default()
//     }
// }

// impl Default for Subscriber {
//     fn default() -> Self {
//         Builder::default().finish()
//     }
// }
// === impl Subscriber ===

// impl<N, E, F, W> tracing_core::Subscriber for Subscriber<N, E, F, W>
// where
//     S: tracing_core::Subscriber + for<'a> LookupSpan<'a> + LookupMetadata,
//     N: for<'writer> FormatFields<'writer> + 'static,
//     E: FormatEvent<S, N> + 'static,
//     F: Layer<Formatter<N, E, W>> + 'static,
//     W: MakeWriter + 'static,
//     layer::Layered<F, Formatter<N, E, W>>: tracing_core::Subscriber,
// {
//     #[inline]
//     fn register_callsite(&self, meta: &'static Metadata<'static>) -> Interest {
//         self.inner.register_callsite(meta)
//     }

//     #[inline]
//     fn enabled(&self, meta: &Metadata<'_>) -> bool {
//         self.inner.enabled(meta)
//     }

//     #[inline]
//     fn new_span(&self, attrs: &span::Attributes<'_>) -> span::Id {
//         self.inner.new_span(attrs)
//     }

//     #[inline]
//     fn record(&self, span: &span::Id, values: &span::Record<'_>) {
//         self.inner.record(span, values)
//     }

//     #[inline]
//     fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {
//         self.inner.record_follows_from(span, follows)
//     }

//     #[inline]
//     fn event(&self, event: &Event<'_>) {
//         self.inner.event(event);
//     }

//     #[inline]
//     fn enter(&self, id: &span::Id) {
//         // TODO: add on_enter hook
//         self.inner.enter(id);
//     }

//     #[inline]
//     fn exit(&self, id: &span::Id) {
//         self.inner.exit(id);
//     }

//     #[inline]
//     fn current_span(&self) -> span::Current {
//         self.inner.current_span()
//     }

//     #[inline]
//     fn clone_span(&self, id: &span::Id) -> span::Id {
//         self.inner.clone_span(id)
//     }

//     #[inline]
//     fn try_close(&self, id: span::Id) -> bool {
//         self.inner.try_close(id)
//     }

//     unsafe fn downcast_raw(&self, id: TypeId) -> Option<*const ()> {
//         if id == TypeId::of::<Self>() {
//             Some(self as *const Self as *const ())
//         } else {
//             self.inner.downcast_raw(id)
//         }
//     }
// }

#[cfg(feature = "registry_unstable")]
impl<N, E, F, W> crate::registry::LookupMetadata for Subscriber<N, E, F, W>
where
    layer::Layered<F, Formatter<N, E, W>>: crate::registry::LookupMetadata,
{
    #[inline]
    fn metadata(&self, id: &span::Id) -> Option<&'static Metadata<'static>> {
        self.inner.metadata(id)
    }
}

// === impl Formatter ===

impl<N, E, W> Formatter<N, E, W>
where
    N: for<'writer> FormatFields<'writer>,
{
    #[inline]
    fn ctx(&self) -> span::Context<'_, N> {
        span::Context::new(&self.spans, &self.fmt_fields)
    }
}

// impl<S, N, E, W> tracing_core::Subscriber for Formatter<N, E, W>
// where
//     N: for<'writer> FormatFields<'writer> + 'static,
//     E: FormatEvent<S, N> + 'static,
//     W: MakeWriter + 'static,
// {
//     fn register_callsite(&self, _: &'static Metadata<'static>) -> Interest {
//         Interest::always()
//     }

//     fn enabled(&self, _: &Metadata<'_>) -> bool {
//         true
//     }

//     #[inline]
//     fn new_span(&self, attrs: &span::Attributes<'_>) -> span::Id {
//         self.spans.new_span(attrs, &self.fmt_fields)
//     }

//     #[inline]
//     fn record(&self, span: &span::Id, values: &span::Record<'_>) {
//         self.spans.record(span, values, &self.fmt_fields)
//     }

//     fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {
//         // TODO: implement this please
//     }

//     fn event(&self, event: &Event<'_>) {
//         thread_local! {
//             static BUF: RefCell<String> = RefCell::new(String::new());
//         }

//         BUF.with(|buf| {
//             let borrow = buf.try_borrow_mut();
//             let mut a;
//             let mut b;
//             let buf = match borrow {
//                 Ok(buf) => {
//                     a = buf;
//                     &mut *a
//                 }
//                 _ => {
//                     b = String::new();
//                     &mut b
//                 }
//             };

//             if self.fmt_event.format_event(&self.ctx(), buf, event).is_ok() {
//                 let mut writer = self.make_writer.make_writer();
//                 let _ = io::Write::write_all(&mut writer, buf.as_bytes());
//             }

//             buf.clear();
//         });
//     }

//     fn enter(&self, id: &span::Id) {
//         // TODO: add on_enter hook
//         self.spans.push(id);
//     }

//     fn exit(&self, id: &span::Id) {
//         self.spans.pop(id);
//     }

//     fn current_span(&self) -> span::Current {
//         if let Some(id) = self.spans.current() {
//             if let Some(meta) = self.spans.get(&id).map(|span| span.metadata()) {
//                 return span::Current::new(id, meta);
//             }
//         }
//         span::Current::none()
//     }

//     #[inline]
//     fn clone_span(&self, id: &span::Id) -> span::Id {
//         self.spans.clone_span(id)
//     }

//     #[inline]
//     fn try_close(&self, id: span::Id) -> bool {
//         self.spans.drop_span(id)
//     }

//     unsafe fn downcast_raw(&self, id: TypeId) -> Option<*const ()> {
//         match () {
//             _ if id == TypeId::of::<Self>() => Some(self as *const Self as *const ()),
//             // _ if id == TypeId::of::<F>() => Some(&self.filter as *const F as *const ()),
//             _ if id == TypeId::of::<E>() => Some(&self.fmt_event as *const E as *const ()),
//             _ if id == TypeId::of::<N>() => Some(&self.fmt_fields as *const N as *const ()),
//             _ => None,
//         }
//     }
// }

#[cfg(feature = "registry_unstable")]
impl<N, E, W> crate::registry::LookupMetadata for Formatter<N, E, W> {
    fn metadata(&self, id: &span::Id) -> Option<&'static Metadata<'static>> {
        self.spans.get(&id).map(|span| span.metadata())
    }
}

// ===== impl Builder =====

// impl Default for Builder {
//     fn default() -> Self {
//         Builder {
//             filter: Subscriber::DEFAULT_MAX_LEVEL,
//             fmt_fields: format::DefaultFields::default(),
//             fmt_event: format::Format::default(),
//             settings: Settings::default(),
//             make_writer: io::stdout,
//         }
//     }
// }

// impl<S, N, E, F, W> Builder<N, E, F, W>
// where
//     N: for<'writer> FormatFields<'writer> + 'static,
//     E: FormatEvent<S, N> + 'static,
//     W: MakeWriter + 'static,
//     F: Layer<Formatter<N, E, W>> + 'static,
// {
//     /// Finish the builder, returning a new `FmtSubscriber`.
//     pub fn finish(self) -> Subscriber<N, E, F, W> {
//         let subscriber = Formatter {
//             fmt_fields: self.fmt_fields,
//             fmt_event: self.fmt_event,
//             spans: span::Store::with_capacity(self.settings.initial_span_capacity),
//             settings: self.settings,
//             make_writer: self.make_writer,
//         };
//         Subscriber {
//             inner: self.filter.with_subscriber(subscriber),
//         }
//     }
// }

// impl<S, N, E, F, W> Builder<N, E, F, W>
// where
//     N: for<'writer> FormatFields<'writer> + 'static,
//     E: FormatEvent<S, N> + 'static,
//     W: MakeWriter + 'static,
//     F: Layer<Formatter<N, E, W>> + 'static,
//     Subscriber<N, E, F, W>: Send + Sync,
// {
//     /// Install this Subscriber as the global default if one is
//     /// not already set.
//     ///
//     /// If the `tracing-log` feature is enabled, this will also install
//     /// the LogTracer to convert `Log` records into `tracing` `Event`s.
//     ///
//     /// # Errors
//     /// Returns an Error if the initialization was unsuccessful, likely
//     /// because a global subscriber was already installed by another
//     /// call to `try_init`.
//     pub fn try_init(self) -> Result<(), impl Error + Send + Sync + 'static> {
//         #[cfg(feature = "tracing-log/std")]
//         tracing_log::LogTracer::init().map_err(Box::new)?;

//         tracing_core::dispatcher::set_global_default(tracing_core::dispatcher::Dispatch::new(
//             self.finish(),
//         ))
//         .map_err(Box::new)
//     }

//     /// Install this Subscriber as the global default.
//     ///
//     /// If the `tracing-log` feature is enabled, this will also install
//     /// the LogTracer to convert `Log` records into `tracing` `Event`s.
//     ///
//     /// # Panics
//     /// Panics if the initialization was unsuccessful, likely because a
//     /// global subscriber was already installed by another call to `try_init`.
//     pub fn init(self) {
//         self.try_init()
//             .expect("Unable to install global subscriber")
//     }
// }

impl<N, L, T, F, W> Builder<N, format::Format<L, T>, F, W>
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
    pub fn with_timer<T2>(self, timer: T2) -> Builder<N, format::Format<L, T2>, F, W> {
        Builder {
            fmt_fields: self.fmt_fields,
            fmt_event: self.fmt_event.with_timer(timer),
            filter: self.filter,
            settings: self.settings,
            make_writer: self.make_writer,
        }
    }

    /// Do not emit timestamps with log messages.
    pub fn without_time(self) -> Builder<N, format::Format<L, ()>, F, W> {
        Builder {
            fmt_fields: self.fmt_fields,
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

#[cfg(feature = "env-filter")]
impl<N, E, W> Builder<N, E, crate::EnvFilter, W>
where
    Formatter<N, E, W>: tracing_core::Subscriber + 'static,
{
    /// Configures the subscriber being built to allow filter reloading at
    /// runtime.
    pub fn with_filter_reloading(
        self,
    ) -> Builder<N, E, crate::reload::Layer<crate::EnvFilter, Formatter<N, E, W>>, W> {
        let (filter, _) = crate::reload::Layer::new(self.filter);
        Builder {
            fmt_fields: self.fmt_fields,
            fmt_event: self.fmt_event,
            filter,
            settings: self.settings,
            make_writer: self.make_writer,
        }
    }
}

#[cfg(feature = "env-filter")]
impl<N, E, W> Builder<N, E, crate::reload::Layer<crate::EnvFilter, Formatter<N, E, W>>, W>
where
    Formatter<N, E, W>: tracing_core::Subscriber + 'static,
{
    /// Returns a `Handle` that may be used to reload the constructed subscriber's
    /// filter.
    pub fn reload_handle(&self) -> crate::reload::Handle<crate::EnvFilter, Formatter<N, E, W>> {
        self.filter.handle()
    }
}

// impl<N, E, F, W> Builder<N, E, F, W> {
//     /// Sets the Visitor that the subscriber being built will use to record
//     /// fields.
//     ///
//     /// For example:
//     /// ```rust
//     /// use tracing_subscriber::fmt::{Subscriber, format};
//     /// use tracing_subscriber::prelude::*;
//     ///
//     /// let formatter =
//     ///     // Construct a custom formatter for `Debug` fields
//     ///     format::debug_fn(|writer, field, value| write!(writer, "{}: {:?}", field, value))
//     ///         // Use the `tracing_subscriber::MakeFmtExt` trait to wrap the
//     ///         // formatter so that a delimiter is added between fields.
//     ///         .delimited(", ");
//     ///
//     /// let subscriber = Subscriber::builder()
//     ///     .fmt_fields(formatter)
//     ///     .finish();
//     /// # drop(subscriber)
//     /// ```
//     pub fn fmt_fields<N2>(self, fmt_fields: N2) -> Builder<N2, E, F, W>
//     where
//         N2: for<'writer> FormatFields<'writer> + 'static,
//     {
//         Builder {
//             fmt_fields: fmt_fields.into(),
//             fmt_event: self.fmt_event,
//             filter: self.filter,
//             settings: self.settings,
//             make_writer: self.make_writer,
//         }
//     }

//     /// Sets the [`EnvFilter`] that the subscriber will use to determine if
//     /// a span or event is enabled.
//     ///
//     /// Note that this method requires the "env-filter" feature flag to be enabled.
//     ///
//     /// If a filter was previously set, or a maximum level was set by the
//     /// [`with_max_level`] method, that value is replaced by the new filter.
//     ///
//     /// # Examples
//     ///
//     /// Setting a filter based on the value of the `RUST_LOG` environment
//     /// variable:
//     /// ```rust
//     /// use tracing_subscriber::{FmtSubscriber, EnvFilter};
//     ///
//     /// let subscriber = FmtSubscriber::builder()
//     ///     .with_env_filter(EnvFilter::from_default_env())
//     ///     .finish();
//     /// ```
//     ///
//     /// Setting a filter based on a pre-set filter directive string:
//     /// ```rust
//     /// use tracing_subscriber::FmtSubscriber;
//     ///
//     /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
//     /// let subscriber = FmtSubscriber::builder()
//     ///     .with_env_filter("my_crate=info,my_crate::my_mod=debug,[my_span]=trace")
//     ///     .finish();
//     /// # Ok(()) }
//     /// ```
//     ///
//     /// Adding additional directives to a filter constructed from an env var:
//     /// ```rust
//     /// use tracing_subscriber::{
//     ///     FmtSubscriber,
//     ///     filter::{EnvFilter, LevelFilter},
//     /// };
//     ///
//     /// # fn filter() -> Result<(), Box<dyn std::error::Error>> {
//     /// let filter = EnvFilter::try_from_env("MY_CUSTOM_FILTER_ENV_VAR")?
//     ///     // Set the base level when not matched by other directives to WARN.
//     ///     .add_directive(LevelFilter::WARN.into())
//     ///     // Set the max level for `my_crate::my_mod` to DEBUG, overriding
//     ///     // any directives parsed from the env variable.
//     ///     .add_directive("my_crate::my_mod=debug".parse()?);
//     ///
//     /// let subscriber = FmtSubscriber::builder()
//     ///     .with_env_filter(filter)
//     ///     .finish();
//     /// # Ok(())}
//     /// ```
//     /// [`EnvFilter`]: ../filter/struct.EnvFilter.html
//     /// [`with_max_level`]: #method.with_max_level
//     #[cfg(feature = "env-filter")]
//     pub fn with_env_filter(
//         self,
//         filter: impl Into<crate::EnvFilter>,
//     ) -> Builder<N, E, crate::EnvFilter, W>
//     where
//         Formatter<N, E, W>: tracing_core::Subscriber + 'static,
//     {
//         let filter = filter.into();
//         Builder {
//             fmt_fields: self.fmt_fields,
//             fmt_event: self.fmt_event,
//             filter,
//             settings: self.settings,
//             make_writer: self.make_writer,
//         }
//     }

//     /// Sets the [`EnvFilter`] that the subscriber will use to determine if
//     /// a span or event is enabled.
//     ///
//     /// **Note**: this method was renamed to [`with_env_filter`] in version
//     /// 0.1.2. This method just wraps a call to `with_env_filter`, and will be
//     /// removed in version 0.2.
//     ///
//     /// [`EnvFilter`]: ../filter/struct.EnvFilter.html
//     /// [`with_env_filter`]: #method.with_env_filter
//     #[cfg(feature = "env-filter")]
//     #[deprecated(since = "0.1.2", note = "renamed to `with_env_filter`")]
//     pub fn with_filter(
//         self,
//         filter: impl Into<crate::EnvFilter>,
//     ) -> Builder<N, E, crate::EnvFilter, W>
//     where
//         Formatter<N, E, W>: tracing_core::Subscriber + 'static,
//     {
//         self.with_env_filter(filter)
//     }

//     /// Sets the maximum [verbosity level] that will be enabled by the
//     /// subscriber.
//     ///
//     /// If the max level has already been set, or a [`EnvFilter`] was added by
//     /// [`with_filter`], this replaces that configuration with the new
//     /// maximum level.
//     ///
//     /// # Examples
//     ///
//     /// Enable up to the `DEBUG` verbosity level:
//     /// ```rust
//     /// use tracing_subscriber::FmtSubscriber;
//     /// use tracing::Level;
//     ///
//     /// let subscriber = FmtSubscriber::builder()
//     ///     .with_max_level(Level::DEBUG)
//     ///     .finish();
//     /// ```
//     /// This subscriber won't record any spans or events!
//     /// ```rust
//     /// use tracing_subscriber::{
//     ///     FmtSubscriber,
//     ///     filter::LevelFilter,
//     /// };
//     ///
//     /// let subscriber = FmtSubscriber::builder()
//     ///     .with_max_level(LevelFilter::OFF)
//     ///     .finish();
//     /// ```
//     /// [verbosity level]: https://docs.rs/tracing-core/0.1.5/tracing_core/struct.Level.html
//     /// [`EnvFilter`]: ../filter/struct.EnvFilter.html
//     /// [`with_filter`]: #method.with_filter
//     pub fn with_max_level(self, filter: impl Into<LevelFilter>) -> Builder<N, E, LevelFilter, W> {
//         let filter = filter.into();
//         Builder {
//             fmt_fields: self.fmt_fields,
//             fmt_event: self.fmt_event,
//             filter,
//             settings: self.settings,
//             make_writer: self.make_writer,
//         }
//     }

//     /// Sets the subscriber being built to use a less verbose formatter.
//     ///
//     /// See [`format::Compact`].
//     pub fn compact(self) -> Builder<N, format::Format<format::Compact>, F, W>
//     where
//         N: for<'writer> FormatFields<'writer> + 'static,
//     {
//         Builder {
//             fmt_event: format::Format::default().compact(),
//             filter: self.filter,
//             fmt_fields: self.fmt_fields,
//             settings: self.settings,
//             make_writer: self.make_writer,
//         }
//     }

//     /// Sets the subscriber being built to use a JSON formatter.
//     ///
//     /// See [`format::Json`]
//     #[cfg(feature = "json")]
//     pub fn json(self) -> Builder<format::JsonFields, format::Format<format::Json>, F, W>
//     where
//         N: for<'writer> FormatFields<'writer> + 'static,
//     {
//         Builder {
//             fmt_event: format::Format::default().json(),
//             filter: self.filter,
//             fmt_fields: format::JsonFields::default(),
//             settings: self.settings,
//             make_writer: self.make_writer,
//         }
//     }

//     /// Sets the function that the subscriber being built should use to format
//     /// events that occur.
//     pub fn on_event<S, E2>(self, fmt_event: E2) -> Builder<N, E2, F, W>
//     where
//         E2: FormatEvent<S, N> + 'static,
//     {
//         Builder {
//             fmt_fields: self.fmt_fields,
//             fmt_event,
//             filter: self.filter,
//             settings: self.settings,
//             make_writer: self.make_writer,
//         }
//     }

//     /// Sets whether or not spans inherit their parents' field values (disabled
//     /// by default).
//     pub fn inherit_fields(self, inherit_fields: bool) -> Self {
//         Builder {
//             settings: Settings {
//                 inherit_fields,
//                 ..self.settings
//             },
//             ..self
//         }
//     }

//     // TODO(eliza): should this be publicly exposed?
//     // /// Configures the initial capacity for the span slab used to store
//     // /// in-progress span data. This may be used for tuning the subscriber's
//     // /// allocation performance, but in general does not need to be manually configured..
//     // pub fn initial_span_capacity(self, initial_span_capacity: usize) -> Self {
//     //     Builder {
//     //         settings: Settings {
//     //             initial_span_capacity,
//     //             ..self.settings
//     //         },
//     //         ..self
//     //     }
//     // }
// }

// impl<N, E, F, W> Builder<N, E, F, W> {
//     /// Sets the [`MakeWriter`] that the subscriber being built will use to write events.
//     ///
//     /// # Examples
//     ///
//     /// Using `stderr` rather than `stdout`:
//     ///
//     /// ```rust
//     /// use std::io;
//     ///
//     /// let subscriber = tracing_subscriber::fmt::Subscriber::builder()
//     ///     .with_writer(io::stderr)
//     ///     .finish();
//     /// ```
//     ///
//     /// [`MakeWriter`]: trait.MakeWriter.html
//     pub fn with_writer<W2>(self, make_writer: W2) -> Builder<N, E, F, W2>
//     where
//         W2: MakeWriter + 'static,
//     {
//         Builder {
//             fmt_fields: self.fmt_fields,
//             fmt_event: self.fmt_event,
//             filter: self.filter,
//             settings: self.settings,
//             make_writer,
//         }
//     }
// }

// impl Default for Settings {
//     fn default() -> Self {
//         Self {
//             inherit_fields: false,
//             initial_span_capacity: 32,
//         }
//     }
// }

/// Install a global tracing subscriber that listens for events and
/// filters based on the value of the [`RUST_LOG` environment variable],
/// if one is not already set.
///
/// If the `tracing-log` feature is enabled, this will also install
/// the [`LogTracer`] to convert `log` records into `tracing` `Event`s.
///
///
/// # Errors
/// Returns an Error if the initialization was unsuccessful,
/// likely because a global subscriber was already installed by another
/// call to `try_init`.
///
/// [`LogTracer`]:
///     https://docs.rs/tracing-log/0.1.0/tracing_log/struct.LogTracer.html
/// [`RUST_LOG` environment variable]:
///     ../filter/struct.EnvFilter.html#associatedconstant.DEFAULT_ENV
pub fn try_init() -> Result<(), String> {
    unimplemented!()
    // Subscriber::builder()
    //     .with_env_filter(crate::EnvFilter::from_default_env())
    //     .try_init()
}

/// Install a global tracing subscriber that listens for events and
/// filters based on the value of the [`RUST_LOG` environment variable].
///
/// If the `tracing-log` feature is enabled, this will also install
/// the LogTracer to convert `Log` records into `tracing` `Event`s.
///
/// # Panics
/// Panics if the initialization was unsuccessful, likely because a
/// global subscriber was already installed by another call to `try_init`.
///
/// [`RUST_LOG` environment variable]:
///     ../filter/struct.EnvFilter.html#associatedconstant.DEFAULT_ENV
pub fn init() {
    unimplemented!()
    // try_init().expect("Unable to install global subscriber")
}

// #[cfg(test)]
// mod test {
//     use super::{writer::MakeWriter, *};
//     use crate::fmt::Subscriber;
//     use std::{
//         io,
//         sync::{Mutex, MutexGuard, TryLockError},
//     };
//     use tracing_core::dispatcher::Dispatch;

//     pub(crate) struct MockWriter<'a> {
//         buf: &'a Mutex<Vec<u8>>,
//     }

//     impl<'a> MockWriter<'a> {
//         pub(crate) fn new(buf: &'a Mutex<Vec<u8>>) -> Self {
//             Self { buf }
//         }

//         pub(crate) fn map_error<Guard>(err: TryLockError<Guard>) -> io::Error {
//             match err {
//                 TryLockError::WouldBlock => io::Error::from(io::ErrorKind::WouldBlock),
//                 TryLockError::Poisoned(_) => io::Error::from(io::ErrorKind::Other),
//             }
//         }

//         pub(crate) fn buf(&self) -> io::Result<MutexGuard<'a, Vec<u8>>> {
//             self.buf.try_lock().map_err(Self::map_error)
//         }
//     }

//     impl<'a> io::Write for MockWriter<'a> {
//         fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
//             self.buf()?.write(buf)
//         }

//         fn flush(&mut self) -> io::Result<()> {
//             self.buf()?.flush()
//         }
//     }

//     pub(crate) struct MockMakeWriter<'a> {
//         buf: &'a Mutex<Vec<u8>>,
//     }

//     impl<'a> MockMakeWriter<'a> {
//         pub(crate) fn new(buf: &'a Mutex<Vec<u8>>) -> Self {
//             Self { buf }
//         }
//     }

//     impl<'a> MakeWriter for MockMakeWriter<'a> {
//         type Writer = MockWriter<'a>;

//         fn make_writer(&self) -> Self::Writer {
//             MockWriter::new(self.buf)
//         }
//     }

//     #[test]
//     fn impls() {
//         let f = format::Format::default().with_timer(time::Uptime::default());
//         let subscriber = Subscriber::builder().on_event(f).finish();
//         let _dispatch = Dispatch::new(subscriber);

//         let f = format::Format::default();
//         let subscriber = Subscriber::builder().on_event(f).finish();
//         let _dispatch = Dispatch::new(subscriber);

//         let f = format::Format::default().compact();
//         let subscriber = Subscriber::builder().on_event(f).finish();
//         let _dispatch = Dispatch::new(subscriber);
//     }

//     #[test]
//     fn subscriber_downcasts() {
//         let subscriber = Subscriber::new();
//         let dispatch = Dispatch::new(subscriber);
//         assert!(dispatch.downcast_ref::<Subscriber>().is_some());
//     }

//     #[test]
//     fn subscriber_downcasts_to_parts() {
//         let subscriber = Subscriber::builder().finish();
//         let dispatch = Dispatch::new(subscriber);
//         assert!(dispatch.downcast_ref::<format::DefaultFields>().is_some());
//         assert!(dispatch.downcast_ref::<LevelFilter>().is_some());
//         assert!(dispatch.downcast_ref::<format::Format>().is_some())
//     }

//     #[test]
//     #[cfg(feature = "registry_unstable")]
//     fn is_lookup_meta() {
//         fn assert_lookup_meta<T: crate::registry::LookupMetadata>(_: T) {}
//         assert_lookup_meta(Subscriber::builder().finish())
//     }
// }
