//! A composable abstraction for building `Subscriber`s.
use tracing_core::{
    metadata::Metadata,
    span,
    subscriber::{Interest, Subscriber},
    Event,
};

#[cfg(feature = "registry")]
use crate::registry::{self, LookupMetadata, LookupSpan, Registry};
use std::{any::TypeId, marker::PhantomData};

/// A composable handler for `tracing` events.
///
/// The [`Subscriber`] trait in `tracing-core` represents the _complete_ set of
/// functionality required to consume `tracing` instrumentation. This means that
/// a single `Subscriber` instance is a self-contained implementation of a
/// complete strategy for collecting traces; but it _also_ means that the
/// `Subscriber` trait cannot easily be composed with other `Subscriber`s.
///
/// In particular, [`Subscriber`]'s are responsible for generating [span IDs] and
/// assigning them to spans. Since these IDs must uniquely identify a span
/// within the context of the current trace, this means that there may only be
/// a single `Subscriber` for a given thread at any point in time &mdash;
/// otherwise, there would be no authoritative source of span IDs.
///
/// On the other hand, the majority of the [`Subscriber`] trait's functionality
/// is composable: any number of subscribers may _observe_ events, span entry
/// and exit, and so on, provided that there is a single authoritative source of
/// span IDs. The `Layer` trait represents this composable subset of the
/// [`Subscriber`] behavior; it can _observe_ events and spans, but does not
/// assign IDs.
///
/// ## Recording Traces
///
/// The `Layer` trait defines a set of methods for consuming notifications from
/// tracing instrumentation, which are generally equivalent to the similarly
/// named methods on [`Subscriber`]. Unlike [`Subscriber`], the methods on
/// `Layer` are additionally passed a [`Context`] type, which exposes additional
/// information provided by the wrapped subscriber (such as [the current span])
/// to the layer.
///
/// ## Filtering with `Layer`s
///
/// As well as strategies for handling trace events, the `Layer` trait may also
/// be used to represent composable _filters_. This allows the determination of
/// what spans and events should be recorded to be decoupled from _how_ they are
/// recorded: a filtering layer can be applied to other layers or
/// subscribers. A `Layer` that implements a filtering strategy should override the
/// [`register_callsite`] and/or [`enabled`] methods. It may also choose to implement
/// methods such as [`on_enter`], if it wishes to filter trace events based on
/// the current span context.
///
/// Note that the [`Layer::register_callsite`] and [`Layer::enabled`] methods
/// determine whether a span or event is enabled *globally*. Thus, they should
/// **not** be used to indicate whether an individual layer wishes to record a
/// particular span or event. Instead, if a layer is only interested in a subset
/// of trace data, but does *not* wish to disable other spans and events for the
/// rest of the layer stack should ignore those spans and events in its
/// notification methods.
///
/// The filtering methods on a stack of `Layer`s are evaluated in a top-down
/// order, starting with the outermost `Layer` and ending with the wrapped
/// [`Subscriber`]. If any layer returns `false` from its [`enabled`] method, or
/// [`Interest::never()`] from its [`register_callsite`] method, filter
/// evaluation will short-circuit and the span or event will be disabled.
///
/// [`Subscriber`]: https://docs.rs/tracing-core/latest/tracing_core/subscriber/trait.Subscriber.html
/// [span IDs]: https://docs.rs/tracing-core/latest/tracing_core/span/struct.Id.html
/// [`Context`]: struct.Context.html
/// [the current span]: struct.Context.html#method.current_span
/// [`register_callsite`]: #method.register_callsite
/// [`enabled`]: #method.enabled
/// [`on_enter`]: #method.on_enter
/// [`Layer::register_callsite`]: #method.register_callsite
/// [`Layer::enabled`]: #method.enabled
/// [`Interest::never()`]: https://docs.rs/tracing-core/latest/tracing_core/subscriber/struct.Interest.html#method.never
pub trait Layer<S>
where
    S: Subscriber,
    Self: 'static,
{
    /// Registers a new callsite with this layer, returning whether or not
    /// the layer is interested in being notified about the callsite, similarly
    /// to [`Subscriber::register_callsite`].
    ///
    /// By default, this returns [`Interest::always()`] if [`self.enabled`] returns
    /// true, or [`Interest::never()`] if it returns false.
    ///
    /// **Note:** This method (and [`Layer::enabled`]) determine whether a
    /// span or event is globally enabled, _not_ whether the individual layer
    /// will be notified about that span or event. This is intended to be used
    /// by layers that implement filtering for the entire stack. Layers which do
    /// not wish to be notified about certain spans or events but do not wish to
    /// globally disable them should ignore those spans or events in their
    /// [`on_event`], [`on_enter`], [`on_exit`], and other notification methods.
    ///
    /// See [the trait-level documentation] for more information on filtering
    /// with `Layer`s.
    ///
    /// Layers may also implement this method to perform any behaviour that
    /// should be run once per callsite. If the layer wishes to use
    /// `register_callsite` for per-callsite behaviour, but does not want to
    /// globally enable or disable those callsites, it should always return
    /// [`Interest::always()`].
    ///
    /// [`Interest`]: https://docs.rs/tracing-core/latest/tracing_core/struct.Interest.html
    /// [`Subscriber::register_callsite`]: https://docs.rs/tracing-core/latest/tracing_core/trait.Subscriber.html#method.register_callsite
    /// [`Interest::never()`]: https://docs.rs/tracing-core/latest/tracing_core/subscriber/struct.Interest.html#method.never
    /// [`Interest::always()`]: https://docs.rs/tracing-core/latest/tracing_core/subscriber/struct.Interest.html#method.always
    /// [`self.enabled`]: #method.enabled
    /// [`Layer::enabled`]: #method.enabled
    /// [`on_event`]: #method.on_event
    /// [`on_enter`]: #method.on_enter
    /// [`on_exit`]: #method.on_exit
    /// [the trait-level documentation]: #filtering-with-layers
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        if self.enabled(metadata, Context::none()) {
            Interest::always()
        } else {
            Interest::never()
        }
    }

    /// Returns `true` if this layer is interested in a span or event with the
    /// given `metadata` in the current [`Context`], similarly to
    /// [`Subscriber::enabled`].
    ///
    /// By default, this always returns `true`, allowing the wrapped subscriber
    /// to choose to disable the span.
    ///
    /// **Note:** This method (and [`Layer::register_callsite`]) determine whether a
    /// span or event is globally enabled, _not_ whether the individual layer
    /// will be notified about that span or event. This is intended to be used
    /// by layers that implement filtering for the entire stack. Layers which do
    /// not wish to be notified about certain spans or events but do not wish to
    /// globally disable them should ignore those spans or events in their
    /// [`on_event`], [`on_enter`], [`on_exit`], and other notification methods.
    ///
    ///
    /// See [the trait-level documentation] for more information on filtering
    /// with `Layer`s.
    ///
    /// [`Interest`]: https://docs.rs/tracing-core/latest/tracing_core/struct.Interest.html
    /// [`Context`]: ../struct.Context.html
    /// [`Subscriber::enabled`]: https://docs.rs/tracing-core/latest/tracing_core/trait.Subscriber.html#method.enabled
    /// [`Layer::register_callsite`]: #method.register_callsite
    /// [`on_event`]: #method.on_event
    /// [`on_enter`]: #method.on_enter
    /// [`on_exit`]: #method.on_exit
    /// [the trait-level documentation]: #filtering-with-layers
    fn enabled(&self, metadata: &Metadata<'_>, ctx: Context<'_, S>) -> bool {
        let _ = (metadata, ctx);
        true
    }

    /// Notifies this layer that a new span was constructed with the given
    /// `Attributes` and `Id`.
    fn new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        let _ = (attrs, id, ctx);
    }

    /// Notifies this layer that a span with the given `Id` recorded the given
    /// `values`.
    // Note: it's unclear to me why we'd need the current span in `record` (the
    // only thing the `Context` type currently provides), but passing it in anyway
    // seems like a good future-proofing measure as it may grow other methods later...
    fn on_record(&self, _span: &span::Id, _values: &span::Record<'_>, _ctx: Context<'_, S>) {}

    /// Notifies this layer that a span with the ID `span` recorded that it
    /// follows from the span with the ID `follows`.
    // Note: it's unclear to me why we'd need the current span in `record` (the
    // only thing the `Context` type currently provides), but passing it in anyway
    // seems like a good future-proofing measure as it may grow other methods later...
    fn on_follows_from(&self, _span: &span::Id, _follows: &span::Id, _ctx: Context<'_, S>) {}

    /// Notifies this layer that an event has occurred.
    fn on_event(&self, _event: &Event<'_>, _ctx: Context<'_, S>) {}

    /// Notifies this layer that a span with the given ID was entered.
    fn on_enter(&self, _id: &span::Id, _ctx: Context<'_, S>) {}

    /// Notifies this layer that the span with the given ID was exited.
    fn on_exit(&self, _id: &span::Id, _ctx: Context<'_, S>) {}

    /// Notifies this layer that the span with the given ID has been closed.
    fn on_close(&self, _id: span::Id, _ctx: Context<'_, S>) {}

    /// Notifies this layer that a span ID has been cloned, and that the
    /// subscriber returned a different ID.
    fn on_id_change(&self, _old: &span::Id, _new: &span::Id, _ctx: Context<'_, S>) {}

    /// Composes this layer around the given `Layer`, returning a `Layered`
    /// struct implementing `Layer`.
    ///
    /// The returned `Layer` will call the methods on this `Layer` and then
    /// those of the new `Layer`, before calling the methods on the subscriber
    /// it wraps. For example:
    ///
    /// ```rust
    /// # use tracing_subscriber::layer::Layer;
    /// # use tracing_core::Subscriber;
    /// # fn main() {
    /// pub struct FooLayer {
    ///     // ...
    /// }
    ///
    /// pub struct BarLayer {
    ///     // ...
    /// }
    ///
    /// pub struct MySubscriber {
    ///     // ...
    /// }
    ///
    /// impl<S: Subscriber> Layer<S> for FooLayer {
    ///     // ...
    /// }
    ///
    /// impl<S: Subscriber> Layer<S> for BarLayer {
    ///     // ...
    /// }
    ///
    /// # impl FooLayer {
    /// # fn new() -> Self { Self {} }
    /// # }
    /// # impl BarLayer {
    /// # fn new() -> Self { Self { }}
    /// # }
    /// # impl MySubscriber {
    /// # fn new() -> Self { Self { }}
    /// # }
    /// # use tracing_core::{span::{Id, Attributes, Record}, Metadata, Event};
    /// # impl tracing_core::Subscriber for MySubscriber {
    /// #   fn new_span(&self, _: &Attributes) -> Id { Id::from_u64(1) }
    /// #   fn record(&self, _: &Id, _: &Record) {}
    /// #   fn event(&self, _: &Event) {}
    /// #   fn record_follows_from(&self, _: &Id, _: &Id) {}
    /// #   fn enabled(&self, _: &Metadata) -> bool { false }
    /// #   fn enter(&self, _: &Id) {}
    /// #   fn exit(&self, _: &Id) {}
    /// # }
    /// let subscriber = FooLayer::new()
    ///     .and_then(BarLayer::new())
    ///     .with_subscriber(MySubscriber::new());
    /// # }
    /// ```
    ///
    /// Multiple layers may be composed in this manner:
    ///
    /// ```rust
    /// # use tracing_subscriber::layer::Layer;
    /// # use tracing_core::Subscriber;
    /// # fn main() {
    /// # pub struct FooLayer {}
    /// # pub struct BarLayer {}
    /// # pub struct MySubscriber {}
    /// # impl<S: Subscriber> Layer<S> for FooLayer {}
    /// # impl<S: Subscriber> Layer<S> for BarLayer {}
    /// # impl FooLayer {
    /// # fn new() -> Self { Self {} }
    /// # }
    /// # impl BarLayer {
    /// # fn new() -> Self { Self { }}
    /// # }
    /// # impl MySubscriber {
    /// # fn new() -> Self { Self { }}
    /// # }
    /// # use tracing_core::{span::{Id, Attributes, Record}, Metadata, Event};
    /// # impl tracing_core::Subscriber for MySubscriber {
    /// #   fn new_span(&self, _: &Attributes) -> Id { Id::from_u64(1) }
    /// #   fn record(&self, _: &Id, _: &Record) {}
    /// #   fn event(&self, _: &Event) {}
    /// #   fn record_follows_from(&self, _: &Id, _: &Id) {}
    /// #   fn enabled(&self, _: &Metadata) -> bool { false }
    /// #   fn enter(&self, _: &Id) {}
    /// #   fn exit(&self, _: &Id) {}
    /// # }
    /// pub struct BazLayer {
    ///     // ...
    /// }
    ///
    /// impl<S: Subscriber> Layer<S> for BazLayer {
    ///     // ...
    /// }
    /// # impl BazLayer { fn new() -> Self { BazLayer {} } }
    ///
    /// let subscriber = FooLayer::new()
    ///     .and_then(BarLayer::new())
    ///     .and_then(BazLayer::new())
    ///     .with_subscriber(MySubscriber::new());
    /// # }
    /// ```
    fn and_then<L>(self, layer: L) -> Layered<L, Self, S>
    where
        L: Layer<S>,
        Self: Sized,
    {
        Layered {
            layer,
            inner: self,
            _s: PhantomData,
        }
    }

    /// Composes this `Layer` with the given [`Subscriber`], returning a
    /// `Layered` struct that implements [`Subscriber`].
    ///
    /// The returned `Layered` subscriber will call the methods on this `Layer`
    /// and then those of the wrapped subscriber.
    ///
    /// For example:
    /// ```rust
    /// # use tracing_subscriber::layer::Layer;
    /// # use tracing_core::Subscriber;
    /// # fn main() {
    /// pub struct FooLayer {
    ///     // ...
    /// }
    ///
    /// pub struct MySubscriber {
    ///     // ...
    /// }
    ///
    /// impl<S: Subscriber> Layer<S> for FooLayer {
    ///     // ...
    /// }
    ///
    /// # impl FooLayer {
    /// # fn new() -> Self { Self {} }
    /// # }
    /// # impl MySubscriber {
    /// # fn new() -> Self { Self { }}
    /// # }
    /// # use tracing_core::{span::{Id, Attributes, Record}, Metadata};
    /// # impl tracing_core::Subscriber for MySubscriber {
    /// #   fn new_span(&self, _: &Attributes) -> Id { Id::from_u64(0) }
    /// #   fn record(&self, _: &Id, _: &Record) {}
    /// #   fn event(&self, _: &tracing_core::Event) {}
    /// #   fn record_follows_from(&self, _: &Id, _: &Id) {}
    /// #   fn enabled(&self, _: &Metadata) -> bool { false }
    /// #   fn enter(&self, _: &Id) {}
    /// #   fn exit(&self, _: &Id) {}
    /// # }
    /// let subscriber = FooLayer::new()
    ///     .with_subscriber(MySubscriber::new());
    /// # }
    ///```
    ///
    /// [`Subscriber`]: https://docs.rs/tracing-core/latest/tracing_core/trait.Subscriber.html
    fn with_subscriber(self, inner: S) -> Layered<Self, S>
    where
        Self: Sized,
    {
        Layered {
            layer: self,
            inner,
            _s: PhantomData,
        }
    }

    #[doc(hidden)]
    unsafe fn downcast_raw(&self, id: TypeId) -> Option<*const ()> {
        if id == TypeId::of::<Self>() {
            Some(&self as *const _ as *const ())
        } else {
            None
        }
    }
}

/// Extension trait adding a `with(Layer)` combinator to `Subscriber`s.
pub trait SubscriberExt: Subscriber + crate::sealed::Sealed {
    /// Wraps `self` with the provided `layer`.
    fn with<L>(self, layer: L) -> Layered<L, Self>
    where
        L: Layer<Self>,
        Self: Sized,
    {
        Layered {
            layer,
            inner: self,
            _s: PhantomData,
        }
    }
}

/// Represents information about the current context provided to [`Layer`]s by the
/// wrapped [`Subscriber`].
///
/// [`Layer`]: ../struct.Layer.html
/// [`Subscriber`]: https://docs.rs/tracing-core/latest/tracing_core/trait.Subscriber.html
#[derive(Debug)]
pub struct Context<'a, S> {
    subscriber: Option<&'a S>,
}

/// A [`Subscriber`] composed of a `Subscriber` wrapped by one or more
/// [`Layer`]s.
///
/// [`Layer`]: ../struct.Layer.html
/// [`Subscriber`]: https://docs.rs/tracing-core/latest/tracing_core/trait.Subscriber.html
#[derive(Clone, Debug)]
pub struct Layered<L, I, S = I> {
    layer: L,
    inner: I,
    _s: PhantomData<fn(S)>,
}

/// A layer that does nothing.
#[derive(Clone, Debug)]
pub struct Identity {
    _p: (),
}

// === impl Layered ===

impl<L, S> Subscriber for Layered<L, S>
where
    L: Layer<S>,
    S: Subscriber,
{
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        let outer = self.layer.register_callsite(metadata);
        if outer.is_never() {
            // if the outer layer has disabled the callsite, return now so that
            // the subscriber doesn't get its hopes up.
            return outer;
        }

        let inner = self.inner.register_callsite(metadata);
        if outer.is_sometimes() {
            // if this interest is "sometimes", return "sometimes" to ensure that
            // filters are reevaluated.
            outer
        } else {
            // otherwise, allow the inner subscriber to weigh in.
            inner
        }
    }

    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        if self.layer.enabled(metadata, self.ctx()) {
            // if the outer layer enables the callsite metadata, ask the subscriber.
            self.inner.enabled(metadata)
        } else {
            // otherwise, the callsite is disabled by the layer
            false
        }
    }

    fn new_span(&self, span: &span::Attributes<'_>) -> span::Id {
        let id = self.inner.new_span(span);
        self.layer.new_span(span, &id, self.ctx());
        id
    }

    fn record(&self, span: &span::Id, values: &span::Record<'_>) {
        self.inner.record(span, values);
        self.layer.on_record(span, values, self.ctx());
    }

    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {
        self.inner.record_follows_from(span, follows);
        self.layer.on_follows_from(span, follows, self.ctx());
    }

    fn event(&self, event: &Event<'_>) {
        self.inner.event(event);
        self.layer.on_event(event, self.ctx());
    }

    fn enter(&self, span: &span::Id) {
        self.inner.enter(span);
        self.layer.on_enter(span, self.ctx());
    }

    fn exit(&self, span: &span::Id) {
        self.inner.exit(span);
        self.layer.on_exit(span, self.ctx());
    }

    fn clone_span(&self, old: &span::Id) -> span::Id {
        let new = self.inner.clone_span(old);
        if &new != old {
            self.layer.on_id_change(old, &new, self.ctx())
        };
        new
    }

    #[inline]
    fn drop_span(&self, id: span::Id) {
        self.try_close(id);
    }

    fn try_close(&self, id: span::Id) -> bool {
        let subscriber = &self.inner as &dyn Subscriber;
        let _guard = match subscriber.downcast_ref::<Registry>() {
            Some(registry) => Some(registry.ref_guard(id.clone())),
            None => None,
        };

        let id2 = id.clone();
        if self.inner.try_close(id) {
            self.layer.on_close(id2, self.ctx());
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
    unsafe fn downcast_raw(&self, id: TypeId) -> Option<*const ()> {
        self.layer
            .downcast_raw(id)
            .or_else(|| self.inner.downcast_raw(id))
    }
}

impl<S, A, B> Layer<S> for Layered<A, B, S>
where
    A: Layer<S>,
    B: Layer<S>,
    S: Subscriber,
{
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        let outer = self.layer.register_callsite(metadata);
        if outer.is_never() {
            // if the outer layer has disabled the callsite, return now so that
            // inner layers  don't get their hopes up.
            return outer;
        }

        let inner = self.inner.register_callsite(metadata);
        if outer.is_sometimes() {
            // if this interest is "sometimes", return "sometimes" to ensure that
            // filters are reevaluated.
            outer
        } else {
            // otherwise, allow the inner layer to weigh in.
            inner
        }
    }

    fn enabled(&self, metadata: &Metadata<'_>, ctx: Context<'_, S>) -> bool {
        if self.layer.enabled(metadata, ctx.clone()) {
            // if the outer layer enables the callsite metadata, ask the inner layer.
            self.inner.enabled(metadata, ctx)
        } else {
            // otherwise, the callsite is disabled by this layer
            false
        }
    }

    #[inline]
    fn new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        self.inner.new_span(attrs, id, ctx.clone());
        self.layer.new_span(attrs, id, ctx);
    }

    #[inline]
    fn on_record(&self, span: &span::Id, values: &span::Record<'_>, ctx: Context<'_, S>) {
        self.inner.on_record(span, values, ctx.clone());
        self.layer.on_record(span, values, ctx);
    }

    #[inline]
    fn on_follows_from(&self, span: &span::Id, follows: &span::Id, ctx: Context<'_, S>) {
        self.inner.on_follows_from(span, follows, ctx.clone());
        self.layer.on_follows_from(span, follows, ctx);
    }

    #[inline]
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        self.inner.on_event(event, ctx.clone());
        self.layer.on_event(event, ctx);
    }

    #[inline]
    fn on_enter(&self, id: &span::Id, ctx: Context<'_, S>) {
        self.inner.on_enter(id, ctx.clone());
        self.layer.on_enter(id, ctx);
    }

    #[inline]
    fn on_exit(&self, id: &span::Id, ctx: Context<'_, S>) {
        self.inner.on_exit(id, ctx.clone());
        self.layer.on_exit(id, ctx);
    }

    #[inline]
    fn on_close(&self, id: span::Id, ctx: Context<'_, S>) {
        self.inner.on_close(id.clone(), ctx.clone());
        self.layer.on_close(id, ctx);
    }

    #[inline]
    fn on_id_change(&self, old: &span::Id, new: &span::Id, ctx: Context<'_, S>) {
        self.inner.on_id_change(old, new, ctx.clone());
        self.layer.on_id_change(old, new, ctx);
    }

    #[doc(hidden)]
    unsafe fn downcast_raw(&self, id: TypeId) -> Option<*const ()> {
        self.layer
            .downcast_raw(id)
            .or_else(|| self.inner.downcast_raw(id))
    }
}

