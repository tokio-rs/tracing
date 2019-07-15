use tracing_core::{
    metadata::Metadata,
    span,
    subscriber::{Interest, Subscriber},
    Event,
};

use std::any::TypeId;

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
/// [`Subscriber`]: https://docs.rs/tracing-core/0.1.1/tracing_core//subscriber/trait.Subscriber.html
/// [span IDs]: https://docs.rs/tracing-core/0.1.1/tracing_core/span/struct.Id.html
pub trait Layer<S>: 'static {
    /// Registers a new callsite with this layer, returning whether or not
    /// the subscriber is interested in being notified about the callsite.
    ///
    /// This function is provided with the [`Interest`] returned by the wrapped
    /// subscriber. The layer may then choose to return that interest, ignore it
    /// entirely, or combine an `Interest` of its own with the prior `Interest`.
    ///
    /// Beyond that, this functions similarly to [`Subscriber::register_callsite`].
    ///
    /// By default, this simply returns the `Interest` returned by the wrapped
    /// subscriber.
    ///
    /// [`Interest`]: https://docs.rs/tracing-core/0.1.1/tracing_core/struct.Interest.html
    /// [`Subscriber::register_callsite`]: https://docs.rs/tracing-core/0.1.1/tracing_core/trait.Subscriber.html#method.register_callsite
    fn register_callsite(&self, _metadata: &'static Metadata<'static>, prev: Interest) -> Interest {
        prev
    }

    /// Returns `true` if this layer is interested in a span or event with the
    /// given `metadata`.
    ///
    /// This function is provided with the return value of the `enabled` function
    /// on the wrapped subscriber. The layer may then choose to return that
    /// value unmodified, ignore it entirely, or combine it with the result of
    /// applying a filter of its own.
    ///
    /// Beyond that, this functions similarly to [`Subscriber::enabled`].
    ///
    /// By default, this simply returns the value returned by the wrapped
    /// subscriber.
    ///
    /// [`Interest`]: https://docs.rs/tracing-core/0.1.1/tracing_core/struct.Interest.html
    /// [`Subscriber::enabled`]: https://docs.rs/tracing-core/0.1.1/tracing_core/trait.Subscriber.html#method.enabled
    fn enabled(&self, _metadata: &Metadata, prev: bool, _ctx: Context<S>) -> bool {
        prev
    }

    /// Notifies this layer that a new span was constructed with the given
    /// `Attributes` and `Id`.
    fn new_span(&self, _attrs: &span::Attributes, _id: &span::Id, _ctx: Context<S>) {}

    /// Notifies this layer that a span with the given `Id` recorded the given
    /// `values`.
    // Note: it's unclear to me why we'd need the current span in `record` (the
    // only thing the `Context` type currently provides), but passing it in anyway
    // seems like a good future-proofing measure as it may grow other methods later...
    fn on_record(&self, _span: &span::Id, _values: &span::Record, _ctx: Context<S>) {}

    /// Notifies this layer that a span with the ID `span` recorded that it
    /// follows from the span with the ID `follows`.
    // Note: it's unclear to me why we'd need the current span in `record` (the
    // only thing the `Context` type currently provides), but passing it in anyway
    // seems like a good future-proofing measure as it may grow other methods later...
    fn on_follows_from(&self, _span: &span::Id, _follows: &span::Id, _ctx: Context<S>) {}

    /// Notifies this layer that an event has occurred.
    fn on_event(&self, _event: &Event, _ctx: Context<S>) {}

    /// Notifies this layer that a span with the given ID was entered.
    fn on_enter(&self, _id: &span::Id, _ctx: Context<S>) {}

    /// Notifies this layer that the span with the given ID was exited.
    fn on_exit(&self, _id: &span::Id, _ctx: Context<S>) {}

    /// Notifies this layer that the span with the given ID has been closed.
    fn on_close(&self, _id: span::Id, _ctx: Context<S>) {}

    /// Notifies this layer that a span ID has been cloned, and that the
    /// subscriber returned a different ID.
    fn on_id_change(&self, _old: &span::Id, _new: &span::Id, _ctx: Context<S>) {}

    /// Composes the given [`Subscriber`] with this `Layer`, returning a `Layered` subscriber.
    ///
    /// The returned `Layered` subscriber will call the methods on this `Layer`
    /// and then those of the wrapped subscriber. Multiple layers may be
    /// composed in this manner. For example:
    /// ```rust
    /// # use tracing_subscriber::layer::Layer;
    /// # fn main() {
    /// pub struct FooLayer {
    ///     // ...
    /// }
    ///
    /// pub struct MySubscriber {
    ///     // ...
    /// }
    ///
    /// impl<S> Layer<S> for FooLayer {
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
    ///     .and_then(MySubscriber::new());
    /// # }
    /// ```
    /// Chaining multiple layers:
    /// ```rust
    /// # use tracing_subscriber::layer::Layer;
    /// # fn main() {
    /// # pub struct FooLayer {}
    /// pub struct BarLayer {
    ///     // ...
    /// }
    /// # pub struct MySubscriber {}
    /// # impl<S> Layer<S> for FooLayer {}
    ///
    /// impl<S> Layer<S> for BarLayer {
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
    ///     .and_then(MySubscriber::new());
    /// # }
    fn and_then(self, inner: S) -> Layered<Self, S>
    where
        Self: Sized,
    {
        Layered { layer: self, inner }
    }
}

