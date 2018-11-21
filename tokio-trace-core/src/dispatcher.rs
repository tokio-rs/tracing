use {
    callsite, field,
    span::{self, Span},
    subscriber::{self, Subscriber},
    Id, Meta,
};

use std::{
    cell::RefCell,
    fmt,
    sync::{Arc, Weak},
    thread,
};

thread_local! {
    static CURRENT_DISPATCH: RefCell<Dispatch> = RefCell::new(Dispatch::none());
}

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

    pub(crate) fn with_current<T, F>(f: F) -> T
    where
        F: FnOnce(&Dispatch) -> T,
    {
        // If we try to access the current dispatcher while it's being
        // dropped, `LocalKey::with` would panic, causing a double panic.
        // However, we can't use `try_with` as we still need to invoke `f`,
        // which would be captured by the closure.
        if thread::panicking() {
            // It's better to fail to collect instrumentation than cause a
            // SIGSEGV.
            return f(&Dispatch::none());
        }
        CURRENT_DISPATCH.with(|current| f(&*current.borrow()))
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

    /// Sets this dispatch as the default for the duration of a closure.
    ///
    /// The default dispatcher is used when creating a new [`Span`] or
    /// [`Event`], _if no span is currently executing_. If a span is currently
    /// executing, new spans or events are dispatched to the subscriber that
    /// tagged that span, instead.
    ///
    /// [`Span`]: ::span::Span
    /// [`Subscriber`]: ::Subscriber
    /// [`Event`]: ::Event
    pub fn as_default<T>(&self, f: impl FnOnce() -> T) -> T {
        if thread::panicking() {
            return f();
        }
        CURRENT_DISPATCH.with(|current| {
            let prior = current.replace(self.clone());
            let result = f();
            *current.borrow_mut() = prior;
            result
        })
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
    fn new_span(&self, span: span::SpanAttributes) -> Id {
        self.subscriber.new_span(span)
    }

    #[inline]
    fn new_id(&self, span: span::Attributes) -> Id {
        self.subscriber.new_id(span)
    }

    #[inline]
    fn record_i64(&self, span: &Id, field: &field::Key, value: i64) {
        self.subscriber.record_i64(span, field, value)
    }

    #[inline]
    fn record_u64(&self, span: &Id, field: &field::Key, value: u64) {
        self.subscriber.record_u64(span, field, value)
    }

    #[inline]
    fn record_bool(&self, span: &Id, field: &field::Key, value: bool) {
        self.subscriber.record_bool(span, field, value)
    }

    #[inline]
    fn record_str(&self, span: &Id, field: &field::Key, value: &str) {
        self.subscriber.record_str(span, field, value)
    }

    #[inline]
    fn record_fmt(&self, span: &Id, field: &field::Key, value: fmt::Arguments) {
        self.subscriber.record_fmt(span, field, value)
    }

    #[inline]
    fn add_follows_from(&self, span: &Id, follows: Id) {
        self.subscriber.add_follows_from(span, follows)
    }

    #[inline]
    fn enabled(&self, metadata: &Meta) -> bool {
        self.subscriber.enabled(metadata)
    }

    #[inline]
    fn enter(&self, span: Span) -> Span {
        self.subscriber.enter(span)
    }

    #[inline]
    fn exit(&self, span: Id, parent: Span) -> Span {
        self.subscriber.exit(span, parent)
    }

    #[inline]
    fn current_span(&self) -> &Span {
        self.subscriber.current_span()
    }

    #[inline]
    fn close(&self, span: Id) {
        self.subscriber.close(span)
    }

    #[inline]
    fn clone_span(&self, id: Id) -> Id {
        self.subscriber.clone_span(id)
    }

    #[inline]
    fn drop_span(&self, id: Id) {
        self.subscriber.drop_span(id)
    }
}

struct NoSubscriber;
impl Subscriber for NoSubscriber {
    fn new_id(&self, _span: span::Attributes) -> Id {
        Id::from_u64(0)
    }

    fn record_fmt(&self, _span: &Id, _key: &field::Key, _value: fmt::Arguments) {}

    fn add_follows_from(&self, _span: &Id, _follows: Id) {}

    fn enabled(&self, _metadata: &Meta) -> bool {
        false
    }

    fn enter(&self, _span: Span) -> Span {
        Span::new_disabled()
    }

    fn current_span(&self) -> &Span {
        &Span::NONE
    }

    fn exit(&self, _exited: Id, _parent: Span) -> Span {
        Span::new_disabled()
    }

    fn close(&self, _span: Id) {}
}

impl Registrar {
    pub(crate) fn try_register(&self, metadata: &Meta) -> Option<subscriber::Interest> {
        self.0.upgrade().map(|s| s.register_callsite(metadata))
    }
}