#[cfg(feature = "registry")]
impl<L, S> LookupMetadata for Layered<L, S>
where
    S: Subscriber + LookupMetadata,
{
    fn metadata(&self, span: &span::Id) -> Option<&'static Metadata<'static>> {
        self.inner.metadata(span)
    }
}

impl<L, S> Layered<L, S>
where
    S: Subscriber,
{
    fn ctx(&self) -> Context<'_, S> {
        Context {
            subscriber: Some(&self.inner),
        }
    }
}

// impl<L, S> Layered<L, S> {
//     // TODO(eliza): is there a compelling use-case for this being public?
//     pub(crate) fn into_inner(self) -> S {
//         self.inner
//     }
// }

// === impl SubscriberExt ===

impl<S: Subscriber> crate::sealed::Sealed for S {}
impl<S: Subscriber> SubscriberExt for S {}

// === impl Context ===

impl<'a, S: Subscriber> Context<'a, S> {
    /// Returns the wrapped subscriber's view of the current span.
    #[inline]
    pub fn current_span(&self) -> span::Current {
        self.subscriber
            .map(Subscriber::current_span)
            // TODO: this would be more correct as "unknown", so perhaps
            // `tracing-core` should make `Current::unknown()` public?
            .unwrap_or_else(span::Current::none)
    }