pub trait SubscriberExt: Subscriber + crate::sealed::Sealed {
    fn with<L>(self, layer: L) -> Layered<L, Self>
    where
        L: Layer<Self>,
        Self: Sized,
    {
        Layered { layer, inner: self }
    }

    // fn with_enabled<F>(self, f: F) -> Layered<filter::EnabledFn<F>, Self>
    // where
    //     F: Fn(&Metadata) -> bool + 'static,
    //     Self: Sized,
    // {
    //     self.layer(filter::enabled_fn(f))
    // }

    // fn with_callsite_filter<F>(self, f: F) -> Layered<filter::InterestFn<F>, Self>
    // where
    //     F: Fn(&Metadata) -> Interest + 'static,
    //     Self: Sized,
    // {
    //     self.layer(filter::enabled_fn(f))
    // }
}

/// Represents information about the current context provided to `Layer`s by the
/// wrapped `Subscriber`.
#[derive(Debug)]
pub struct Context<'a, S> {
    subscriber: Option<&'a S>,
}

#[derive(Clone, Debug)]
pub struct Layered<L, S> {
    layer: L,
    inner: S,
}

// === impl Layered ===

impl<L, S> Subscriber for Layered<L, S>
where
    L: Layer<S>,
    S: Subscriber,
{
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        let interest = self.inner.register_callsite(metadata);
        self.layer.register_callsite(metadata, interest)
    }

    fn enabled(&self, metadata: &Metadata) -> bool {
        let enabled = self.inner.enabled(metadata);
        self.layer.enabled(metadata, enabled, self.ctx())
    }

    fn new_span(&self, span: &span::Attributes) -> span::Id {
        let id = self.inner.new_span(span);
        self.layer.new_span(span, &id, self.ctx());
        id
    }

    fn record(&self, span: &span::Id, values: &span::Record) {
        self.inner.record(span, values);
        self.layer.on_record(span, values, self.ctx());
    }

    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {
        self.inner.record_follows_from(span, follows);
        self.layer.on_follows_from(span, follows, self.ctx());
    }

    fn event(&self, event: &Event) {
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
        let id2 = id.clone();
        if self.inner.try_close(id) {
            self.layer.on_close(id2, self.ctx());
            true
        } else {
            false
        }
    }

    unsafe fn downcast_raw(&self, id: TypeId) -> Option<*const ()> {
        if id == TypeId::of::<L>() {
            Some(&self.layer as *const _ as *const ())
        } else {
            self.inner.downcast_raw(id)
        }
    }
}

impl<S, A, B> Layer<S> for Layered<A, B>
where
    A: Layer<S>,
    B: Layer<S>,
{
    #[inline]
    fn register_callsite(&self, metadata: &'static Metadata<'static>, prev: Interest) -> Interest {
        let prev = self.inner.register_callsite(metadata, prev);
        self.layer.register_callsite(metadata, prev)
    }

    #[inline]
    fn enabled(&self, metadata: &Metadata, prev: bool, ctx: Context<S>) -> bool {
        let prev = self.inner.enabled(metadata, prev, ctx.clone());
        self.layer.enabled(metadata, prev, ctx)
    }

    #[inline]
    fn new_span(&self, attrs: &span::Attributes, id: &span::Id, ctx: Context<S>) {
        self.inner.new_span(attrs, id, ctx.clone());
        self.layer.new_span(attrs, id, ctx);
    }

    #[inline]
    fn on_record(&self, span: &span::Id, values: &span::Record, ctx: Context<S>) {
        self.inner.on_record(span, values, ctx.clone());
        self.layer.on_record(span, values, ctx);
    }

    #[inline]
    fn on_follows_from(&self, span: &span::Id, follows: &span::Id, ctx: Context<S>) {
        self.inner.on_follows_from(span, follows, ctx.clone());
        self.layer.on_follows_from(span, follows, ctx);
    }

    #[inline]
    fn on_event(&self, event: &Event, ctx: Context<S>) {
        self.inner.on_event(event, ctx.clone());
        self.layer.on_event(event, ctx);
    }

    #[inline]
    fn on_enter(&self, id: &span::Id, ctx: Context<S>) {
        self.inner.on_enter(id, ctx.clone());
        self.layer.on_enter(id, ctx);
    }

    #[inline]
    fn on_exit(&self, id: &span::Id, ctx: Context<S>) {
        self.inner.on_exit(id, ctx.clone());
        self.layer.on_exit(id, ctx);
    }

    #[inline]
    fn on_close(&self, id: span::Id, ctx: Context<S>) {
        self.inner.on_close(id.clone(), ctx.clone());
        self.layer.on_close(id, ctx);
    }

    #[inline]
    fn on_id_change(&self, old: &span::Id, new: &span::Id, ctx: Context<S>) {
        self.inner.on_id_change(old, new, ctx.clone());
        self.layer.on_id_change(old, new, ctx);
    }
}

impl<L, S> Layered<L, S>
where
    S: Subscriber,
{
    fn ctx(&self) -> Context<S> {
        Context {
            subscriber: Some(&self.inner),
        }
    }
}

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
