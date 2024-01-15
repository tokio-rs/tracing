//! The [`Subscribe`] trait, a composable abstraction for building [collector]s.
//!
//! The [`Collect`] trait in `tracing-core` represents the _complete_ set of
//! functionality required to consume `tracing` instrumentation. This means that
//! a single `Collect` instance is a self-contained implementation of a
//! complete strategy for collecting traces; but it _also_ means that the
//! `Collect` trait cannot easily be composed with other collectors.
//!
//! In particular, [collector]s are responsible for generating [span IDs] and
//! assigning them to spans. Since these IDs must uniquely identify a span
//! within the context of the current trace, this means that there may only be
//! a single collector for a given thread at any point in time &mdash;
//! otherwise, there would be no authoritative source of span IDs.
//!
//! On the other hand, the majority of the [`Collect`] trait's functionality
//! is composable: any number of subscribers may _observe_ events, span entry
//! and exit, and so on, provided that there is a single authoritative source of
//! span IDs. The [`Subscribe`] trait represents this composable subset of the
//! [`Collect`] behavior; it can _observe_ events and spans, but does not
//! assign IDs.
//!
//! # Composing Subscribers
//!
//! Since a [subscriber] does not implement a complete strategy for collecting
//! traces, it must be composed with a [collector] in order to be used. The
//! [`Subscribe`] trait is generic over a type parameter (called `C` in the trait
//! definition), representing the types of `Collect` they can be composed
//! with. Thus, a subscriber may be implemented that will only compose with a
//! particular `Collect` implementation, or additional trait bounds may be
//! added to constrain what types implementing `Collect` a subscriber can wrap.
//!
//! Subscribers may be added to a collector by using the [`CollectExt::with`]
//! method, which is provided by `tracing-subscriber`'s [prelude]. This method
//! returns a [`Layered`] struct that implements [`Collect`] by composing the
//! `Subscribe` with the collector.
//!
//! For example:
//! ```rust
//! use tracing_subscriber::Subscribe;
//! use tracing_subscriber::prelude::*;
//! use tracing::Collect;
//!
//! pub struct MySubscriber {
//!     // ...
//! }
//!
//! impl<C: Collect> Subscribe<C> for MySubscriber {
//!     // ...
//! }
//!
//! pub struct MyCollector {
//!     // ...
//! }
//!
//! # use tracing_core::{span::{Id, Attributes, Record}, Metadata, Event};
//! impl Collect for MyCollector {
//!     // ...
//! #   fn new_span(&self, _: &Attributes) -> Id { Id::from_u64(1) }
//! #   fn record(&self, _: &Id, _: &Record) {}
//! #   fn event(&self, _: &Event) {}
//! #   fn record_follows_from(&self, _: &Id, _: &Id) {}
//! #   fn enabled(&self, _: &Metadata) -> bool { false }
//! #   fn enter(&self, _: &Id) {}
//! #   fn exit(&self, _: &Id) {}
//! #   fn current_span(&self) -> tracing_core::span::Current { tracing_core::span::Current::none() }
//! }
//! # impl MySubscriber {
//! # fn new() -> Self { Self {} }
//! # }
//! # impl MyCollector {
//! # fn new() -> Self { Self { }}
//! # }
//!
//! let collector = MyCollector::new()
//!     .with(MySubscriber::new());
//!
//! tracing::collect::set_global_default(collector);
//! ```
//!
//! Multiple subscriber may be composed in the same manner:
//! ```rust
//! # use tracing_subscriber::{Subscribe, subscribe::CollectExt};
//! # use tracing::Collect;
//! pub struct MyOtherSubscriber {
//!     // ...
//! }
//!
//! impl<C: Collect> Subscribe<C> for MyOtherSubscriber {
//!     // ...
//! }
//!
//! pub struct MyThirdSubscriber {
//!     // ...
//! }
//!
//! impl<C: Collect> Subscribe<C> for MyThirdSubscriber {
//!     // ...
//! }
//! # pub struct MySubscriber {}
//! # impl<C: Collect> Subscribe<C> for MySubscriber {}
//! # pub struct MyCollector { }
//! # use tracing_core::{span::{Id, Attributes, Record}, Metadata, Event};
//! # impl Collect for MyCollector {
//! #   fn new_span(&self, _: &Attributes) -> Id { Id::from_u64(1) }
//! #   fn record(&self, _: &Id, _: &Record) {}
//! #   fn event(&self, _: &Event) {}
//! #   fn record_follows_from(&self, _: &Id, _: &Id) {}
//! #   fn enabled(&self, _: &Metadata) -> bool { false }
//! #   fn current_span(&self) -> tracing_core::span::Current { tracing_core::span::Current::none() }
//! #   fn enter(&self, _: &Id) {}
//! #   fn exit(&self, _: &Id) {}
//! }
//! # impl MySubscriber {
//! # fn new() -> Self { Self {} }
//! # }
//! # impl MyOtherSubscriber {
//! # fn new() -> Self { Self {} }
//! # }
//! # impl MyThirdSubscriber {
//! # fn new() -> Self { Self {} }
//! # }
//! # impl MyCollector {
//! # fn new() -> Self { Self { }}
//! # }
//!
//! let collect = MyCollector::new()
//!     .with(MySubscriber::new())
//!     .with(MyOtherSubscriber::new())
//!     .with(MyThirdSubscriber::new());
//!
//! tracing::collect::set_global_default(collect);
//! ```
//!
//! The [`Subscribe::with_collector`] constructs the [`Layered`] type from a
//! [`Subscribe`] and [`Collect`], and is called by [`CollectExt::with`]. In
//! general, it is more idiomatic to use [`CollectExt::with`], and treat
//! [`Subscribe::with_collector`] as an implementation detail, as `with_collector`
//! calls must be nested, leading to less clear code for the reader.
//!
//! ## Runtime Configuration With Subscribers
//!
//! In some cases, a particular [subscriber] may be enabled or disabled based on
//! runtime configuration. This can introduce challenges, because the type of a
//! layered [collector] depends on which subscribers are added to it: if an `if`
//! or `match` expression adds some [`Subscribe`] implementation in one branch,
//! and other subscribers in another, the [collector] values returned by those
//! branches will have different types. For example, the following _will not_
//! work:
//!
//! ```compile_fail
//! # fn docs() -> Result<(), Box<dyn std::error::Error + 'static>> {
//! # struct Config {
//! #    is_prod: bool,
//! #    path: &'static str,
//! # }
//! # let cfg = Config { is_prod: false, path: "debug.log" };
//! use std::fs::File;
//! use tracing_subscriber::{Registry, prelude::*};
//!
//! let stdout_log = tracing_subscriber::fmt::subscriber().pretty();
//! let collector = Registry::default().with(stdout_log);
//!
//! // The compile error will occur here because the if and else
//! // branches have different (and therefore incompatible) types.
//! let collector = if cfg.is_prod {
//!     let file = File::create(cfg.path)?;
//!     let collector = tracing_subscriber::fmt::subscriber()
//!         .json()
//!         .with_writer(Arc::new(file));
//!     collector.with(subscriber)
//! } else {
//!     collector
//! };
//!
//! tracing::collect::set_global_default(collector)
//!     .expect("Unable to set global collector");
//! # Ok(()) }
//! ```
//!
//! However, a [`Subscribe`] wrapped in an [`Option`] [also implements the `Subscribe`
//! trait][option-impl]. This allows individual layers to be enabled or disabled at
//! runtime while always producing a [`Collect`] of the same type. For
//! example:
//!
//! ```
//! # fn docs() -> Result<(), Box<dyn std::error::Error + 'static>> {
//! # struct Config {
//! #    is_prod: bool,
//! #    path: &'static str,
//! # }
//! # let cfg = Config { is_prod: false, path: "debug.log" };
//! use std::fs::File;
//! use tracing_subscriber::{Registry, prelude::*};
//!
//! let stdout_log = tracing_subscriber::fmt::subscriber().pretty();
//! let collector = Registry::default().with(stdout_log);
//!
//! // if `cfg.is_prod` is true, also log JSON-formatted logs to a file.
//! let json_log = if cfg.is_prod {
//!     let file = File::create(cfg.path)?;
//!     let json_log = tracing_subscriber::fmt::subscriber()
//!         .json()
//!         .with_writer(file);
//!     Some(json_log)
//! } else {
//!     None
//! };
//!
//! // If `cfg.is_prod` is false, then `json` will be `None`, and this subscriber
//! // will do nothing. However, the collector will still have the same type
//! // regardless of whether the `Option`'s value is `None` or `Some`.
//! let collector = collector.with(json_log);
//!
//! tracing::collect::set_global_default(collector)
//!    .expect("Unable to set global collector");
//! # Ok(()) }
//! ```
//!
//! If a subscriber may be one of several different types, note that [`Box<dyn
//! Subscribe<C> + Send + Sync + 'static>` implements `Subscribe`][box-impl].
//! This may be used to erase the type of a subscriber.
//!
//! For example, a function that configures a subscriber to log to one of
//! several outputs might return a `Box<dyn Subscribe<C> + Send + Sync + 'static>`:
//! ```
//! use tracing_subscriber::{
//!     Subscribe,
//!     registry::LookupSpan,
//!     prelude::*,
//! };
//! use std::{path::PathBuf, fs::File, io};
//!
//! /// Configures whether logs are emitted to a file, to stdout, or to stderr.
//! pub enum LogConfig {
//!     File(PathBuf),
//!     Stdout,
//!     Stderr,
//! }
//!
//! impl LogConfig {
//!     pub fn subscriber<C>(self) -> Box<dyn Subscribe<C> + Send + Sync + 'static>
//!     where
//!         C: tracing_core::Collect,
//!         for<'a> C: LookupSpan<'a>,
//!     {
//!         // Shared configuration regardless of where logs are output to.
//!         let fmt = tracing_subscriber::fmt::subscriber()
//!             .with_target(true)
//!             .with_thread_names(true);
//!
//!         // Configure the writer based on the desired log target:
//!         match self {
//!             LogConfig::File(path) => {
//!                 let file = File::create(path).expect("failed to create log file");
//!                 Box::new(fmt.with_writer(file))
//!             },
//!             LogConfig::Stdout => Box::new(fmt.with_writer(io::stdout)),
//!             LogConfig::Stderr => Box::new(fmt.with_writer(io::stderr)),
//!         }
//!     }
//! }
//!
//! let config = LogConfig::Stdout;
//! tracing_subscriber::registry()
//!     .with(config.subscriber())
//!     .init();
//! ```
//!
//! The [`Subscribe::boxed`] method is provided to make boxing a subscriber
//! more convenient, but [`Box::new`] may be used as well.
//!
//! When the number of subscribers varies at runtime, note that a
//! [`Vec<S> where S: Subscribe` also implements `Subscribe`][vec-impl]. This
//! can be used to add a variable number of subscribers to a collector:
//!
//! ```
//! use tracing_subscriber::{Subscribe, prelude::*};
//! struct MySubscriber {
//!     // ...
//! }
//! # impl MySubscriber { fn new() -> Self { Self {} }}
//!
//! impl<C: tracing_core::Collect> Subscribe<C> for MySubscriber {
//!     // ...
//! }
//!
//! /// Returns how many subscribers we need
//! fn how_many_subscribers() -> usize {
//!     // ...
//!     # 3
//! }
//!
//! // Create a variable-length `Vec` of subscribers
//! let mut subscribers = Vec::new();
//! for _ in 0..how_many_subscribers() {
//!     subscribers.push(MySubscriber::new());
//! }
//!
//! tracing_subscriber::registry()
//!     .with(subscribers)
//!     .init();
//! ```
//!
//! If a variable number of subscribers is needed and those subscribers have
//! different types, a `Vec` of [boxed subscriber trait objects][box-impl] may
//! be used. For example:
//!
//! ```
//! use tracing_subscriber::{filter::LevelFilter, Subscribe, prelude::*};
//! use std::fs::File;
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! struct Config {
//!     enable_log_file: bool,
//!     enable_stdout: bool,
//!     enable_stderr: bool,
//!     // ...
//! }
//! # impl Config {
//! #    fn from_config_file()-> Result<Self, Box<dyn std::error::Error>> {
//! #         // don't enable the log file so that the example doesn't actually create it
//! #         Ok(Self { enable_log_file: false, enable_stdout: true, enable_stderr: true })
//! #    }
//! # }
//!
//! let cfg = Config::from_config_file()?;
//!
//! // Based on our dynamically loaded config file, create any number of subscribers:
//! let mut subscribers = Vec::new();
//!
//! if cfg.enable_log_file {
//!     let file = File::create("myapp.log")?;
//!     let subscriber = tracing_subscriber::fmt::subscriber()
//!         .with_thread_names(true)
//!         .with_target(true)
//!         .json()
//!         .with_writer(file)
//!         // Box the subscriber as a type-erased trait object, so that it can
//!         // be pushed to the `Vec`.
//!         .boxed();
//!     subscribers.push(subscriber);
//! }
//!
//! if cfg.enable_stdout {
//!     let subscriber = tracing_subscriber::fmt::subscriber()
//!         .pretty()
//!         .with_filter(LevelFilter::INFO)
//!         // Box the subscriber as a type-erased trait object, so that it can
//!         // be pushed to the `Vec`.
//!         .boxed();
//!     subscribers.push(subscriber);
//! }
//!
//! if cfg.enable_stdout {
//!     let subscriber = tracing_subscriber::fmt::subscriber()
//!         .with_target(false)
//!         .with_filter(LevelFilter::WARN)
//!         // Box the subscriber as a type-erased trait object, so that it can
//!         // be pushed to the `Vec`.
//!         .boxed();
//!     subscribers.push(subscriber);
//! }
//!
//! tracing_subscriber::registry()
//!     .with(subscribers)
//!     .init();
//!# Ok(()) }
//! ```
//!
//! Finally, if the number of subscribers _changes_ at runtime, a `Vec` of
//! subscribers can be used alongside the [`reload`](crate::reload) module to
//! add or remove subscribers dynamically at runtime.
//!
//! [prelude]: crate::prelude
//! [option-impl]: crate::subscribe::Subscribe#impl-Subscribe<C>-for-Option<S>
//! [box-impl]: Subscribe#impl-Subscribe%3CC%3E-for-Box%3Cdyn%20Subscribe%3CC%3E%20+%20Send%20+%20Sync%20+%20%27static%3E
//! [vec-impl]: Subscribe#impl-Subscribe<C>-for-Vec<S>
//!
//! # Recording Traces
//!
//! The [`Subscribe`] trait defines a set of methods for consuming notifications from
//! tracing instrumentation, which are generally equivalent to the similarly
//! named methods on [`Collect`]. Unlike [`Collect`], the methods on
//! `Subscribe` are additionally passed a [`Context`] type, which exposes additional
//! information provided by the wrapped subscriber (such as [the current span])
//! to the subscriber.
//!
//! # Filtering with `Subscriber`s
//!
//! As well as strategies for handling trace events, the `Subscribe` trait may also
//! be used to represent composable _filters_. This allows the determination of
//! what spans and events should be recorded to be decoupled from _how_ they are
//! recorded: a filtering subscriber can be applied to other subscribers or
//! subscribers. `Subscribe`s can be used to implement _global filtering_, where a
//! `Subscribe` provides a filtering strategy for the entire subscriber.
//! Additionally, individual recording `Subscribe`s or sets of `Subscribe`s may be
//! combined with _per-subscriber filters_ that control what spans and events are
//! recorded by those subscribers.
//!
//! ## Global Filtering
//!
//! A `Subscribe` that implements a filtering strategy should override the
//! [`register_callsite`] and/or [`enabled`] methods. It may also choose to implement
//! methods such as [`on_enter`], if it wishes to filter trace events based on
//! the current span context.
//!
//! Note that the [`Subscribe::register_callsite`] and [`Subscribe::enabled`] methods
//! determine whether a span or event is enabled *globally*. Thus, they should
//! **not** be used to indicate whether an individual subscriber wishes to record a
//! particular span or event. Instead, if a subscriber is only interested in a subset
//! of trace data, but does *not* wish to disable other spans and events for the
//! rest of the subscriber stack should ignore those spans and events in its
//! notification methods.
//!
//! The filtering methods on a stack of `Subscribe`s are evaluated in a top-down
//! order, starting with the outermost `Subscribe` and ending with the wrapped
//! [`Collect`]. If any subscriber returns `false` from its [`enabled`] method, or
//! [`Interest::never()`] from its [`register_callsite`] method, filter
//! evaluation will short-circuit and the span or event will be disabled.
//!
//! ### Enabling Interest
//!
//! Whenever an tracing event (or span) is emitted, it goes through a number of
//! steps to determine how and how much it should be processed. The earlier an
//! event is disabled, the less work has to be done to process the event, so
//! subscribers that implement filtering should attempt to disable unwanted
//! events as early as possible. In order, each event checks:
//!
//! - [`register_callsite`], once per callsite (roughly: once per time that
//!   `event!` or `span!` is written in the source code; this is cached at the
//!   callsite). See [`Collect::register_callsite`] and
//!   [`tracing_core::callsite`] for a summary of how this behaves.
//! - [`enabled`], once per emitted event (roughly: once per time that `event!`
//!   or `span!` is *executed*), and only if `register_callsite` registers an
//!   [`Interest::sometimes`]. This is the main customization point to globally
//!   filter events based on their [`Metadata`]. If an event can be disabled
//!   based only on [`Metadata`], it should be, as this allows the construction
//!   of the actual `Event`/`Span` to be skipped.
//! - For events only (and not spans), [`event_enabled`] is called just before
//!   processing the event. This gives subscribers one last chance to say that
//!   an event should be filtered out, now that the event's fields are known.
//!
//! ## Per-Subscriber Filtering
//!
//! **Note**: per-subscriber filtering APIs currently require the [`"registry"` crate
//! feature flag][feat] to be enabled.
//!
//! Sometimes, it may be desirable for one `Subscribe` to record a particular subset
//! of spans and events, while a different subset of spans and events are
//! recorded by other `Subscribe`s. For example:
//!
//! - A subscriber that records metrics may wish to observe only events including
//!   particular tracked values, while a logging subscriber ignores those events.
//! - If recording a distributed trace is expensive, it might be desirable to
//!   only send spans with `INFO` and lower verbosity to the distributed tracing
//!   system, while logging more verbose spans to a file.
//! - Spans and events with a particular target might be recorded differently
//!   from others, such as by generating an HTTP access log from a span that
//!   tracks the lifetime of an HTTP request.
//!
//! The [`Filter`] trait is used to control what spans and events are
//! observed by an individual `Subscribe`, while still allowing other `Subscribe`s to
//! potentially record them. The [`Subscribe::with_filter`] method combines a
//! `Subscribe` with a [`Filter`], returning a [`Filtered`] subscriber.
//!
//! This crate's [`filter`] module provides a number of types which implement
//! the [`Filter`] trait, such as [`LevelFilter`], [`Targets`], and
//! [`FilterFn`]. These [`Filter`]s provide ready-made implementations of common
//! forms of filtering. For custom filtering policies, the [`FilterFn`] and
//! [`DynFilterFn`] types allow implementing a [`Filter`] with a closure or
//! function pointer. In addition, when more control is required, the [`Filter`]
//! trait may also be implemented for user-defined types.
//!
//! [`Option<Filter>`] also implements [`Filter`], which allows for an optional
//! filter. [`None`](Option::None) filters out _nothing_ (that is, allows
//! everything through). For example:
//!
//! ```rust
//! # use tracing_subscriber::{filter::filter_fn, Subscribe};
//! # use tracing_core::{Metadata, collect::Collect};
//! # struct MySubscriber<C>(std::marker::PhantomData<C>);
//! # impl<C> MySubscriber<C> { fn new() -> Self { Self(std::marker::PhantomData)} }
//! # impl<C: Collect> Subscribe<C> for MySubscriber<C> {}
//! # fn my_filter(_: &str) -> impl Fn(&Metadata) -> bool { |_| true  }
//! fn setup_tracing<C: Collect>(filter_config: Option<&str>) {
//!     let layer = MySubscriber::<C>::new()
//!         .with_filter(filter_config.map(|config| filter_fn(my_filter(config))));
//! //...
//! }
//! ```
//!
//! <div class="example-wrap" style="display:inline-block">
//! <pre class="compile_fail" style="white-space:normal;font:inherit;">
//!     <strong>Warning</strong>: Currently, the <a href="../struct.Registry.html">
//!     <code>Registry</code></a> type defined in this crate is the only root
//!     <code>Collect</code> capable of supporting subscriberss with
//!     per-subscriber filters. In the future, new APIs will be added to allow other
//!     root <code>Collect</code>s to support per-subscriber filters.
//! </pre></div>
//!
//! For example, to generate an HTTP access log based on spans with
//! the `http_access` target, while logging other spans and events to
//! standard out, a [`Filter`] can be added to the access log subscriber:
//!
//! ```
//! use tracing_subscriber::{filter, prelude::*};
//!
//! // Generates an HTTP access log.
//! let access_log = // ...
//!     # filter::LevelFilter::INFO;
//!
//! // Add a filter to the access log subscriber so that it only observes
//! // spans and events with the `http_access` target.
//! let access_log = access_log.with_filter(filter::filter_fn(|metadata| {
//!     // Returns `true` if and only if the span or event's target is
//!     // "http_access".
//!     metadata.target() == "http_access"
//! }));
//!
//! // A general-purpose logging subscriber.
//! let fmt_subscriber = tracing_subscriber::fmt::subscriber();
//!
//! // Build a subscriber that combines the access log and stdout log
//! // subscribers.
//! tracing_subscriber::registry()
//!     .with(fmt_subscriber)
//!     .with(access_log)
//!     .init();
//! ```
//!
//! Multiple subscribers can have their own, separate per-subscriber filters. A span or
//! event will be recorded if it is enabled by _any_ per-subscriber filter, but it
//! will be skipped by the subscribers whose filters did not enable it. Building on
//! the previous example:
//!
//! ```
//! use tracing_subscriber::{filter::{filter_fn, LevelFilter}, prelude::*};
//!
//! let access_log = // ...
//!     # LevelFilter::INFO;
//! let fmt_subscriber = tracing_subscriber::fmt::subscriber();
//!
//! tracing_subscriber::registry()
//!     // Add the filter for the "http_access" target to the access
//!     // log subscriber, like before.
//!     .with(access_log.with_filter(filter_fn(|metadata| {
//!         metadata.target() == "http_access"
//!     })))
//!     // Add a filter for spans and events with the INFO level
//!     // and below to the logging subscriber.
//!     .with(fmt_subscriber.with_filter(LevelFilter::INFO))
//!     .init();
//!
//! // Neither subscriber will observe this event
//! tracing::debug!(does_anyone_care = false, "a tree fell in the forest");
//!
//! // This event will be observed by the logging subscriber, but not
//! // by the access log subscriber.
//! tracing::warn!(dose_roentgen = %3.8, "not great, but not terrible");
//!
//! // This event will be observed only by the access log subscriber.
//! tracing::trace!(target: "http_access", "HTTP request started");
//!
//! // Both subscribers will observe this event.
//! tracing::error!(target: "http_access", "HTTP request failed with a very bad error!");
//! ```
//!
//! A per-subscriber filter can be applied to multiple [`Subscribe`]s at a time, by
//! combining them into a [`Layered`] subscriber using [`Subscribe::and_then`], and then
//! calling [`Subscribe::with_filter`] on the resulting [`Layered`] subscriber.
//!
//! Consider the following:
//! - `subscriber_a` and `subscriber_b`, which should only receive spans and events at
//!    the [`INFO`] [level] and above.
//! - A third subscriber, `subscriber_c`, which should receive spans and events at
//!    the [`DEBUG`] [level] as well.
//! The subscribers and filters would be composed thusly:
//!
//! ```
//! use tracing_subscriber::{filter::LevelFilter, prelude::*};
//!
//! let subscriber_a = // ...
//! # LevelFilter::INFO;
//! let subscriber_b =  // ...
//! # LevelFilter::INFO;
//! let subscriber_c =  // ...
//! # LevelFilter::INFO;
//!
//! let info_subscribers = subscriber_a
//!     // Combine `subscriber_a` and `subscriber_b` into a `Layered` subscriber:
//!     .and_then(subscriber_b)
//!     // ...and then add an `INFO` `LevelFilter` to that subscriber:
//!     .with_filter(LevelFilter::INFO);
//!
//! tracing_subscriber::registry()
//!     // Add `subscriber_c` with a `DEBUG` filter.
//!     .with(subscriber_c.with_filter(LevelFilter::DEBUG))
//!     .with(info_subscribers)
//!     .init();
//!```
//!
//! If a [`Filtered`] [`Subscribe`] is combined with another [`Subscribe`]
//! [`Subscribe::and_then`], and a filter is added to the [`Layered`] subscriber, that
//! subscriber will be filtered by *both* the inner filter and the outer filter.
//! Only spans and events that are enabled by *both* filters will be
//! observed by that subscriber. This can be used to implement complex filtering
//! trees.
//!
//! As an example, consider the following constraints:
//! - Suppose that a particular [target] is used to indicate events that
//!   should be counted as part of a metrics system, which should be only
//!   observed by a subscriber that collects metrics.
//! - A log of high-priority events ([`INFO`] and above) should be logged
//!   to stdout, while more verbose events should be logged to a debugging log file.
//! - Metrics-focused events should *not* be included in either log output.
//!
//! In that case, it is possible to apply a filter to both logging subscribers to
//! exclude the metrics events, while additionally adding a [`LevelFilter`]
//! to the stdout log:
//!
//! ```
//! # // wrap this in a function so we don't actually create `debug.log` when
//! # // running the doctests..
//! # fn docs() -> Result<(), Box<dyn std::error::Error + 'static>> {
//! use tracing_subscriber::{filter, prelude::*};
//! use std::{fs::File, sync::Arc};
//!
//! // A subscriber that logs events to stdout using the human-readable "pretty"
//! // format.
//! let stdout_log = tracing_subscriber::fmt::subscriber()
//!     .pretty();
//!
//! // A subscriber that logs events to a file.
//! let file = File::create("debug.log")?;
//! let debug_log = tracing_subscriber::fmt::subscriber()
//!     .with_writer(file);
//!
//! // A subscriber that collects metrics using specific events.
//! let metrics_subscriber = /* ... */ filter::LevelFilter::INFO;
//!
//! tracing_subscriber::registry()
//!     .with(
//!         stdout_log
//!             // Add an `INFO` filter to the stdout logging subscriber
//!             .with_filter(filter::LevelFilter::INFO)
//!             // Combine the filtered `stdout_log` subscriber with the
//!             // `debug_log` subscriber, producing a new `Layered` subscriber.
//!             .and_then(debug_log)
//!             // Add a filter to *both* subscribers that rejects spans and
//!             // events whose targets start with `metrics`.
//!             .with_filter(filter::filter_fn(|metadata| {
//!                 !metadata.target().starts_with("metrics")
//!             }))
//!     )
//!     .with(
//!         // Add a filter to the metrics label that *only* enables
//!         // events whose targets start with `metrics`.
//!         metrics_subscriber.with_filter(filter::filter_fn(|metadata| {
//!             metadata.target().starts_with("metrics")
//!         }))
//!     )
//!     .init();
//!
//! // This event will *only* be recorded by the metrics subscriber.
//! tracing::info!(target: "metrics::cool_stuff_count", value = 42);
//!
//! // This event will only be seen by the debug log file subscriber:
//! tracing::debug!("this is a message, and part of a system of messages");
//!
//! // This event will be seen by both the stdout log subscriber *and*
//! // the debug log file subscriber, but not by the metrics subscriber.
//! tracing::warn!("the message is a warning about danger!");
//! # Ok(()) }
//! ```
//!
//! [subscriber]: Subscribe
//! [`Collect`]:tracing_core::Collect
//! [collector]: tracing_core::Collect
//! [span IDs]: https://docs.rs/tracing-core/latest/tracing_core/span/struct.Id.html
//! [the current span]: Context::current_span
//! [`register_callsite`]: Subscribe::register_callsite
//! [`enabled`]: Subscribe::enabled
//! [`event_enabled`]: Subscribe::event_enabled
//! [`on_enter`]: Subscribe::on_enter
//! [`Subscribe::register_callsite`]: Subscribe::register_callsite
//! [`Subscribe::enabled`]: Subscribe::enabled
//! [`Interest::never()`]: tracing_core::collect::Interest::never
//! [`Filtered`]: crate::filter::Filtered
//! [`filter`]: crate::filter
//! [`Targets`]: crate::filter::Targets
//! [`FilterFn`]: crate::filter::FilterFn
//! [`DynFilterFn`]: crate::filter::DynFilterFn
//! [level]: tracing_core::Level
//! [`INFO`]: tracing_core::Level::INFO
//! [`DEBUG`]: tracing_core::Level::DEBUG
//! [target]: tracing_core::Metadata::target
//! [`LevelFilter`]: crate::filter::LevelFilter
//! [feat]: crate#feature-flags
use crate::filter;

use tracing_core::{
    collect::{Collect, Interest},
    metadata::Metadata,
    span, Dispatch, Event, LevelFilter,
};

use core::{any::TypeId, ptr::NonNull};

feature! {
    #![feature = "alloc"]
    use alloc::boxed::Box;
    use core::ops::{Deref, DerefMut};
}

mod context;
mod layered;
pub use self::{context::*, layered::*};

// The `tests` module is `pub(crate)` because it contains test utilities used by
// other modules.
#[cfg(test)]
pub(crate) mod tests;

/// A composable handler for `tracing` events.
///
/// A type that implements `Subscribe` &mdash a "subscriber" &mdash; provides a
/// particular behavior for recording or collecting traces that can
/// be composed together with other subscribers to build a [collector]. See the
/// [module-level documentation](crate::subscribe) for details.
///
/// [collector]: tracing_core::Collect
#[cfg_attr(docsrs, doc(notable_trait))]
pub trait Subscribe<C>
where
    C: Collect,
    Self: 'static,
{
    /// Performs late initialization when installing this subscriber as a
    /// [collector].
    ///
    /// ## Avoiding Memory Leaks
    ///
    /// Subscribers should not store the [`Dispatch`] pointing to the collector
    /// that they are a part of. Because the `Dispatch` owns the collector,
    /// storing the `Dispatch` within the collector will create a reference
    /// count cycle, preventing the `Dispatch` from ever being dropped.
    ///
    /// Instead, when it is necessary to store a cyclical reference to the
    /// `Dispatch` within a subscriber, use [`Dispatch::downgrade`] to convert a
    /// `Dispatch` into a [`WeakDispatch`]. This type is analogous to
    /// [`std::sync::Weak`], and does not create a reference count cycle. A
    /// [`WeakDispatch`] can be stored within a subscriber without causing a
    /// memory leak, and can be [upgraded] into a `Dispatch` temporarily when
    /// the `Dispatch` must be accessed by the subscriber.
    ///
    /// [`WeakDispatch`]: tracing_core::dispatch::WeakDispatch
    /// [upgraded]: tracing_core::dispatch::WeakDispatch::upgrade
    /// [collector]: tracing_core::Collect
    fn on_register_dispatch(&self, collector: &Dispatch) {
        let _ = collector;
    }

    /// Performs late initialization when attaching a subscriber to a
    /// [collector].
    ///
    /// This is a callback that is called when the `Subscribe` is added to a
    /// [`Collect`] (e.g. in [`Subscribe::with_collector`] and
    /// [`CollectExt::with`]). Since this can only occur before the
    /// [`Collect`] has been set as the default, both the subscriber and
    /// [`Collect`] are passed to this method _mutably_. This gives the
    /// subscribe the opportunity to set any of its own fields with values
    /// received by method calls on the [`Collect`].
    ///
    /// For example, [`Filtered`] subscribers implement `on_subscribe` to call the
    /// [`Collect`]'s [`register_filter`] method, and store the returned
    /// [`FilterId`] as a field.
    ///
    /// **Note** In most cases, subscriber implementations will not need to
    /// implement this method. However, in cases where a type implementing
    /// subscriber wraps one or more other types that implement `Subscribe`, like the
    /// [`Layered`] and [`Filtered`] types in this crate, that type MUST ensure
    /// that the inner `Subscribe` instance's' `on_subscribe` methods are
    /// called. Otherwise, unctionality that relies on `on_subscribe`, such as
    /// [per-subscriber filtering], may not work correctly.
    ///
    /// [`Filtered`]: crate::filter::Filtered
    /// [`register_filter`]: crate::registry::LookupSpan::register_filter
    /// [per-subscribe filtering]: #per-subscriber-filtering
    /// [`FilterId`]: crate::filter::FilterId
    /// [collector]: tracing_core::Collect
    fn on_subscribe(&mut self, collector: &mut C) {
        let _ = collector;
    }

    /// Registers a new callsite with this subscriber, returning whether or not
    /// the subscriber is interested in being notified about the callsite, similarly
    /// to [`Collect::register_callsite`].
    ///
    /// By default, this returns [`Interest::always()`] if [`self.enabled`] returns
    /// true, or [`Interest::never()`] if it returns false.
    ///
    /// <div class="example-wrap" style="display:inline-block">
    /// <pre class="ignore" style="white-space:normal;font:inherit;">
    ///
    /// **Note**: This method (and [`Subscribe::enabled`]) determine whether a span or event is
    /// globally enabled, *not* whether the individual subscriber will be notified about that
    /// span or event.  This is intended to be used by subscribers that implement filtering for
    /// the entire stack. Subscribers which do not wish to be notified about certain spans or
    /// events but do not wish to globally disable them should ignore those spans or events in
    /// their [on_event][Self::on_event], [on_enter][Self::on_enter], [on_exit][Self::on_exit],
    /// and other notification methods.
    ///
    /// </pre></div>
    ///
    /// See [the trait-level documentation] for more information on filtering
    /// with subscribers.
    ///
    /// Subscribers may also implement this method to perform any behaviour that
    /// should be run once per callsite. If the subscriber wishes to use
    /// `register_callsite` for per-callsite behaviour, but does not want to
    /// globally enable or disable those callsites, it should always return
    /// [`Interest::always()`].
    ///
    /// [`Interest`]: tracing_core::collect::Interest
    /// [`Collect::register_callsite`]: tracing_core::Collect::register_callsite()
    /// [`self.enabled`]: Subscribe::enabled()
    /// [the trait-level documentation]: #filtering-with-subscribers
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        if self.enabled(metadata, Context::none()) {
            Interest::always()
        } else {
            Interest::never()
        }
    }

    /// Returns `true` if this subscriber is interested in a span or event with the
    /// given `metadata` in the current [`Context`], similarly to
    /// [`Collect::enabled`].
    ///
    /// By default, this always returns `true`, allowing the wrapped collector
    /// to choose to disable the span.
    ///
    /// <div class="example-wrap" style="display:inline-block">
    /// <pre class="ignore" style="white-space:normal;font:inherit;">
    ///
    /// **Note**: This method (and [`register_callsite`][Self::register_callsite])
    /// determine whether a span or event is
    /// globally enabled, *not* whether the individual subscriber will be
    /// notified about that span or event. This is intended to be used
    /// by subscribers that implement filtering for the entire stack. Layers which do
    /// not wish to be notified about certain spans or events but do not wish to
    /// globally disable them should ignore those spans or events in their
    /// [on_event][Self::on_event], [on_enter][Self::on_enter], [on_exit][Self::on_exit],
    /// and other notification methods.
    ///
    /// </pre></div>
    ///
    ///
    /// See [the trait-level documentation] for more information on filtering
    /// with subscribers.
    ///
    /// [`Interest`]: tracing_core::Interest
    /// [the trait-level documentation]: #filtering-with-subscribers
    fn enabled(&self, metadata: &Metadata<'_>, ctx: Context<'_, C>) -> bool {
        let _ = (metadata, ctx);
        true
    }

    /// Notifies this subscriber that a new span was constructed with the given
    /// `Attributes` and `Id`.
    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, C>) {
        let _ = (attrs, id, ctx);
    }

    // TODO(eliza): do we want this to be a public API? If we end up moving
    // filtering subscribers to a separate trait, we may no longer want subscribers to
    // be able to participate in max level hinting...
    #[doc(hidden)]
    fn max_level_hint(&self) -> Option<LevelFilter> {
        None
    }

    /// Notifies this subscriber that a span with the given `Id` recorded the given
    /// `values`.
    // Note: it's unclear to me why we'd need the current span in `record` (the
    // only thing the `Context` type currently provides), but passing it in anyway
    // seems like a good future-proofing measure as it may grow other methods later...
    fn on_record(&self, _span: &span::Id, _values: &span::Record<'_>, _ctx: Context<'_, C>) {}

    /// Notifies this subscriber that a span with the ID `span` recorded that it
    /// follows from the span with the ID `follows`.
    // Note: it's unclear to me why we'd need the current span in `record` (the
    // only thing the `Context` type currently provides), but passing it in anyway
    // seems like a good future-proofing measure as it may grow other methods later...
    fn on_follows_from(&self, _span: &span::Id, _follows: &span::Id, _ctx: Context<'_, C>) {}

    /// Called before [`on_event`], to determine if `on_event` should be called.
    ///
    /// <div class="example-wrap" style="display:inline-block">
    /// <pre class="ignore" style="white-space:normal;font:inherit;">
    ///
    /// **Note**: This method determines whether an event is globally enabled,
    /// *not* whether the individual subscriber will be notified about the
    /// event. This is intended to be used by subscribers that implement
    /// filtering for the entire stack. Subscribers which do not wish to be
    /// notified about certain events but do not wish to globally disable them
    /// should ignore those events in their [on_event][Self::on_event].
    ///
    /// </pre></div>
    ///
    /// See [the trait-level documentation] for more information on filtering
    /// with `Subscriber`s.
    ///
    /// [`on_event`]: Self::on_event
    /// [`Interest`]: tracing_core::Interest
    /// [the trait-level documentation]: #filtering-with-subscribers
    #[inline] // collapse this to a constant please mrs optimizer
    fn event_enabled(&self, _event: &Event<'_>, _ctx: Context<'_, C>) -> bool {
        true
    }

    /// Notifies this subscriber that an event has occurred.
    fn on_event(&self, _event: &Event<'_>, _ctx: Context<'_, C>) {}

    /// Notifies this subscriber that a span with the given ID was entered.
    fn on_enter(&self, _id: &span::Id, _ctx: Context<'_, C>) {}

    /// Notifies this subscriber that the span with the given ID was exited.
    fn on_exit(&self, _id: &span::Id, _ctx: Context<'_, C>) {}

    /// Notifies this subscriber that the span with the given ID has been closed.
    fn on_close(&self, _id: span::Id, _ctx: Context<'_, C>) {}

    /// Notifies this subscriber that a span ID has been cloned, and that the
    /// subscriber returned a different ID.
    fn on_id_change(&self, _old: &span::Id, _new: &span::Id, _ctx: Context<'_, C>) {}

    /// Composes this subscriber around the given collector, returning a `Layered`
    /// struct implementing `Subscribe`.
    ///
    /// The returned subscriber will call the methods on this subscriber and then
    /// those of the new subscriber, before calling the methods on the collector
    /// it wraps. For example:
    ///
    /// ```rust
    /// # use tracing_subscriber::subscribe::Subscribe;
    /// # use tracing_core::Collect;
    /// # use tracing_core::span::Current;
    /// pub struct FooSubscriber {
    ///     // ...
    /// }
    ///
    /// pub struct BarSubscriber {
    ///     // ...
    /// }
    ///
    /// pub struct MyCollector {
    ///     // ...
    /// }
    ///
    /// impl<C: Collect> Subscribe<C> for FooSubscriber {
    ///     // ...
    /// }
    ///
    /// impl<C: Collect> Subscribe<C> for BarSubscriber {
    ///     // ...
    /// }
    ///
    /// # impl FooSubscriber {
    /// # fn new() -> Self { Self {} }
    /// # }
    /// # impl BarSubscriber {
    /// # fn new() -> Self { Self { }}
    /// # }
    /// # impl MyCollector {
    /// # fn new() -> Self { Self { }}
    /// # }
    /// # use tracing_core::{span::{Id, Attributes, Record}, Metadata, Event};
    /// # impl tracing_core::Collect for MyCollector {
    /// #   fn new_span(&self, _: &Attributes) -> Id { Id::from_u64(1) }
    /// #   fn record(&self, _: &Id, _: &Record) {}
    /// #   fn event(&self, _: &Event) {}
    /// #   fn record_follows_from(&self, _: &Id, _: &Id) {}
    /// #   fn enabled(&self, _: &Metadata) -> bool { false }
    /// #   fn enter(&self, _: &Id) {}
    /// #   fn exit(&self, _: &Id) {}
    /// #   fn current_span(&self) -> Current { Current::unknown() }
    /// # }
    /// let collector = FooSubscriber::new()
    ///     .and_then(BarSubscriber::new())
    ///     .with_collector(MyCollector::new());
    /// ```
    ///
    /// Multiple subscribers may be composed in this manner:
    ///
    /// ```rust
    /// # use tracing_subscriber::subscribe::Subscribe;
    /// # use tracing_core::{Collect, span::Current};
    /// # pub struct FooSubscriber {}
    /// # pub struct BarSubscriber {}
    /// # pub struct MyCollector {}
    /// # impl<C: Collect> Subscribe<C> for FooSubscriber {}
    /// # impl<C: Collect> Subscribe<C> for BarSubscriber {}
    /// # impl FooSubscriber {
    /// # fn new() -> Self { Self {} }
    /// # }
    /// # impl BarSubscriber {
    /// # fn new() -> Self { Self { }}
    /// # }
    /// # impl MyCollector {
    /// # fn new() -> Self { Self { }}
    /// # }
    /// # use tracing_core::{span::{Id, Attributes, Record}, Metadata, Event};
    /// # impl tracing_core::Collect for MyCollector {
    /// #   fn new_span(&self, _: &Attributes) -> Id { Id::from_u64(1) }
    /// #   fn record(&self, _: &Id, _: &Record) {}
    /// #   fn event(&self, _: &Event) {}
    /// #   fn record_follows_from(&self, _: &Id, _: &Id) {}
    /// #   fn enabled(&self, _: &Metadata) -> bool { false }
    /// #   fn enter(&self, _: &Id) {}
    /// #   fn exit(&self, _: &Id) {}
    /// #   fn current_span(&self) -> Current { Current::unknown() }
    /// # }
    /// pub struct BazSubscriber {
    ///     // ...
    /// }
    ///
    /// impl<C: Collect> Subscribe<C> for BazSubscriber {
    ///     // ...
    /// }
    /// # impl BazSubscriber { fn new() -> Self { BazSubscriber {} } }
    ///
    /// let collector = FooSubscriber::new()
    ///     .and_then(BarSubscriber::new())
    ///     .and_then(BazSubscriber::new())
    ///     .with_collector(MyCollector::new());
    /// ```
    fn and_then<S>(self, subscriber: S) -> Layered<S, Self, C>
    where
        S: Subscribe<C>,
        Self: Sized,
    {
        let inner_has_subscriber_filter = filter::subscriber_has_psf(&self);
        Layered::new(subscriber, self, inner_has_subscriber_filter)
    }

    /// Composes this subscriber with the given collector, returning a
    /// `Layered` struct that implements [`Collect`].
    ///
    /// The returned `Layered` subscriber will call the methods on this subscriber
    /// and then those of the wrapped collector.
    ///
    /// For example:
    /// ```rust
    /// # use tracing_subscriber::subscribe::Subscribe;
    /// # use tracing_core::Collect;
    /// # use tracing_core::span::Current;
    /// pub struct FooSubscriber {
    ///     // ...
    /// }
    ///
    /// pub struct MyCollector {
    ///     // ...
    /// }
    ///
    /// impl<C: Collect> Subscribe<C> for FooSubscriber {
    ///     // ...
    /// }
    ///
    /// # impl FooSubscriber {
    /// # fn new() -> Self { Self {} }
    /// # }
    /// # impl MyCollector {
    /// # fn new() -> Self { Self { }}
    /// # }
    /// # use tracing_core::{span::{Id, Attributes, Record}, Metadata};
    /// # impl tracing_core::Collect for MyCollector {
    /// #   fn new_span(&self, _: &Attributes) -> Id { Id::from_u64(0) }
    /// #   fn record(&self, _: &Id, _: &Record) {}
    /// #   fn event(&self, _: &tracing_core::Event) {}
    /// #   fn record_follows_from(&self, _: &Id, _: &Id) {}
    /// #   fn enabled(&self, _: &Metadata) -> bool { false }
    /// #   fn enter(&self, _: &Id) {}
    /// #   fn exit(&self, _: &Id) {}
    /// #   fn current_span(&self) -> Current { Current::unknown() }
    /// # }
    /// let collector = FooSubscriber::new()
    ///     .with_collector(MyCollector::new());
    ///```
    ///
    /// [`Collect`]: tracing_core::Collect
    fn with_collector(mut self, mut inner: C) -> Layered<Self, C>
    where
        Self: Sized,
    {
        let inner_has_subscriber_filter = filter::collector_has_psf(&inner);
        self.on_subscribe(&mut inner);
        Layered::new(self, inner, inner_has_subscriber_filter)
    }

    /// Combines `self` with a [`Filter`], returning a [`Filtered`] subscriber.
    ///
    /// The [`Filter`] will control which spans and events are enabled for
    /// this subscriber. See [the trait-level documentation][psf] for details on
    /// per-subscriber filtering.
    ///
    /// [`Filtered`]: crate::filter::Filtered
    /// [psf]: #per-subscriber-filtering
    #[cfg(all(feature = "registry", feature = "std"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "registry", feature = "std"))))]
    fn with_filter<F>(self, filter: F) -> filter::Filtered<Self, F, C>
    where
        Self: Sized,
        F: Filter<C>,
    {
        filter::Filtered::new(self, filter)
    }

    /// Erases the type of this subscriber, returning a [`Box`]ed `dyn
    /// Subscribe` trait object.
    ///
    /// This can be used when a function returns a subscriber which may be of
    /// one of several types, or when a composed subscriber has a very long type
    /// signature.
    ///
    /// # Examples
    ///
    /// The following example will *not* compile, because the value assigned to
    /// `log_subscriber` may have one of several different types:
    ///
    /// ```compile_fail
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use tracing_subscriber::{Subscribe, filter::LevelFilter, prelude::*};
    /// use std::{path::PathBuf, fs::File, io};
    ///
    /// /// Configures whether logs are emitted to a file, to stdout, or to stderr.
    /// pub enum LogConfig {
    ///     File(PathBuf),
    ///     Stdout,
    ///     Stderr,
    /// }
    ///
    /// let config = // ...
    ///     # LogConfig::Stdout;
    ///
    /// // Depending on the config, construct a subscriber of one of several types.
    /// let log_subscriber = match config {
    ///     // If logging to a file, use a maximally-verbose configuration.
    ///     LogConfig::File(path) => {
    ///         let file = File::create(path)?;
    ///         tracing_subscriber::fmt::subscriber()
    ///             .with_thread_ids(true)
    ///             .with_thread_names(true)
    ///             // Selecting the JSON logging format changes the subscriber's
    ///             // type.
    ///             .json()
    ///             .with_span_list(true)
    ///             // Setting the writer to use our log file changes the
    ///             // subscriber's type again.
    ///             .with_writer(file)
    ///     },
    ///
    ///     // If logging to stdout, use a pretty, human-readable configuration.
    ///     LogConfig::Stdout => tracing_subscriber::fmt::subscriber()
    ///         // Selecting the "pretty" logging format changes the
    ///         // subscriber's type!
    ///         .pretty()
    ///         .with_writer(io::stdout)
    ///         // Add a filter based on the RUST_LOG environment variable;
    ///         // this changes the type too!
    ///         .and_then(tracing_subscriber::EnvFilter::from_default_env()),
    ///
    ///     // If logging to stdout, only log errors and warnings.
    ///     LogConfig::Stderr => tracing_subscriber::fmt::subscriber()
    ///         // Changing the writer changes the subscriber's type
    ///         .with_writer(io::stderr)
    ///         // Only log the `WARN` and `ERROR` levels. Adding a filter
    ///         // changes the subscriber's type to `Filtered<LevelFilter, ...>`.
    ///         .with_filter(LevelFilter::WARN),
    /// };
    ///
    /// tracing_subscriber::registry()
    ///     .with(log_subscriber)
    ///     .init();
    /// # Ok(()) }
    /// ```
    ///
    /// However, adding a call to `.boxed()` after each match arm erases the
    /// subscriber's type, so this code *does* compile:
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # use tracing_subscriber::{Subscribe, filter::LevelFilter, prelude::*};
    /// # use std::{path::PathBuf, fs::File, io};
    /// # pub enum LogConfig {
    /// #    File(PathBuf),
    /// #    Stdout,
    /// #    Stderr,
    /// # }
    /// # let config = LogConfig::Stdout;
    /// let log_subscriber = match config {
    ///     LogConfig::File(path) => {
    ///         let file = File::create(path)?;
    ///         tracing_subscriber::fmt::subscriber()
    ///             .with_thread_ids(true)
    ///             .with_thread_names(true)
    ///             .json()
    ///             .with_span_list(true)
    ///             .with_writer(file)
    ///             // Erase the type by boxing the subscriber
    ///             .boxed()
    ///     },
    ///
    ///     LogConfig::Stdout => tracing_subscriber::fmt::subscriber()
    ///         .pretty()
    ///         .with_writer(io::stdout)
    ///         .and_then(tracing_subscriber::EnvFilter::from_default_env())
    ///         // Erase the type by boxing the subscriber
    ///         .boxed(),
    ///
    ///     LogConfig::Stderr => tracing_subscriber::fmt::subscriber()
    ///         .with_writer(io::stderr)
    ///         .with_filter(LevelFilter::WARN)
    ///         // Erase the type by boxing the subscriber
    ///         .boxed(),
    /// };
    ///
    /// tracing_subscriber::registry()
    ///     .with(log_subscriber)
    ///     .init();
    /// # Ok(()) }
    /// ```
    #[cfg(any(feature = "alloc", feature = "std"))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "alloc", feature = "std"))))]
    fn boxed(self) -> Box<dyn Subscribe<C> + Send + Sync + 'static>
    where
        Self: Sized,
        Self: Subscribe<C> + Send + Sync + 'static,
        C: Collect,
    {
        Box::new(self)
    }

    #[doc(hidden)]
    unsafe fn downcast_raw(&self, id: TypeId) -> Option<NonNull<()>> {
        if id == TypeId::of::<Self>() {
            Some(NonNull::from(self).cast())
        } else {
            None
        }
    }
}

/// A per-[`Subscribe`] filter that determines whether a span or event is enabled
/// for an individual subscriber.
#[cfg(all(feature = "registry", feature = "std"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "registry", feature = "std"))))]
#[cfg_attr(docsrs, doc(notable_trait))]
pub trait Filter<S> {
    /// Returns `true` if this subscriber is interested in a span or event with the
    /// given [`Metadata`] in the current [`Context`], similarly to
    /// [`Collect::enabled`].
    ///
    /// If this returns `false`, the span or event will be disabled _for the
    /// wrapped [`Subscribe`]_. Unlike [`Subscribe::enabled`], the span or event will
    /// still be recorded if any _other_ subscribers choose to enable it. However,
    /// the subscriber [filtered] by this filter will skip recording that span or
    /// event.
    ///
    /// If all subscribers indicate that they do not wish to see this span or event,
    /// it will be disabled.
    ///
    /// [`metadata`]: tracing_core::Metadata
    /// [`Collect::enabled`]: tracing_core::Collect::enabled
    /// [filtered]: crate::filter::Filtered
    fn enabled(&self, meta: &Metadata<'_>, cx: &Context<'_, S>) -> bool;

    /// Returns an [`Interest`] indicating whether this subscriber will [always],
    /// [sometimes], or [never] be interested in the given [`Metadata`].
    ///
    /// When a given callsite will [always] or [never] be enabled, the results
    /// of evaluating the filter may be cached for improved performance.
    /// Therefore, if a filter is capable of determining that it will always or
    /// never enable a particular callsite, providing an implementation of this
    /// function is recommended.
    ///
    /// <div class="example-wrap" style="display:inline-block">
    /// <pre class="ignore" style="white-space:normal;font:inherit;">
    /// <strong>Note</strong>: If a <code>Filter</code> will perform
    /// <em>dynamic filtering</em> that depends on the current context in which
    /// a span or event was observed (e.g. only enabling an event when it
    /// occurs within a particular span), it <strong>must</strong> return
    /// <code>Interest::sometimes()</code> from this method. If it returns
    /// <code>Interest::always()</code> or <code>Interest::never()</code>, the
    /// <code>enabled</code> method may not be called when a particular instance
    /// of that span or event is recorded.
    /// </pre>
    /// </div>
    ///
    /// This method is broadly similar to [`Collect::register_callsite`];
    /// however, since the returned value represents only the interest of
    /// *this* subscriber, the resulting behavior is somewhat different.
    ///
    /// If a [`Collect`] returns [`Interest::always()`][always] or
    /// [`Interest::never()`][never] for a given [`Metadata`], its [`enabled`]
    /// method is then *guaranteed* to never be called for that callsite. On the
    /// other hand, when a `Filter` returns [`Interest::always()`][always] or
    /// [`Interest::never()`][never] for a callsite, _other_ [`Subscribe`]s may have
    /// differing interests in that callsite. If this is the case, the callsite
    /// will receive [`Interest::sometimes()`][sometimes], and the [`enabled`]
    /// method will still be called for that callsite when it records a span or
    /// event.
    ///
    /// Returning [`Interest::always()`][always] or [`Interest::never()`][never] from
    /// `Filter::callsite_enabled` will permanently enable or disable a
    /// callsite (without requiring subsequent calls to [`enabled`]) if and only
    /// if the following is true:
    ///
    /// - all [`Subscribe`]s that comprise the subscriber include `Filter`s
    ///   (this includes a tree of [`Layered`] subscribers that share the same
    ///   `Filter`)
    /// - all those `Filter`s return the same [`Interest`].
    ///
    /// For example, if a [`Collect`] consists of two [`Filtered`] subscribers,
    /// and both of those subscribers return [`Interest::never()`][never], that
    /// callsite *will* never be enabled, and the [`enabled`] methods of those
    /// [`Filter`]s will not be called.
    ///
    /// ## Default Implementation
    ///
    /// The default implementation of this method assumes that the
    /// `Filter`'s [`enabled`] method _may_ perform dynamic filtering, and
    /// returns [`Interest::sometimes()`][sometimes], to ensure that [`enabled`]
    /// is called to determine whether a particular _instance_ of the callsite
    /// is enabled in the current context. If this is *not* the case, and the
    /// `Filter`'s [`enabled`] method will always return the same result
    /// for a particular [`Metadata`], this method can be overridden as
    /// follows:
    ///
    /// ```
    /// use tracing_subscriber::subscribe;
    /// use tracing_core::{Metadata, collect::Interest};
    ///
    /// struct MyFilter {
    ///     // ...
    /// }
    ///
    /// impl MyFilter {
    ///     // The actual logic for determining whether a `Metadata` is enabled
    ///     // must be factored out from the `enabled` method, so that it can be
    ///     // called without a `Context` (which is not provided to the
    ///     // `callsite_enabled` method).
    ///     fn is_enabled(&self, metadata: &Metadata<'_>) -> bool {
    ///         // ...
    ///         # drop(metadata); true
    ///     }
    /// }
    ///
    /// impl<C> subscribe::Filter<C> for MyFilter {
    ///     fn enabled(&self, metadata: &Metadata<'_>, _: &subscribe::Context<'_, C>) -> bool {
    ///         // Even though we are implementing `callsite_enabled`, we must still provide a
    ///         // working implementation of `enabled`, as returning `Interest::always()` or
    ///         // `Interest::never()` will *allow* caching, but will not *guarantee* it.
    ///         // Other filters may still return `Interest::sometimes()`, so we may be
    ///         // asked again in `enabled`.
    ///         self.is_enabled(metadata)
    ///     }
    ///
    ///     fn callsite_enabled(&self, metadata: &'static Metadata<'static>) -> Interest {
    ///         // The result of `self.enabled(metadata, ...)` will always be
    ///         // the same for any given `Metadata`, so we can convert it into
    ///         // an `Interest`:
    ///         if self.is_enabled(metadata) {
    ///             Interest::always()
    ///         } else {
    ///             Interest::never()
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// [`Metadata`]: tracing_core::Metadata
    /// [`Interest`]: tracing_core::Interest
    /// [always]: tracing_core::Interest::always
    /// [sometimes]: tracing_core::Interest::sometimes
    /// [never]: tracing_core::Interest::never
    /// [`Collect::register_callsite`]: tracing_core::Collect::register_callsite
    /// [`Collect`]: tracing_core::Collect
    /// [`enabled`]: Filter::enabled
    /// [`Filtered`]: crate::filter::Filtered
    fn callsite_enabled(&self, meta: &'static Metadata<'static>) -> Interest {
        let _ = meta;
        Interest::sometimes()
    }

    /// Returns an optional hint of the highest [verbosity level][level] that
    /// this `Filter` will enable.
    ///
    /// If this method returns a [`LevelFilter`], it will be used as a hint to
    /// determine the most verbose level that will be enabled. This will allow
    /// spans and events which are more verbose than that level to be skipped
    /// more efficiently. An implementation of this method is optional, but
    /// strongly encouraged.
    ///
    /// If the maximum level the `Filter` will enable can change over the
    /// course of its lifetime, it is free to return a different value from
    /// multiple invocations of this method. However, note that changes in the
    /// maximum level will **only** be reflected after the callsite [`Interest`]
    /// cache is rebuilt, by calling the
    /// [`tracing_core::callsite::rebuild_interest_cache`] function.
    /// Therefore, if the `Filter will change the value returned by this
    /// method, it is responsible for ensuring that [`rebuild_interest_cache`]
    /// is called after the value of the max level changes.
    ///
    /// ## Default Implementation
    ///
    /// By default, this method returns `None`, indicating that the maximum
    /// level is unknown.
    ///
    /// [level]: tracing_core::metadata::Level
    /// [`LevelFilter`]: crate::filter::LevelFilter
    /// [`Interest`]: tracing_core::collect::Interest
    /// [`rebuild_interest_cache`]: tracing_core::callsite::rebuild_interest_cache
    fn max_level_hint(&self) -> Option<LevelFilter> {
        None
    }

    /// Called before the filtered subscribers' [`on_event`], to determine if
    /// `on_event` should be called.
    ///
    /// This gives a chance to filter events based on their fields. Note,
    /// however, that this *does not* override [`enabled`], and is not even
    /// called if [`enabled`] returns `false`.
    ///
    /// ## Default Implementation
    ///
    /// By default, this method returns `true`, indicating that no events are
    /// filtered out based on their fields.
    ///
    /// [`enabled`]: crate::subscribe::Filter::enabled
    /// [`on_event`]: crate::subscribe::Subscribe::on_event
    #[inline] // collapse this to a constant please mrs optimizer
    fn event_enabled(&self, event: &Event<'_>, cx: &Context<'_, S>) -> bool {
        let _ = (event, cx);
        true
    }

    /// Notifies this filter that a new span was constructed with the given
    /// `Attributes` and `Id`.
    ///
    /// By default, this method does nothing. `Filter` implementations that
    /// need to be notified when new spans are created can override this
    /// method.
    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        let _ = (attrs, id, ctx);
    }

    /// Notifies this filter that a span with the given `Id` recorded the given
    /// `values`.
    ///
    /// By default, this method does nothing. `Filter` implementations that
    /// need to be notified when new spans are created can override this
    /// method.
    fn on_record(&self, id: &span::Id, values: &span::Record<'_>, ctx: Context<'_, S>) {
        let _ = (id, values, ctx);
    }

    /// Notifies this filter that a span with the given ID was entered.
    ///
    /// By default, this method does nothing. `Filter` implementations that
    /// need to be notified when a span is entered can override this method.
    fn on_enter(&self, id: &span::Id, ctx: Context<'_, S>) {
        let _ = (id, ctx);
    }

    /// Notifies this filter that a span with the given ID was exited.
    ///
    /// By default, this method does nothing. `Filter` implementations that
    /// need to be notified when a span is exited can override this method.
    fn on_exit(&self, id: &span::Id, ctx: Context<'_, S>) {
        let _ = (id, ctx);
    }

    /// Notifies this filter that a span with the given ID has been closed.
    ///
    /// By default, this method does nothing. `Filter` implementations that
    /// need to be notified when a span is closed can override this method.
    fn on_close(&self, id: span::Id, ctx: Context<'_, S>) {
        let _ = (id, ctx);
    }
}

/// Extension trait adding a `with(Subscribe)` combinator to types implementing
/// [`Collect`].
pub trait CollectExt: Collect + crate::sealed::Sealed {
    /// Wraps `self` with the provided `subscriber`.
    fn with<S>(self, subscriber: S) -> Layered<S, Self>
    where
        S: Subscribe<Self>,
        Self: Sized,
    {
        subscriber.with_collector(self)
    }
}
/// A subscriber that does nothing.
#[derive(Clone, Debug, Default)]
pub struct Identity {
    _p: (),
}

// === impl Subscribe ===

#[derive(Clone, Copy)]
pub(crate) struct NoneLayerMarker(());
static NONE_LAYER_MARKER: NoneLayerMarker = NoneLayerMarker(());

/// Is a type implementing `Subscriber` `Option::<_>::None`?
pub(crate) fn subscriber_is_none<S, C>(subscriber: &S) -> bool
where
    S: Subscribe<C>,
    C: Collect,
{
    unsafe {
        // Safety: we're not actually *doing* anything with this pointer ---
        // this only care about the `Option`, which is essentially being used
        // as a bool. We can rely on the pointer being valid, because it is
        // a crate-private type, and is only returned by the `Subscribe` impl
        // for `Option`s. However, even if the subscriber *does* decide to be
        // evil and give us an invalid pointer here, that's fine, because we'll
        // never actually dereference it.
        subscriber.downcast_raw(TypeId::of::<NoneLayerMarker>())
    }
    .is_some()
}

/// Is a type implementing `Collect` `Option::<_>::None`?
pub(crate) fn collector_is_none<C>(collector: &C) -> bool
where
    C: Collect,
{
    unsafe {
        // Safety: we're not actually *doing* anything with this pointer ---
        // this only care about the `Option`, which is essentially being used
        // as a bool. We can rely on the pointer being valid, because it is
        // a crate-private type, and is only returned by the `Subscribe` impl
        // for `Option`s. However, even if the subscriber *does* decide to be
        // evil and give us an invalid pointer here, that's fine, because we'll
        // never actually dereference it.
        collector.downcast_raw(TypeId::of::<NoneLayerMarker>())
    }
    .is_some()
}

impl<S, C> Subscribe<C> for Option<S>
where
    S: Subscribe<C>,
    C: Collect,
{
    fn on_register_dispatch(&self, collector: &Dispatch) {
        if let Some(ref subscriber) = self {
            subscriber.on_register_dispatch(collector)
        }
    }

    fn on_subscribe(&mut self, collector: &mut C) {
        if let Some(ref mut subscriber) = self {
            subscriber.on_subscribe(collector)
        }
    }

    #[inline]
    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, C>) {
        if let Some(ref inner) = self {
            inner.on_new_span(attrs, id, ctx)
        }
    }

    #[inline]
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        match self {
            Some(ref inner) => inner.register_callsite(metadata),
            None => Interest::always(),
        }
    }

    #[inline]
    fn enabled(&self, metadata: &Metadata<'_>, ctx: Context<'_, C>) -> bool {
        match self {
            Some(ref inner) => inner.enabled(metadata, ctx),
            None => true,
        }
    }

    #[inline]
    fn max_level_hint(&self) -> Option<LevelFilter> {
        match self {
            Some(ref inner) => inner.max_level_hint(),
            None => {
                // There is no inner subscriber, so this subscriber will
                // never enable anything.
                Some(LevelFilter::OFF)
            }
        }
    }

    #[inline]
    fn on_record(&self, span: &span::Id, values: &span::Record<'_>, ctx: Context<'_, C>) {
        if let Some(ref inner) = self {
            inner.on_record(span, values, ctx);
        }
    }

    #[inline]
    fn on_follows_from(&self, span: &span::Id, follows: &span::Id, ctx: Context<'_, C>) {
        if let Some(ref inner) = self {
            inner.on_follows_from(span, follows, ctx);
        }
    }

    #[inline]
    fn event_enabled(&self, event: &Event<'_>, ctx: Context<'_, C>) -> bool {
        match self {
            Some(ref inner) => inner.event_enabled(event, ctx),
            None => true,
        }
    }

    #[inline]
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, C>) {
        if let Some(ref inner) = self {
            inner.on_event(event, ctx);
        }
    }

    #[inline]
    fn on_enter(&self, id: &span::Id, ctx: Context<'_, C>) {
        if let Some(ref inner) = self {
            inner.on_enter(id, ctx);
        }
    }

    #[inline]
    fn on_exit(&self, id: &span::Id, ctx: Context<'_, C>) {
        if let Some(ref inner) = self {
            inner.on_exit(id, ctx);
        }
    }

    #[inline]
    fn on_close(&self, id: span::Id, ctx: Context<'_, C>) {
        if let Some(ref inner) = self {
            inner.on_close(id, ctx);
        }
    }

    #[inline]
    fn on_id_change(&self, old: &span::Id, new: &span::Id, ctx: Context<'_, C>) {
        if let Some(ref inner) = self {
            inner.on_id_change(old, new, ctx)
        }
    }

    #[doc(hidden)]
    #[inline]
    unsafe fn downcast_raw(&self, id: TypeId) -> Option<NonNull<()>> {
        if id == TypeId::of::<Self>() {
            Some(NonNull::from(self).cast())
        } else if id == TypeId::of::<NoneLayerMarker>() && self.is_none() {
            Some(NonNull::from(&NONE_LAYER_MARKER).cast())
        } else {
            self.as_ref().and_then(|inner| inner.downcast_raw(id))
        }
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
macro_rules! subscriber_impl_body {
    () => {
        fn on_register_dispatch(&self, collector: &Dispatch) {
            self.deref().on_register_dispatch(collector);
        }

        #[inline]
        fn on_subscribe(&mut self, collect: &mut C) {
            self.deref_mut().on_subscribe(collect);
        }

        #[inline]
        fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
            self.deref().register_callsite(metadata)
        }

        #[inline]
        fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, C>) {
            self.deref().on_new_span(attrs, id, ctx)
        }

        #[inline]
        fn enabled(&self, metadata: &Metadata<'_>, ctx: Context<'_, C>) -> bool {
            self.deref().enabled(metadata, ctx)
        }

        #[inline]
        fn on_record(&self, span: &span::Id, values: &span::Record<'_>, ctx: Context<'_, C>) {
            self.deref().on_record(span, values, ctx)
        }

        #[inline]
        fn on_follows_from(&self, span: &span::Id, follows: &span::Id, ctx: Context<'_, C>) {
            self.deref().on_follows_from(span, follows, ctx)
        }

        #[inline]
        fn event_enabled(&self, event: &Event<'_>, ctx: Context<'_, C>) -> bool {
            self.deref().event_enabled(event, ctx)
        }

        #[inline]
        fn on_event(&self, event: &Event<'_>, ctx: Context<'_, C>) {
            self.deref().on_event(event, ctx)
        }

        #[inline]
        fn on_enter(&self, id: &span::Id, ctx: Context<'_, C>) {
            self.deref().on_enter(id, ctx)
        }

        #[inline]
        fn on_exit(&self, id: &span::Id, ctx: Context<'_, C>) {
            self.deref().on_exit(id, ctx)
        }

        #[inline]
        fn on_close(&self, id: span::Id, ctx: Context<'_, C>) {
            self.deref().on_close(id, ctx)
        }

        #[inline]
        fn on_id_change(&self, old: &span::Id, new: &span::Id, ctx: Context<'_, C>) {
            self.deref().on_id_change(old, new, ctx)
        }

        #[inline]
        fn max_level_hint(&self) -> Option<LevelFilter> {
            self.deref().max_level_hint()
        }

        #[doc(hidden)]
        #[inline]
        unsafe fn downcast_raw(&self, id: TypeId) -> ::core::option::Option<NonNull<()>> {
            self.deref().downcast_raw(id)
        }
    };
}

