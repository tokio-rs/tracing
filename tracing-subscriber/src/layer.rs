use tracing_core::{
    metadata::Metadata,
    span,
    subscriber::{Interest, Subscriber},
    Event,
};

use std::any::TypeId;

pub trait Layer<S>: 'static {
    /// Registers a new callsite with this layer, returning whether or not
    /// the subscriber is interested in being notified about the callsite.
    ///
    /// This function is provided with the `Interest` returned by the wrapped
    /// subscriber. The layer may then choose to return
    fn register_callsite(&self, _metadata: &'static Metadata<'static>, prev: Interest) -> Interest {
        prev
    }

    fn enabled(&self, _metadata: &Metadata, prev: bool, _ctx: Ctx<S>) -> bool {
        prev
    }

    fn new_span(&self, _attrs: &span::Attributes, _id: &span::Id, _ctx: Ctx<S>) {}

    // Note: it's unclear to me why we'd need the current span in `record` (the
    // only thing the `Ctx` type currently provides), but passing it in anyway
    // seems like a good future-proofing measure as it may grow other methods later...
    fn record(&self, _span: &span::Id, _values: &span::Record, _ctx: Ctx<S>) {}
    // Note: it's unclear to me why we'd need the current span in `record` (the
    // only thing the `Ctx` type currently provides), but passing it in anyway
    // seems like a good future-proofing measure as it may grow other methods later...
    fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id, _ctx: Ctx<S>) {}

    fn event(&self, _event: &Event, _ctx: Ctx<S>) {}
    fn enter(&self, _id: &span::Id, _ctx: Ctx<S>) {}
    fn exit(&self, _id: &span::Id, _ctx: Ctx<S>) {}

    /// Notifies this layer that the span with the given ID has been closed.
    fn close(&self, _id: span::Id, _ctx: Ctx<S>) {}

    /// Notifies this layer that a span ID has been cloned, and that the
    /// subscriber returned a different ID.
    fn change_id(&self, _old: &span::Id, _new: &span::Id, _ctx: Ctx<S>) {}

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
    // /// pub struct BarLayer {
    // ///     // ...
    // /// }
    // ///
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
    // ///     .and_then(BarLayer::new()
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
pub struct Ctx<'a, S> {
    subscriber: Option<&'a S>,
}

#[derive(Clone, Debug)]
pub struct Layered<L, S> {
    layer: L,
    inner: S,
}

// === impl Layered ===

impl<A, B> Layered<A, B> {
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
    /// pub struct BarLayer {
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
    pub fn and_then<C>(self, inner: C) -> Layered<A, Layered<B, C>> {
        let inner = Layered {
            layer: self.inner,
            inner,
        };
        Layered {
            layer: self.layer,
            inner,
        }
    }

    fn ctx(&self) -> Ctx<B> {
        Ctx {
            subscriber: Some(&self.inner),
        }
    }
}

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
        self.layer.record(span, values, self.ctx());
    }

    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {
        self.inner.record_follows_from(span, follows);
        self.layer.record_follows_from(span, follows, self.ctx());
    }

    fn event(&self, event: &Event) {
        self.inner.event(event);
        self.layer.event(event, self.ctx());
    }

    fn enter(&self, span: &span::Id) {
        self.inner.enter(span);
        self.layer.enter(span, self.ctx());
    }

    fn exit(&self, span: &span::Id) {
        self.inner.exit(span);
        self.layer.exit(span, self.ctx());
    }

    fn clone_span(&self, old: &span::Id) -> span::Id {
        let new = self.inner.clone_span(old);
        if &new != old {
            self.layer.change_id(old, &new, self.ctx())
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
            self.layer.close(id2, self.ctx());
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

// === impl SubscriberExt ===

impl<S: Subscriber> crate::sealed::Sealed for S {}
impl<S: Subscriber> SubscriberExt for S {}

// === impl Ctx ===

impl<'a, S: Subscriber> Ctx<'a, S> {
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
