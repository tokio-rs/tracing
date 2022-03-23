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
//! ## Composing Subscribers
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
//! [prelude]: crate::prelude
//!
//! ## Recording Traces
//!
//! The [`Subscribe`] trait defines a set of methods for consuming notifications from
//! tracing instrumentation, which are generally equivalent to the similarly
//! named methods on [`Collect`]. Unlike [`Collect`], the methods on
//! `Subscribe` are additionally passed a [`Context`] type, which exposes additional
//! information provided by the wrapped subscriber (such as [the current span])
//! to the subscriber.
//!
//! ## Filtering with `Subscribers`s
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
//! ### Global Filtering
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
//! ### Per-Subscriber Filtering
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
//! [`FilterFn`]. These [`Filter`]s provide ready-made implementations of
//! common forms of filtering. For custom filtering policies, the [`FilterFn`]
//! and [`DynFilterFn`] types allow implementing a [`Filter`] with a closure or
//! function pointer. In addition, when more control is required, the [`Filter`]
//! trait may also be implemented for user-defined types.
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
//! // A gteneral-purpose logging subscriber.
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
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};
use tracing_core::{
    collect::{Collect, Interest},
    metadata::Metadata,
    span, Event, LevelFilter,
};

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
pub trait Subscribe<C>
where
    C: Collect,
    Self: 'static,
{
    /// Performs late initialization when attaching a subscriber to a
    /// [collector].
    ///
    /// This is a callback that is called when the `Subscribe` is added to a
    /// [`Collect`] (e.g. in [`Subscribe::with_collector`] and
    /// [`CollectExt::with`]). Since this can only occur before the
    /// [`Collect`] has been set as the default, both the subscriber and
    /// [`Collect`] are passed to this method _mutably_. This gives the
    /// subscribe the opportunity to set any of its own fields with values
    /// recieved by method calls on the [`Collect`].
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
#[cfg(feature = "registry")]
#[cfg_attr(docsrs, doc(cfg(feature = "registry"), notable_trait))]
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
    /// a span or event was observered (e.g. only enabling an event when it
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
    /// will recieve [`Interest::sometimes()`][sometimes], and the [`enabled`]
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
    /// [`tracing_core::callsite::rebuild_interest_cache`][rebuild] function.
    /// Therefore, if the `Filter will change the value returned by this
    /// method, it is responsible for ensuring that
    /// [`rebuild_interest_cache`][rebuild] is called after the value of the max
    /// level changes.
    ///
    /// ## Default Implementation
    ///
    /// By default, this method returns `None`, indicating that the maximum
    /// level is unknown.
    ///
    /// [level]: tracing_core::metadata::Level
    /// [`LevelFilter`]: crate::filter::LevelFilter
    /// [`Interest`]: tracing_core::collect::Interest
    /// [rebuild]: tracing_core::callsite::rebuild_interest_cache
    fn max_level_hint(&self) -> Option<LevelFilter> {
        None
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

impl<S, C> Subscribe<C> for Option<S>
where
    S: Subscribe<C>,
    C: Collect,
{
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
            None => None,
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
        } else {
            self.as_ref().and_then(|inner| inner.downcast_raw(id))
        }
    }
}

macro_rules! subscriber_impl_body {
    () => {
        #[inline]
        fn on_subscribe(&mut self, collect: &mut C) {
            self.deref_mut().on_subscribe(collect);
        }

        #[inline]
        fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, C>) {
            self.deref().on_new_span(attrs, id, ctx)
        }

        #[inline]
        fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
            self.deref().register_callsite(metadata)
        }

        #[inline]
        fn enabled(&self, metadata: &Metadata<'_>, ctx: Context<'_, C>) -> bool {
            self.deref().enabled(metadata, ctx)
        }

        #[inline]
        fn max_level_hint(&self) -> Option<LevelFilter> {
            self.deref().max_level_hint()
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

        #[doc(hidden)]
        #[inline]
        unsafe fn downcast_raw(&self, id: TypeId) -> Option<NonNull<()>> {
            self.deref().downcast_raw(id)
        }
    };
}

impl<S, C> Subscribe<C> for Box<S>
where
    S: Subscribe<C>,
    C: Collect,
{
    subscriber_impl_body! {}
}

impl<C> Subscribe<C> for Box<dyn Subscribe<C>>
where
    C: Collect,
{
    subscriber_impl_body! {}
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
