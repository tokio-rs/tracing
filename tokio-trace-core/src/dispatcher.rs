use {
    callsite,
    span::{self, Span},
    subscriber::{self, Subscriber},
    Event, IntoValue, Meta,
};

use std::{
    cell::RefCell,
    default::Default,
    fmt,
    sync::{
        atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT},
        Arc,
    },
};

thread_local! {
    static CURRENT_DISPATCH: RefCell<Dispatch> = RefCell::new(Dispatch::none());
}

/// `Dispatch` trace data to a [`Subscriber`].
#[derive(Clone)]
pub struct Dispatch {
    subscriber: Arc<dyn Subscriber + Send + Sync>,
    id: usize,
}

impl Dispatch {
    /// Returns a new `Dispatch` that discards events and spans.
    pub fn none() -> Self {
        Dispatch {
            subscriber: Arc::new(NoSubscriber),
            id: 0,
        }
    }

    /// Returns the subscriber that a new [`Span`] or [`Event`] would dispatch
    /// to.
    ///
    /// This returns a `Dispatch` to the [`Subscriber`] that created the
    /// current [`Span`], or the thread's default subscriber if no
    /// span is currently executing.
    ///
    /// [`Span`]: ::span::Span
    /// [`Subscriber`]: ::Subscriber
    /// [`Event`]: ::Event
    pub fn current() -> Dispatch {
        Span::current().dispatch().cloned().unwrap_or_default()
    }

    /// Returns a `Dispatch` to the given [`Subscriber`](::Subscriber).
    pub fn to<S>(subscriber: S) -> Self
    // TODO: Add some kind of `UnsyncDispatch`?
    where
        S: Subscriber + Send + Sync + 'static,
    {
        static GEN: AtomicUsize = ATOMIC_USIZE_INIT;
        Dispatch {
            subscriber: Arc::new(subscriber),
            id: GEN.fetch_add(1, Ordering::AcqRel),
        }
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
        CURRENT_DISPATCH.with(|current| {
            let prior = current.replace(self.clone());
            let result = f();
            *current.borrow_mut() = prior;
            result
        })
    }

    /// Performs an operation if a callsite is enabled.
    ///
    /// If the given callsite is enabled for this subscriber, this function
    /// calls the given closure with the dispatcher and the callsite's metadata,
    /// and returns the result. Otherwise, it returns `None`.
    #[inline]
    pub fn if_enabled<F, T>(self, callsite: &callsite::Callsite, f: F) -> Option<T>
    where
        F: FnOnce(Dispatch, &'static Meta<'static>) -> T,
    {
        if self.is_enabled(callsite) {
            return Some(f(self, callsite.metadata()));
        }

        None
    }

    #[inline]
    fn is_invalid(&self, callsite: &callsite::Callsite) -> bool {
        callsite.0.with(|cache| {
            // If the callsite was last filtered by a different subscriber, assume
            // the filter is no longer valid.
            if cache.cached_filter.get().is_none() || cache.last_filtered_by.get() != self.id {
                // Update the stamp on the call site so this subscriber is now the
                // last to filter it.
                cache.last_filtered_by.set(self.id);
                return true;
            }

            // Otherwise, just ask the subscriber what it thinks.
            self.subscriber.should_invalidate_filter(&cache.meta)
        })
    }

    #[inline]
    fn is_enabled(&self, callsite: &callsite::Callsite) -> bool {
        if self.is_invalid(callsite) {
            callsite.0.with(|cache| {
                let enabled = self.subscriber.enabled(&cache.meta);
                cache.cached_filter.set(Some(enabled));
                enabled
            })
        } else if let Some(cached) = callsite.0.with(|cache| cache.cached_filter.get()) {
            cached
        } else {
            callsite.0.with(|cache| {
                let enabled = self.subscriber.enabled(&cache.meta);
                cache.cached_filter.set(Some(enabled));
                enabled
            })
        }
    }
}

impl fmt::Debug for Dispatch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad("Dispatch(...)")
    }
}

impl Default for Dispatch {
    fn default() -> Self {
        CURRENT_DISPATCH.with(|current| current.borrow().clone())
    }
}

impl Subscriber for Dispatch {
    #[inline]
    fn new_span(&self, span: span::Data) -> span::Id {
        self.subscriber.new_span(span)
    }

    #[inline]
    fn should_invalidate_filter(&self, metadata: &Meta) -> bool {
        self.subscriber.should_invalidate_filter(metadata)
    }

    #[inline]
    fn add_value(
        &self,
        span: &span::Id,
        name: &'static str,
        value: &dyn IntoValue,
    ) -> Result<(), subscriber::AddValueError> {
        self.subscriber.add_value(span, name, value)
    }

    #[inline]
    fn add_follows_from(
        &self,
        span: &span::Id,
        follows: span::Id,
    ) -> Result<(), subscriber::FollowsError> {
        self.subscriber.add_follows_from(span, follows)
    }

    #[inline]
    fn enabled(&self, metadata: &Meta) -> bool {
        self.subscriber.enabled(metadata)
    }

    #[inline]
    fn observe_event<'event, 'meta: 'event>(&self, event: &'event Event<'event, 'meta>) {
        self.subscriber.observe_event(event)
    }

    #[inline]
    fn enter(&self, span: span::Id) {
        self.subscriber.enter(span)
    }

    #[inline]
    fn exit(&self, span: span::Id) {
        self.subscriber.exit(span)
    }

    #[inline]
    fn close(&self, span: span::Id) {
        self.subscriber.close(span)
    }
}

struct NoSubscriber;

impl Subscriber for NoSubscriber {
    fn new_span(&self, _span: span::Data) -> span::Id {
        span::Id::from_u64(0)
    }

    fn add_value(
        &self,
        _span: &span::Id,
        _name: &'static str,
        _value: &dyn IntoValue,
    ) -> Result<(), subscriber::AddValueError> {
        Ok(())
    }

    fn add_follows_from(
        &self,
        _span: &span::Id,
        _follows: span::Id,
    ) -> Result<(), subscriber::FollowsError> {
        Ok(())
    }

    fn enabled(&self, _metadata: &Meta) -> bool {
        false
    }

    fn observe_event<'event, 'meta: 'event>(&self, _event: &'event Event<'event, 'meta>) {
        // Do nothing.
    }

    fn enter(&self, _span: span::Id) {}

    fn exit(&self, _span: span::Id) {}

    fn close(&self, _span: span::Id) {}
}
