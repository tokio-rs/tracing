//! A composable abstraction for building [collector]s.
//!
//! [collector]: tracing_core::Collect
use tracing_core::{
    collect::{Collect, Interest},
    metadata::Metadata,
    span, Event, LevelFilter,
};

#[cfg(all(feature = "std", feature = "registry"))]
use crate::registry::Registry;
use crate::registry::{self, LookupSpan, SpanRef};
use core::{any::TypeId, cmp, marker::PhantomData, ptr::NonNull};

feature! {
    #![feature = "alloc"]
    use alloc::boxed::Box;
    use core::ops::Deref;
}

/// A composable handler for `tracing` events.
///
/// The [`Collect`] trait in `tracing-core` represents the _complete_ set of
/// functionality required to consume `tracing` instrumentation. This means that
/// a single [collector] instance is a self-contained implementation of a
/// complete strategy for collecting traces; but it _also_ means that the
/// `Collect` trait cannot easily be composed with other `Collect`s.
///
/// In particular, collectors are responsible for generating [span IDs] and
/// assigning them to spans. Since these IDs must uniquely identify a span
/// within the context of the current trace, this means that there may only be
/// a single [collector] for a given thread at any point in time &mdash;
/// otherwise, there would be no authoritative source of span IDs.
///
/// [collector]: tracing_core::Collect
///
/// On the other hand, the majority of the [`Collect`] trait's functionality
/// is composable: any number of collectors may _observe_ events, span entry
/// and exit, and so on, provided that there is a single authoritative source of
/// span IDs. The `Subscribe` trait represents this composable subset of the
/// [`Collect`]'s behavior; it can _observe_ events and spans, but does not
/// assign IDs.
///
/// ## Composing Subscribers
///
/// Since `Subscribe` does not implement a complete strategy for collecting
/// traces, it must be composed with a `Collect` in order to be used. The
/// `Subscribe` trait is generic over a type parameter (called `C` in the trait
/// definition), representing the types of `Collect` they can be composed
/// with. Thus, a subscriber may be implemented that will only compose with a
/// particular `Collect` implementation, or additional trait bounds may be
/// added to constrain what types implementing `Collect` a subscriber can wrap.
///
/// Subscribers may be added to a `Collect` by using the [`CollectExt::with`]
/// method, which is provided by `tracing-subscriber`'s [prelude]. This method
/// returns a [`Layered`] struct that implements `Collect` by composing the
/// subscriber with the collector.
///
/// For example:
/// ```rust
/// use tracing_subscriber::Subscribe;
/// use tracing_subscriber::subscribe::CollectExt;
/// use tracing::Collect;
/// use tracing_core::span::Current;
///
/// pub struct MySubscriber {
///     // ...
/// }
///
/// impl<C: Collect> Subscribe<C> for MySubscriber {
///     // ...
/// }
///
/// pub struct MyCollector {
///     // ...
/// }
///
/// # use tracing_core::{span::{Id, Attributes, Record}, Metadata, Event};
/// impl Collect for MyCollector {
///     // ...
/// #   fn new_span(&self, _: &Attributes) -> Id { Id::from_u64(1) }
/// #   fn record(&self, _: &Id, _: &Record) {}
/// #   fn event(&self, _: &Event) {}
/// #   fn record_follows_from(&self, _: &Id, _: &Id) {}
/// #   fn enabled(&self, _: &Metadata) -> bool { false }
/// #   fn enter(&self, _: &Id) {}
/// #   fn exit(&self, _: &Id) {}
/// #   fn current_span(&self) -> Current { Current::unknown() }
/// }
/// # impl MySubscriber {
/// # fn new() -> Self { Self {} }
/// # }
/// # impl MyCollector {
/// # fn new() -> Self { Self { }}
/// # }
///
/// let collector = MyCollector::new()
///     .with(MySubscriber::new());
///
/// tracing::collect::set_global_default(collector);
/// ```
///
/// Multiple `Subscriber`s may be composed in the same manner:
/// ```rust
/// # use tracing_subscriber::Subscribe;
/// # use tracing_subscriber::subscribe::CollectExt;
/// # use tracing::Collect;
/// # use tracing_core::span::Current;
/// pub struct MyOtherSubscriber {
///     // ...
/// }
///
/// impl<C: Collect> Subscribe<C> for MyOtherSubscriber {
///     // ...
/// }
///
/// pub struct MyThirdSubscriber {
///     // ...
/// }
///
/// impl<C: Collect> Subscribe<C> for MyThirdSubscriber {
///     // ...
/// }
/// # pub struct MySubscriber {}
/// # impl<C: Collect> Subscribe<C> for MySubscriber {}
/// # pub struct MyCollector { }
/// # use tracing_core::{span::{Id, Attributes, Record}, Metadata, Event};
/// # impl Collect for MyCollector {
/// #   fn new_span(&self, _: &Attributes) -> Id { Id::from_u64(1) }
/// #   fn record(&self, _: &Id, _: &Record) {}
/// #   fn event(&self, _: &Event) {}
/// #   fn record_follows_from(&self, _: &Id, _: &Id) {}
/// #   fn enabled(&self, _: &Metadata) -> bool { false }
/// #   fn enter(&self, _: &Id) {}
/// #   fn exit(&self, _: &Id) {}
/// #   fn current_span(&self) -> Current { Current::unknown() }
/// }
/// # impl MySubscriber {
/// # fn new() -> Self { Self {} }
/// # }
/// # impl MyOtherSubscriber {
/// # fn new() -> Self { Self {} }
/// # }
/// # impl MyThirdSubscriber {
/// # fn new() -> Self { Self {} }
/// # }
/// # impl MyCollector {
/// # fn new() -> Self { Self { }}
/// # }
///
/// let collector = MyCollector::new()
///     .with(MySubscriber::new())
///     .with(MyOtherSubscriber::new())
///     .with(MyThirdSubscriber::new());
///
/// tracing::collect::set_global_default(collector);
/// ```
///
/// The [`Subscribe::with_collector` method][with-col] constructs the `Layered`
/// type from a `Subscribe` and `Collect`, and is called by
/// [`CollectExt::with`]. In general, it is more idiomatic to use
/// `CollectExt::with`, and treat `Subscribe::with_collector` as an
/// implementation detail, as `with_collector` calls must be nested, leading to
/// less clear code for the reader. However, subscribers which wish to perform
/// additional behavior when composed with a subscriber may provide their own
/// implementations of `SubscriberExt::with`.
///
/// [prelude]: super::prelude
/// [with-col]: Subscribe::with_collector()
///
/// ## Recording Traces
///
/// The `Subscribe` trait defines a set of methods for consuming notifications from
/// tracing instrumentation, which are generally equivalent to the similarly
/// named methods on [`Collect`]. Unlike [`Collect`], the methods on
/// `Subscribe` are additionally passed a [`Context`] type, which exposes additional
/// information provided by the wrapped collector (such as [the current span])
/// to the subscriber.
///
/// ## Filtering with Subscribers
///
/// As well as strategies for handling trace events, the `Subscriber` trait may also
/// be used to represent composable _filters_. This allows the determination of
/// what spans and events should be recorded to be decoupled from _how_ they are
/// recorded: a filtering subscriber can be applied to other subscribers or
/// collectors. A `Subscriber` that implements a filtering strategy should override the
/// [`register_callsite`] and/or [`enabled`] methods. It may also choose to implement
/// methods such as [`on_enter`], if it wishes to filter trace events based on
/// the current span context.
///
/// Note that the [`Subscribe::register_callsite`] and [`Subscribe::enabled`] methods
/// determine whether a span or event is enabled *globally*. Thus, they should
/// **not** be used to indicate whether an individual subscriber wishes to record a
/// particular span or event. Instead, if a subscriber is only interested in a subset
/// of trace data, but does *not* wish to disable other spans and events for the
/// rest of the subscriber stack should ignore those spans and events in its
/// notification methods.
///
/// The filtering methods on a stack of subscribers are evaluated in a top-down
/// order, starting with the outermost `Subscribe` and ending with the wrapped
/// [`Collect`]. If any subscriber returns `false` from its [`enabled`] method, or
/// [`Interest::never()`] from its [`register_callsite`] method, filter
/// evaluation will short-circuit and the span or event will be disabled.
///
/// [`Collect`]: tracing_core::collect::Collect
/// [span IDs]: tracing_core::span::Id
/// [the current span]: Context::current_span()
/// [`register_callsite`]: Subscribe::register_callsite()
/// [`enabled`]: Subscribe::enabled()
/// [`on_enter`]: Subscribe::on_enter()
pub trait Subscribe<C>
where
    C: Collect,
    Self: 'static,
{
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
    /// with `Subscriber`s.
    ///
    /// Subscribers may also implement this method to perform any behaviour that
    /// should be run once per callsite. If the subscriber wishes to use
    /// `register_callsite` for per-callsite behaviour, but does not want to
    /// globally enable or disable those callsites, it should always return
    /// [`Interest::always()`].
    ///
    /// [`Interest`]: tracing_core::Interest
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
    /// by layers that implement filtering for the entire stack. Layers which do
    /// not wish to be notified about certain spans or events but do not wish to
    /// globally disable them should ignore those spans or events in their
    /// [on_event][Self::on_event], [on_enter][Self::on_enter], [on_exit][Self::on_exit],
    /// and other notification methods.
    ///
    /// </pre></div>
    ///
    ///
    /// See [the trait-level documentation] for more information on filtering
    /// with `Subscriber`s.
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
    // filtering subscribers to a separate trait, we may no longer want `Subscriber`s to
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
        Layered {
            subscriber,
            inner: self,
            _s: PhantomData,
        }
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
    fn with_collector(self, inner: C) -> Layered<Self, C>
    where
        Self: Sized,
    {
        Layered {
            subscriber: self,
            inner,
            _s: PhantomData,
        }
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

/// Extension trait adding a `with(Subscriber)` combinator to `Collect`.
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

/// Represents information about the current context provided to [subscriber]s by the
/// wrapped [collector].
///
/// To access [stored data] keyed by a span ID, implementors of the `Subscribe`
/// trait should ensure that the `Collect` type parameter is *also* bound by the
/// [`LookupSpan`]:
///
/// ```rust
/// use tracing::Collect;
/// use tracing_subscriber::{Subscribe, registry::LookupSpan};
///
/// pub struct MyCollector;
///
/// impl<C> Subscribe<C> for MyCollector
/// where
///     C: Collect + for<'a> LookupSpan<'a>,
/// {
///     // ...
/// }
/// ```
///
/// [subscriber]: Subscribe
/// [collector]: tracing_core::Collect
/// [stored data]: super::registry::SpanRef
#[derive(Debug)]
pub struct Context<'a, C> {
    collector: Option<&'a C>,
}

/// A [collector] composed of a collector wrapped by one or more
/// [subscriber]s.
///
/// [subscriber]: super::subscribe::Subscribe
/// [collector]: tracing_core::Collect
#[derive(Clone, Debug)]
pub struct Layered<S, I, C = I> {
    subscriber: S,
    inner: I,
    _s: PhantomData<fn(C)>,
}

/// A Subscriber that does nothing.
#[derive(Clone, Debug, Default)]
pub struct Identity {
    _p: (),
}

// === impl Layered ===

impl<S, C> Collect for Layered<S, C>
where
    S: Subscribe<C>,
    C: Collect,
{
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        let outer = self.subscriber.register_callsite(metadata);
        if outer.is_never() {
            // if the outer subscriber has disabled the callsite, return now so that
            // the collector doesn't get its hopes up.
            return outer;
        }

        // The intention behind calling `inner.register_callsite()` before the if statement
        // is to ensure that the inner subscriber is informed that the callsite exists
        // regardless of the outer subscriber's filtering decision.
        let inner = self.inner.register_callsite(metadata);
        if outer.is_sometimes() {
            // if this interest is "sometimes", return "sometimes" to ensure that
            // filters are reevaluated.
            outer
        } else {
            // otherwise, allow the inner subscriber or collector to weigh in.
            inner
        }
    }

    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        if self.subscriber.enabled(metadata, self.ctx()) {
            // if the outer subscriber enables the callsite metadata, ask the collector.
            self.inner.enabled(metadata)
        } else {
            // otherwise, the callsite is disabled by the subscriber
            false
        }
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        cmp::max(
            self.subscriber.max_level_hint(),
            self.inner.max_level_hint(),
        )
    }

    fn new_span(&self, span: &span::Attributes<'_>) -> span::Id {
        let id = self.inner.new_span(span);
        self.subscriber.on_new_span(span, &id, self.ctx());
        id
    }

    fn record(&self, span: &span::Id, values: &span::Record<'_>) {
        self.inner.record(span, values);
        self.subscriber.on_record(span, values, self.ctx());
    }

    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {
        self.inner.record_follows_from(span, follows);
        self.subscriber.on_follows_from(span, follows, self.ctx());
    }

    fn event(&self, event: &Event<'_>) {
        self.inner.event(event);
        self.subscriber.on_event(event, self.ctx());
    }

    fn enter(&self, span: &span::Id) {
        self.inner.enter(span);
        self.subscriber.on_enter(span, self.ctx());
    }

    fn exit(&self, span: &span::Id) {
        self.inner.exit(span);
        self.subscriber.on_exit(span, self.ctx());
    }

    fn clone_span(&self, old: &span::Id) -> span::Id {
        let new = self.inner.clone_span(old);
        if &new != old {
            self.subscriber.on_id_change(old, &new, self.ctx())
        };
        new
    }

    #[inline]
    fn drop_span(&self, id: span::Id) {
        self.try_close(id);
    }

    fn try_close(&self, id: span::Id) -> bool {
        #[cfg(all(feature = "registry", feature = "std"))]
        let subscriber = &self.inner as &dyn Collect;
        #[cfg(all(feature = "registry", feature = "std"))]
        let mut guard = subscriber
            .downcast_ref::<Registry>()
            .map(|registry| registry.start_close(id.clone()));
        if self.inner.try_close(id.clone()) {
            // If we have a registry's close guard, indicate that the span is
            // closing.
            #[cfg(all(feature = "registry", feature = "std"))]
            {
                if let Some(g) = guard.as_mut() {
                    g.is_closing()
                };
            }

            self.subscriber.on_close(id, self.ctx());
            true
        } else {
            false
        }
    }

    #[inline]
    fn current_span(&self) -> span::Current {
        self.inner.current_span()
    }

    #[doc(hidden)]
    unsafe fn downcast_raw(&self, id: TypeId) -> Option<NonNull<()>> {
        if id == TypeId::of::<Self>() {
            return Some(NonNull::from(self).cast());
        }
        self.subscriber
            .downcast_raw(id)
            .or_else(|| self.inner.downcast_raw(id))
    }
}

