use {
    callsite, field,
    subscriber::{self, Subscriber},
    Meta, Span,
};

use std::{
    fmt,
    sync::{Arc, Weak},
};

/// `Dispatch` trace data to a [`Subscriber`].
#[derive(Clone)]
pub struct Dispatch {
    subscriber: Arc<Subscriber + Send + Sync>,
}

pub(crate) struct Registrar(Weak<Subscriber + Send + Sync>);

impl Dispatch {
    /// Returns a new `Dispatch` that discards events and spans.
    pub fn none() -> Self {
        Dispatch {
            subscriber: Arc::new(NoSubscriber),
        }
    }

    /// Returns a `Dispatch` to the given [`Subscriber`](::Subscriber).
    pub fn new<S>(subscriber: S) -> Self
    // TODO: Add some kind of `UnsyncDispatch`?
    where
        S: Subscriber + Send + Sync + 'static,
    {
        let me = Dispatch {
            subscriber: Arc::new(subscriber),
        };
        callsite::register_dispatch(&me);
        me
    }

    pub(crate) fn registrar(&self) -> Registrar {
        Registrar(Arc::downgrade(&self.subscriber))
    }
}

impl fmt::Debug for Dispatch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad("Dispatch(...)")
    }
}

impl Subscriber for Dispatch {
    #[inline]
    fn register_callsite(&self, metadata: &Meta) -> subscriber::Interest {
        self.subscriber.register_callsite(metadata)
    }

    #[inline]
    fn new_static(&self, metadata: &'static Meta<'static>) -> Span {
        self.subscriber.new_static(metadata)
    }

    #[inline]
    fn new_span(&self, metadata: &Meta) -> Span {
        self.subscriber.new_span(metadata)
    }

    #[inline]
    fn record_i64(&self, span: &Span, field: &field::Key, value: i64) {
        self.subscriber.record_i64(span, field, value)
    }

    #[inline]
    fn record_u64(&self, span: &Span, field: &field::Key, value: u64) {
        self.subscriber.record_u64(span, field, value)
    }

    #[inline]
    fn record_bool(&self, span: &Span, field: &field::Key, value: bool) {
        self.subscriber.record_bool(span, field, value)
    }

    #[inline]
    fn record_str(&self, span: &Span, field: &field::Key, value: &str) {
        self.subscriber.record_str(span, field, value)
    }

    #[inline]
    fn record_fmt(&self, span: &Span, field: &field::Key, value: fmt::Arguments) {
        self.subscriber.record_fmt(span, field, value)
    }

    #[inline]
    fn add_follows_from(&self, span: &Span, follows: Span) {
        self.subscriber.add_follows_from(span, follows)
    }

    #[inline]
    fn enabled(&self, metadata: &Meta) -> bool {
        self.subscriber.enabled(metadata)
    }

    #[inline]
    fn enter(&self, span: &Span) {
        self.subscriber.enter(span)
    }

    #[inline]
    fn exit(&self, span: &Span) {
        self.subscriber.exit(span)
    }

    #[inline]
    fn clone_span(&self, id: &Span) -> Span {
        self.subscriber.clone_span(&id)
    }

    #[inline]
    fn drop_span(&self, id: Span) {
        self.subscriber.drop_span(id)
    }
}

struct NoSubscriber;
impl Subscriber for NoSubscriber {
    fn new_span(&self, _meta: &Meta) -> Span {
        Span::from_u64(0)
    }

    fn record_fmt(&self, _span: &Span, _key: &field::Key, _value: fmt::Arguments) {}

    fn add_follows_from(&self, _span: &Span, _follows: Span) {}

    fn enabled(&self, _metadata: &Meta) -> bool {
        false
    }

    fn enter(&self, _span: &Span) {}
    fn exit(&self, _span: &Span) {}
}

impl Registrar {
    pub(crate) fn try_register(&self, metadata: &Meta) -> Option<subscriber::Interest> {
        self.0.upgrade().map(|s| s.register_callsite(metadata))
    }
}