    /// Returns whether the wrapped subscriber would enable the current span.
    #[inline]
    pub fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.subscriber
            .map(|subscriber| subscriber.enabled(metadata))
            // If this context is `None`, we are registering a callsite, so
            // return `true` so that the layer does not incorrectly assume that
            // the inner subscriber has disabled this metadata.
            // TODO(eliza): would it be more correct for this to return an `Option`?
            .unwrap_or(true)
    }

    /// Records the provided `event` with the wrapped subscriber.
    ///
    /// # Notes
    ///
    /// - The subscriber is free to expect that the event's callsite has been
    ///   [registered][register], and may panic or fail to observe the event if this is
    ///   not the case. The `tracing` crate's macros ensure that all events are
    ///   registered, but if the event is constructed through other means, the
    ///   user is responsible for ensuring that [`register_callsite`][register]
    ///   has been called prior to calling this method.
    /// - This does _not_ call [`enabled`] on the inner subscriber. If the
    ///   caller wishes to apply the wrapped subscriber's filter before choosing
    ///   whether to record the event, it may first call [`Context::enabled`] to
    ///   check whether the event would be enabled. This allows `Layer`s to
    ///   elide constructing the event if it would not be recorded.
    ///
    /// [register]: https://docs.rs/tracing-core/latest/tracing_core/subscriber/trait.Subscriber.html#method.register_callsite
    /// [`enabled`]: https://docs.rs/tracing-core/latest/tracing_core/subscriber/trait.Subscriber.html#method.enabled
    /// [`Context::enabled`]: #method.enabled
    #[inline]
    pub fn event(&self, event: &Event<'_>) {
        if let Some(ref subscriber) = self.subscriber {
            subscriber.event(event);
        }
    }

    /// Returns metadata for the span with the given `id`, if it exists.
    ///
    /// If this returns `None`, then no span exists for that ID (either it has
    /// closed or the ID is invalid).
    ///
    /// **Note**: This requires the wrapped subscriber to implement the
    /// [`LookupMetadata`] trait. `Layer` implementations that wish to use this
    /// function can bound their `Subscriber` type parameter with
    /// ```rust,ignore
    /// where S: Subscriber + LookupMetadata,
    /// ```
    /// or similar.
    ///
    /// [`LookupMetadata`]: ../registry/trait.LookupMetadata.html
    #[inline]
    #[cfg(feature = "registry")]
    pub fn metadata(&self, id: &span::Id) -> Option<&'static Metadata<'static>>
    where
        S: LookupMetadata,
    {
        self.subscriber.as_ref()?.metadata(id)
    }

    /// Returns [stored data] for the span with the given `id`, if it exists.
    ///
    /// If this returns `None`, then no span exists for that ID (either it has
    /// closed or the ID is invalid).
    ///
    /// **Note**: This requires the wrapped subscriber to implement the
    /// [`LookupSpan`] trait. `Layer` implementations that wish to use this
    /// function can bound their `Subscriber` type parameter with
    /// ```rust,ignore
    /// where S: Subscriber + for<'span> LookupSpan<'span>,
    /// ```
    /// or similar.
    ///
    /// [stored data]: ../registry/struct.SpanRef.html
    /// [`LookupSpan`]: ../registry/trait.LookupSpan.html
    #[inline]
    #[cfg(feature = "registry")]
    pub fn span(&'a self, id: &span::Id) -> Option<registry::SpanRef<'a, S>>
    where
        S: LookupSpan<'a>,
    {
        self.subscriber.as_ref()?.span(id)
    }

    /// Returns `true` if an active span exists for the given `Id`.
    ///
    /// **Note**: This requires the wrapped subscriber to implement the
    /// [`LookupMetadata`] trait. `Layer` implementations that wish to use this
    /// function can bound their `Subscriber` type parameter with
    /// ```rust,ignore
    /// where S: Subscriber + LookupMetadata,
    /// ```
    /// or similar.
    ///
    /// [`LookupMetadata`]: ../registry/trait.LookupMetadata.html
    #[inline]
    #[cfg(feature = "registry")]
    pub fn exists(&self, id: &span::Id) -> bool
    where
        S: LookupMetadata,
    {
        self.subscriber
            .as_ref()
            .map(|s| s.exists(id))
            .unwrap_or(false)
    }
}