feature! {
    #![any(feature = "std", feature = "alloc")]

    impl<S, C> Subscribe<C> for Box<S>
    where
        S: Subscribe<C>,
        C: Collect,
    {
        subscriber_impl_body! {}
    }

    impl<C> Subscribe<C> for Box<dyn Subscribe<C> + Send + Sync + 'static>
    where
        C: Collect,
    {
        subscriber_impl_body! {}
    }


    impl<C, S> Subscribe<C> for alloc::vec::Vec<S>
    where
        S: Subscribe<C>,
        C: Collect,
    {
        fn on_register_dispatch(&self, collector: &Dispatch) {
            for s in self {
                s.on_register_dispatch(collector);
            }
        }

        fn on_subscribe(&mut self, collector: &mut C) {
            for s in self {
                s.on_subscribe(collector);
            }
        }

        fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
            // Return highest level of interest.
            let mut interest = Interest::never();
            for s in self {
                let new_interest = s.register_callsite(metadata);
                if (interest.is_sometimes() && new_interest.is_always())
                    || (interest.is_never() && !new_interest.is_never())
                {
                    interest = new_interest;
                }
            }

            interest
        }

        fn enabled(&self, metadata: &Metadata<'_>, ctx: Context<'_, C>) -> bool {
            self.iter().all(|s| s.enabled(metadata, ctx.clone()))
        }

        fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, C>) {
            for s in self {
                s.on_new_span(attrs, id, ctx.clone());
            }
        }

        fn max_level_hint(&self) -> Option<LevelFilter> {
            // Default to `OFF` if there are no underlying subscribers
            let mut max_level = LevelFilter::OFF;
            for s in self {
                // NOTE(eliza): this is slightly subtle: if *any* subscriber
                // returns `None`, we have to return `None`, assuming there is
                // no max level hint, since that particular subscriber cannot
                // provide a hint.
                let hint = s.max_level_hint()?;
                max_level = core::cmp::max(hint, max_level);
            }
            Some(max_level)
        }

        fn on_record(&self, span: &span::Id, values: &span::Record<'_>, ctx: Context<'_, C>) {
            for s in self {
                s.on_record(span, values, ctx.clone())
            }
        }

        fn on_follows_from(&self, span: &span::Id, follows: &span::Id, ctx: Context<'_, C>) {
            for s in self {
                s.on_follows_from(span, follows, ctx.clone());
            }
        }

        fn on_event(&self, event: &Event<'_>, ctx: Context<'_, C>) {
            for s in self {
                s.on_event(event, ctx.clone());
            }
        }

        fn on_enter(&self, id: &span::Id, ctx: Context<'_, C>) {
            for s in self {
                s.on_enter(id, ctx.clone());
            }
        }

        fn on_exit(&self, id: &span::Id, ctx: Context<'_, C>) {
            for s in self {
                s.on_exit(id, ctx.clone());
            }
        }

        fn on_close(&self, id: span::Id, ctx: Context<'_, C>) {
            for s in self {
                s.on_close(id.clone(), ctx.clone());
            }
        }

        #[doc(hidden)]
        unsafe fn downcast_raw(&self, id: TypeId) -> Option<NonNull<()>> {
            // If downcasting to `Self`, return a pointer to `self`.
            if id == TypeId::of::<Self>() {
                return Some(NonNull::from(self).cast());
            }

            // Someone is looking for per-subscriber filters. But, this `Vec`
            // might contain subscribers with per-subscriber filters *and*
            // subscribers without filters. It should only be treated as a
            // per-subscriber-filtered subscriber if *all* its subscribers have
            // per-subscriber filters.
            // XXX(eliza): it's a bummer we have to do this linear search every
            // time. It would be nice if this could be cached, but that would
            // require replacing the `Vec` impl with an impl for a newtype...
            if filter::is_psf_downcast_marker(id) && self.iter().any(|s| s.downcast_raw(id).is_none()) {
                return None;
            }

            // Otherwise, return the first child of `self` that downcaaasts to
            // the selected type, if any.
            // XXX(eliza): hope this is reasonable lol
            self.iter().find_map(|s| s.downcast_raw(id))
        }
    }
}

// === impl CollectExt ===

impl<C: Collect> crate::sealed::Sealed for C {}
impl<C: Collect> CollectExt for C {}

// === impl Identity ===

impl<C: Collect> Subscribe<C> for Identity {}

impl Identity {
    /// Returns a new `Identity` subscriber.
    pub fn new() -> Self {
        Self { _p: () }
    }
}
