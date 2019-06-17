use trace::{
    metadata::Metadata,
    span,
    subscriber::{Interest, Subscriber},
    Event,
};


use crate::filter;
use std::any::TypeId;
pub trait Layer<S>: 'static {
    /// Registers a new callsite with this layer, returning whether or not
    /// the subscriber is interested in being notified about the callsite.
    ///
    /// This function is provided with the `Interest` returned by the wrapped
    /// subscriber. The layer may then choose to return
    fn register_callsite(&self, _metadata: &Metadata, prev: Interest) -> Interest {
        prev
    }
    fn enabled(&self, _metadata: &Metadata, prev: bool) -> bool {
        prev
    }
    fn new_span(&self, _attrs: &span::Attributes, _id: &span::Id) {}
    fn record(&self, _span: &span::Id, _values: &span::Record) {}
    fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {}
    fn event(&self, _event: &Event) {}
    fn enter(&self, _id: &span::Id) {}
    fn exit(&self, _id: &span::Id) {}
    fn clone_span(&self, _id: &span::Id, _new: Option<&span::Id>) {}
    fn drop_span(&self, _id: &span::Id) {}

    /// Composes the given [`Subscriber`] with this `Layer`, returning a `Layered` subscriber.
    ///
    /// The returned `Layered` subscriber will call the methods on this `Layer`
    /// and then those of the wrapped subscriber. Multiple layers may be
    /// composed in this manner. For example:
    /// ```rust
    /// # extern crate tokio_trace_core;
    /// # extern crate tokio_trace_subscriber;
    /// # use tokio_trace_subscriber::layer::Layer;
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
    /// # use tokio_trace_core::{span::{Id, Attributes, Record}, Metadata};
    /// # impl tokio_trace_core::Subscriber for MySubscriber {
    /// #   fn new_span(&self, _: &Attributes) -> Id { Id::from_u64(0) }
    /// #   fn record(&self, _: &Id, _: &Record) {}
    /// #   fn event(&self, _: &tokio_trace_core::Event) {}
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
    /// # extern crate tokio_trace_core;
    /// # extern crate tokio_trace_subscriber;
    /// # use tokio_trace_subscriber::layer::Layer;
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
    /// # use tokio_trace_core::{span::{Id, Attributes, Record}, Metadata};
    /// # impl tokio_trace_core::Subscriber for MySubscriber {
    /// #   fn new_span(&self, _: &Attributes) -> Id { Id::from_u64(0) }
    /// #   fn record(&self, _: &Id, _: &Record) {}
    /// #   fn event(&self, _: &tokio_trace_core::Event) {}
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
}

impl<L, S> Subscriber for Layered<L, S>
where
    L: Layer<S>,
    S: Subscriber,
{
    fn register_callsite(&self, metadata: &Metadata) -> Interest {
        let interest = self.inner.register_callsite(metadata);
        self.layer.register_callsite(metadata, interest)
    }

    fn enabled(&self, metadata: &Metadata) -> bool {
        let enabled = self.inner.enabled(metadata);
        self.layer.enabled(metadata, enabled)
    }

    fn new_span(&self, span: &span::Attributes) -> span::Id {
        let id = self.inner.new_span(span);
        self.layer.new_span(span, &id);
        id
    }

    fn record(&self, span: &span::Id, values: &span::Record) {
        self.inner.record(span, values);
        self.layer.record(span, values);
    }

    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {
        self.inner.record_follows_from(span, follows);
        self.layer.record_follows_from(span, follows);
    }

    fn event(&self, event: &Event) {
        self.inner.event(event);
        self.layer.event(event);
    }

    fn enter(&self, span: &span::Id) {
        self.inner.enter(span);
        self.layer.enter(span);
    }

    fn exit(&self, span: &span::Id) {
        self.inner.exit(span);
        self.layer.exit(span);
    }

    fn clone_span(&self, old: &span::Id) -> span::Id {
        let new = self.inner.clone_span(old);
        if &new != old {
            self.layer.clone_span(old, Some(&new));
        } else {
            self.layer.clone_span(old, None);
        };
        new
    }

    fn drop_span(&self, id: span::Id) {
        self.layer.drop_span(&id);
        self.inner.drop_span(id);
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