impl<C, A, B> Subscribe<C> for Layered<A, B, C>
where
    A: Subscribe<C>,
    B: Subscribe<C>,
    C: Collect,
{
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        let outer = self.subscriber.register_callsite(metadata);
        if outer.is_never() {
            // if the outer subscriber has disabled the callsite, return now so that
            // inner subscribers don't get their hopes up.
            return outer;
        }

        // The intention behind calling `inner.register_callsite()` before the if statement
        // is to ensure that the inner subscriber is informed that the callsite exists
        // regardless of the outer subscriber's filtering decision.
        let inner = self.inner.register_callsite(metadata);
        if outer.is_sometimes() {
            // if this interest is "sometimes", return "sometimes" to ensure that
            // filters are reevaluated.
            outer
        } else {
            // otherwise, allow the inner subscriber or collector to weigh in.
            inner
        }
    }

    fn enabled(&self, metadata: &Metadata<'_>, ctx: Context<'_, C>) -> bool {
        if self.subscriber.enabled(metadata, ctx.clone()) {
            // if the outer subscriber enables the callsite metadata, ask the inner subscriber.
            self.inner.enabled(metadata, ctx)
        } else {
            // otherwise, the callsite is disabled by this subscriber
            false
        }
    }

    #[inline]
    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, C>) {
        self.inner.on_new_span(attrs, id, ctx.clone());
        self.subscriber.on_new_span(attrs, id, ctx);
    }

    #[inline]
    fn on_record(&self, span: &span::Id, values: &span::Record<'_>, ctx: Context<'_, C>) {
        self.inner.on_record(span, values, ctx.clone());
        self.subscriber.on_record(span, values, ctx);
    }

    #[inline]
    fn on_follows_from(&self, span: &span::Id, follows: &span::Id, ctx: Context<'_, C>) {
        self.inner.on_follows_from(span, follows, ctx.clone());
        self.subscriber.on_follows_from(span, follows, ctx);
    }

    #[inline]
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, C>) {
        self.inner.on_event(event, ctx.clone());
        self.subscriber.on_event(event, ctx);
    }

    #[inline]
    fn on_enter(&self, id: &span::Id, ctx: Context<'_, C>) {
        self.inner.on_enter(id, ctx.clone());
        self.subscriber.on_enter(id, ctx);
    }

    #[inline]
    fn on_exit(&self, id: &span::Id, ctx: Context<'_, C>) {
        self.inner.on_exit(id, ctx.clone());
        self.subscriber.on_exit(id, ctx);
    }

    #[inline]
    fn on_close(&self, id: span::Id, ctx: Context<'_, C>) {
        self.inner.on_close(id.clone(), ctx.clone());
        self.subscriber.on_close(id, ctx);
    }

    #[inline]
    fn on_id_change(&self, old: &span::Id, new: &span::Id, ctx: Context<'_, C>) {
        self.inner.on_id_change(old, new, ctx.clone());
        self.subscriber.on_id_change(old, new, ctx);
    }

    #[doc(hidden)]
    unsafe fn downcast_raw(&self, id: TypeId) -> Option<NonNull<()>> {
        if id == TypeId::of::<Self>() {
            return Some(NonNull::from(self).cast());
        }
        self.subscriber
            .downcast_raw(id)
            .or_else(|| self.inner.downcast_raw(id))
    }
}