impl<'a, S> Context<'a, S> {
    pub(crate) fn none() -> Self {
        Self { subscriber: None }
    }
}

impl<'a, S> Clone for Context<'a, S> {
    #[inline]
    fn clone(&self) -> Self {
        let subscriber = if let Some(ref subscriber) = self.subscriber {
            Some(*subscriber)
        } else {
            None
        };
        Context { subscriber }
    }
}

// === impl Identity ===
//
impl<S: Subscriber> Layer<S> for Identity {}

impl Identity {
    /// Returns a new `Identity` layer.
    pub fn new() -> Self {
        Self { _p: () }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) struct NopSubscriber;

    impl Subscriber for NopSubscriber {
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
    }

    struct NopLayer;
    impl<S: Subscriber> Layer<S> for NopLayer {}

    struct NopLayer2;
    impl<S: Subscriber> Layer<S> for NopLayer2 {}

    fn assert_subscriber(_s: impl Subscriber) {}

    #[test]
    fn layer_is_subscriber() {
        let s = NopLayer.with_subscriber(NopSubscriber);
        assert_subscriber(s)
    }

    #[test]
    fn two_layers_are_subscriber() {
        let s = NopLayer.and_then(NopLayer).with_subscriber(NopSubscriber);
        assert_subscriber(s)
    }

    #[test]
    fn three_layers_are_subscriber() {
        let s = NopLayer
            .and_then(NopLayer)
            .and_then(NopLayer)
            .with_subscriber(NopSubscriber);
        assert_subscriber(s)
    }

    #[test]
    fn downcasts_to_subscriber() {
        let s = NopLayer
            .and_then(NopLayer)
            .and_then(NopLayer)
            .with_subscriber(NopSubscriber);
        assert!(Subscriber::downcast_ref::<NopSubscriber>(&s).is_some());
    }

    #[test]
    fn downcasts_to_layer() {
        let s = NopLayer
            .and_then(NopLayer)
            .and_then(NopLayer2)
            .with_subscriber(NopSubscriber);
        assert!(Subscriber::downcast_ref::<NopLayer>(&s).is_some());
        assert!(Subscriber::downcast_ref::<NopLayer2>(&s).is_some());
    }
}
