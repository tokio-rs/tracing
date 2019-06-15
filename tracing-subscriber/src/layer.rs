use tokio_trace::{
    metadata::Metadata,
    span,
    subscriber::{Interest, Subscriber},
    Event,
};

use std::any::TypeId;

pub trait Layer<S: Subscriber>: 'static {
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

    fn and_then(self, inner: S) -> Layered<Self, S>
    where
        Self: Sized,
    {
        Layered { layer: self, inner }
    }
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