impl<S, C> Subscribe<C> for Option<S>
where
    S: Subscribe<C>,
    C: Collect,
{
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
    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, C>) {
        if let Some(ref inner) = self {
            inner.on_new_span(attrs, id, ctx)
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

feature! {
    #![any(feature = "std", feature = "alloc")]

    macro_rules! subscriber_impl_body {
        () => {
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

    impl<C> Subscribe<C> for Box<dyn Subscribe<C> + Send + Sync>
    where
        C: Collect,
    {
        subscriber_impl_body! {}
    }
}

impl<'a, S, C> LookupSpan<'a> for Layered<S, C>
where
    C: Collect + LookupSpan<'a>,
{
    type Data = C::Data;

    fn span_data(&'a self, id: &span::Id) -> Option<Self::Data> {
        self.inner.span_data(id)
    }
}

impl<S, C> Layered<S, C>
where
    C: Collect,
{
    fn ctx(&self) -> Context<'_, C> {
        Context {
            collector: Some(&self.inner),
        }
    }
}

// impl<L, S> Layered<L, S> {
//     // TODO(eliza): is there a compelling use-case for this being public?
//     pub(crate) fn into_inner(self) -> S {
//         self.inner
//     }
// }

// === impl CollectExt ===

impl<C: Collect> crate::sealed::Sealed for C {}
impl<C: Collect> CollectExt for C {}

// === impl Context ===

impl<'a, C> Context<'a, C>
where
    C: Collect,
{
    /// Returns the wrapped subscriber's view of the current span.
    #[inline]
    pub fn current_span(&self) -> span::Current {
        self.collector
            .map(Collect::current_span)
            // TODO: this would be more correct as "unknown", so perhaps
            // `tracing-core` should make `Current::unknown()` public?
            .unwrap_or_else(span::Current::none)
    }

    /// Returns whether the wrapped subscriber would enable the current span.
    #[inline]
    pub fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.collector
            .map(|collector| collector.enabled(metadata))
            // If this context is `None`, we are registering a callsite, so
            // return `true` so that the subscriber does not incorrectly assume that
            // the inner collector has disabled this metadata.
            // TODO(eliza): would it be more correct for this to return an `Option`?
            .unwrap_or(true)
    }

    /// Records the provided `event` with the wrapped collector.
    ///
    /// # Notes
    ///
    /// - The collector is free to expect that the event's callsite has been
    ///   [registered][register], and may panic or fail to observe the event if this is
    ///   not the case. The `tracing` crate's macros ensure that all events are
    ///   registered, but if the event is constructed through other means, the
    ///   user is responsible for ensuring that [`register_callsite`][register]
    ///   has been called prior to calling this method.
    /// - This does _not_ call [`enabled`] on the inner collector. If the
    ///   caller wishes to apply the wrapped collector's filter before choosing
    ///   whether to record the event, it may first call [`Context::enabled`] to
    ///   check whether the event would be enabled. This allows `Collectors`s to
    ///   elide constructing the event if it would not be recorded.
    ///
    /// [register]: tracing_core::collect::Collect::register_callsite()
    /// [`enabled`]: tracing_core::collect::Collect::enabled()
    /// [`Context::enabled`]: Layered::enabled()
    #[inline]
    pub fn event(&self, event: &Event<'_>) {
        if let Some(collector) = self.collector {
            collector.event(event);
        }
    }

    /// Returns a [`SpanRef`] for the parent span of the given [`Event`], if
    /// it has a parent.
    ///
    /// If the event has an explicitly overridden parent, this method returns
    /// a reference to that span. If the event's parent is the current span,
    /// this returns a reference to the current span, if there is one. If this
    /// returns `None`, then either the event's parent was explicitly set to
    /// `None`, or the event's parent was defined contextually, but no span
    /// is currently entered.
    ///
    /// Compared to [`Context::current_span`] and [`Context::lookup_current`],
    /// this respects overrides provided by the [`Event`].
    ///
    /// Compared to [`Event::parent`], this automatically falls back to the contextual
    /// span, if required.
    ///
    /// ```rust
    /// use tracing::{Collect, Event};
    /// use tracing_subscriber::{
    ///     subscribe::{Context, Subscribe},
    ///     prelude::*,
    ///     registry::LookupSpan,
    /// };
    ///
    /// struct PrintingSubscriber;
    /// impl<C> Subscribe<C> for PrintingSubscriber
    /// where
    ///     C: Collect + for<'lookup> LookupSpan<'lookup>,
    /// {
    ///     fn on_event(&self, event: &Event, ctx: Context<C>) {
    ///         let span = ctx.event_span(event);
    ///         println!("Event in span: {:?}", span.map(|s| s.name()));
    ///     }
    /// }
    ///
    /// tracing::collect::with_default(tracing_subscriber::registry().with(PrintingSubscriber), || {
    ///     tracing::info!("no span");
    ///     // Prints: Event in span: None
    ///
    ///     let span = tracing::info_span!("span");
    ///     tracing::info!(parent: &span, "explicitly specified");
    ///     // Prints: Event in span: Some("span")
    ///
    ///     let _guard = span.enter();
    ///     tracing::info!("contextual span");
    ///     // Prints: Event in span: Some("span")
    /// });
    /// ```
    ///
    /// <div class="example-wrap" style="display:inline-block">
    /// <pre class="ignore" style="white-space:normal;font:inherit;">
    /// <strong>Note</strong>: This requires the wrapped subscriber to implement the
    /// <a href="../registry/trait.LookupSpan.html"><code>LookupSpan</code></a> trait.
    /// See the documentation on <a href="./struct.Context.html"><code>Context</code>'s
    /// declaration</a> for details.
    /// </pre></div>
    #[inline]
    pub fn event_span(&self, event: &Event<'_>) -> Option<SpanRef<'_, C>>
    where
        C: for<'lookup> LookupSpan<'lookup>,
    {
        if event.is_root() {
            None
        } else if event.is_contextual() {
            self.lookup_current()
        } else {
            event.parent().and_then(|id| self.span(id))
        }
    }

    /// Returns metadata for the span with the given `id`, if it exists.
    ///
    /// If this returns `None`, then no span exists for that ID (either it has
    /// closed or the ID is invalid).
    #[inline]
    pub fn metadata(&self, id: &span::Id) -> Option<&'static Metadata<'static>>
    where
        C: for<'lookup> LookupSpan<'lookup>,
    {
        let span = self.collector.as_ref()?.span(id)?;
        Some(span.metadata())
    }

    /// Returns [stored data] for the span with the given `id`, if it exists.
    ///
    /// If this returns `None`, then no span exists for that ID (either it has
    /// closed or the ID is invalid).
    ///
    /// <div class="example-wrap" style="display:inline-block">
    /// <pre class="ignore" style="white-space:normal;font:inherit;">
    ///
    /// **Note**: This requires the wrapped collector to implement the [`LookupSpan`] trait.
    /// See the documentation on [`Context`]'s declaration for details.
    ///
    /// </pre></div>
    ///
    /// [stored data]: super::registry::SpanRef
    #[inline]
    pub fn span(&self, id: &span::Id) -> Option<registry::SpanRef<'_, C>>
    where
        C: for<'lookup> LookupSpan<'lookup>,
    {
        self.collector.as_ref()?.span(id)
    }

    /// Returns `true` if an active span exists for the given `Id`.
    ///
    /// <div class="example-wrap" style="display:inline-block">
    /// <pre class="ignore" style="white-space:normal;font:inherit;">
    ///
    /// **Note**: This requires the wrapped subscriber to implement the [`LookupSpan`] trait.
    /// See the documentation on [`Context`]'s declaration for details.
    ///
    /// </pre></div>
    #[inline]
    pub fn exists(&self, id: &span::Id) -> bool
    where
        C: for<'lookup> LookupSpan<'lookup>,
    {
        self.collector.as_ref().and_then(|s| s.span(id)).is_some()
    }

    /// Returns [stored data] for the span that the wrapped collector considers
    /// to be the current.
    ///
    /// If this returns `None`, then we are not currently within a span.
    ///
    /// <div class="example-wrap" style="display:inline-block">
    /// <pre class="ignore" style="white-space:normal;font:inherit;">
    ///
    /// **Note**: This requires the wrapped subscriber to implement the [`LookupSpan`] trait.
    /// See the documentation on [`Context`]'s declaration for details.
    ///
    /// </pre></div>
    ///
    /// [stored data]: super::registry::SpanRef
    #[inline]
    pub fn lookup_current(&self) -> Option<registry::SpanRef<'_, C>>
    where
        C: for<'lookup> LookupSpan<'lookup>,
    {
        let collector = self.collector.as_ref()?;
        let current = collector.current_span();
        let id = current.id()?;
        let span = collector.span(id);
        debug_assert!(
            span.is_some(),
            "the subscriber should have data for the current span ({:?})!",
            id,
        );
        span
    }

    /// Returns an iterator over the [stored data] for all the spans in the
    /// current context, starting with the specified span and ending with the
    /// root of the trace tree and ending with the current span.
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
    /// [stored data]: ../registry/struct.SpanRef.html
    pub fn span_scope(&self, id: &span::Id) -> Option<registry::Scope<'_, C>>
    where
        C: for<'lookup> registry::LookupSpan<'lookup>,
    {
        Some(self.span(id)?.scope())
    }

    /// Returns an iterator over the [stored data] for all the spans in the
    /// current context, starting with the parent span of the specified event,
    /// and ending with the root of the trace tree and ending with the current span.
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
    /// [stored data]: ../registry/struct.SpanRef.html
    pub fn event_scope(&self, event: &Event<'_>) -> Option<registry::Scope<'_, C>>
    where
        C: for<'lookup> registry::LookupSpan<'lookup>,
    {
        Some(self.event_span(event)?.scope())
    }
}

impl<'a, C> Context<'a, C> {
    pub(crate) fn none() -> Self {
        Self { collector: None }
    }
}

impl<'a, C> Clone for Context<'a, C> {
    #[inline]
    fn clone(&self) -> Self {
        let collector = self.collector.as_ref().copied();
        Context { collector }
    }
}

// === impl Identity ===
//
impl<C: Collect> Subscribe<C> for Identity {}

impl Identity {
    /// Returns a new `Identity` subscriber.
    pub fn new() -> Self {
        Self { _p: () }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) struct NopCollector;

    impl Collect for NopCollector {
        fn register_callsite(&self, _: &'static Metadata<'static>) -> Interest {
            Interest::never()
        }

        fn enabled(&self, _: &Metadata<'_>) -> bool {
            false
        }

        fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
            span::Id::from_u64(1)
        }

        fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
        fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
        fn event(&self, _: &Event<'_>) {}
        fn enter(&self, _: &span::Id) {}
        fn exit(&self, _: &span::Id) {}
        fn current_span(&self) -> span::Current {
            span::Current::unknown()
        }
    }

    struct NopSubscriber;
    impl<C: Collect> Subscribe<C> for NopSubscriber {}

    #[allow(dead_code)]
    struct NopSubscriber2;
    impl<C: Collect> Subscribe<C> for NopSubscriber2 {}

    /// A subscriber that holds a string.
    ///
    /// Used to test that pointers returned by downcasting are actually valid.
    struct StringSubscriber(&'static str);
    impl<C: Collect> Subscribe<C> for StringSubscriber {}

    struct StringSubscriber2(&'static str);
    impl<C: Collect> Subscribe<C> for StringSubscriber2 {}

    struct StringSubscriber3(&'static str);
    impl<C: Collect> Subscribe<C> for StringSubscriber3 {}

    pub(crate) struct StringCollector(&'static str);

    impl Collect for StringCollector {
        fn register_callsite(&self, _: &'static Metadata<'static>) -> Interest {
            Interest::never()
        }

        fn enabled(&self, _: &Metadata<'_>) -> bool {
            false
        }

        fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
            span::Id::from_u64(1)
        }

        fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
        fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
        fn event(&self, _: &Event<'_>) {}
        fn enter(&self, _: &span::Id) {}
        fn exit(&self, _: &span::Id) {}
        fn current_span(&self) -> span::Current {
            span::Current::unknown()
        }
    }

    fn assert_collector(_s: impl Collect) {}

    #[test]
    fn subscriber_is_collector() {
        let s = NopSubscriber.with_collector(NopCollector);
        assert_collector(s)
    }

    #[test]
    fn two_subscribers_are_collector() {
        let s = NopSubscriber
            .and_then(NopSubscriber)
            .with_collector(NopCollector);
        assert_collector(s)
    }

    #[test]
    fn three_subscribers_are_collector() {
        let s = NopSubscriber
            .and_then(NopSubscriber)
            .and_then(NopSubscriber)
            .with_collector(NopCollector);
        assert_collector(s)
    }

    #[test]
    fn downcasts_to_collector() {
        let s = NopSubscriber
            .and_then(NopSubscriber)
            .and_then(NopSubscriber)
            .with_collector(StringCollector("collector"));
        let collector =
            <dyn Collect>::downcast_ref::<StringCollector>(&s).expect("collector should downcast");
        assert_eq!(collector.0, "collector");
    }

    #[test]
    fn downcasts_to_subscriber() {
        let s = StringSubscriber("subscriber_1")
            .and_then(StringSubscriber2("subscriber_2"))
            .and_then(StringSubscriber3("subscriber_3"))
            .with_collector(NopCollector);
        let subscriber = <dyn Collect>::downcast_ref::<StringSubscriber>(&s)
            .expect("subscriber 2 should downcast");
        assert_eq!(subscriber.0, "subscriber_1");
        let subscriber = <dyn Collect>::downcast_ref::<StringSubscriber2>(&s)
            .expect("subscriber 2 should downcast");
        assert_eq!(subscriber.0, "subscriber_2");
        let subscriber = <dyn Collect>::downcast_ref::<StringSubscriber3>(&s)
            .expect("subscriber 3 should downcast");
        assert_eq!(subscriber.0, "subscriber_3");
    }

    #[test]
    #[cfg(all(feature = "registry", feature = "std"))]
    fn context_event_span() {
        use std::sync::{Arc, Mutex};
        let last_event_span = Arc::new(Mutex::new(None));

        struct RecordingSubscriber {
            last_event_span: Arc<Mutex<Option<&'static str>>>,
        }

        impl<S> Subscribe<S> for RecordingSubscriber
        where
            S: Collect + for<'lookup> LookupSpan<'lookup>,
        {
            fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
                let span = ctx.event_span(event);
                *self.last_event_span.lock().unwrap() = span.map(|s| s.name());
            }
        }

        tracing::collect::with_default(
            crate::registry().with(RecordingSubscriber {
                last_event_span: last_event_span.clone(),
            }),
            || {
                tracing::info!("no span");
                assert_eq!(*last_event_span.lock().unwrap(), None);

                let parent = tracing::info_span!("explicit");
                tracing::info!(parent: &parent, "explicit span");
                assert_eq!(*last_event_span.lock().unwrap(), Some("explicit"));

                let _guard = tracing::info_span!("contextual").entered();
                tracing::info!("contextual span");
                assert_eq!(*last_event_span.lock().unwrap(), Some("contextual"));
            },
        );
    }
}
