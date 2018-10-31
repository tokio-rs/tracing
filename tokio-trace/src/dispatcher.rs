use {
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

    #[doc(hidden)]
    #[inline]
    pub fn id(&self) -> usize {
        self.id
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
    fn add_prior_span(
        &self,
        span: &span::Id,
        follows: span::Id,
    ) -> Result<(), subscriber::PriorError> {
        self.subscriber.add_prior_span(span, follows)
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
    fn enter(&self, span: span::Id, state: span::State) {
        self.subscriber.enter(span, state)
    }

    #[inline]
    fn exit(&self, span: span::Id, state: span::State) {
        self.subscriber.exit(span, state)
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

    fn add_prior_span(
        &self,
        _span: &span::Id,
        _follows: span::Id,
    ) -> Result<(), subscriber::PriorError> {
        Ok(())
    }

    fn enabled(&self, _metadata: &Meta) -> bool {
        false
    }

    fn observe_event<'event, 'meta: 'event>(&self, _event: &'event Event<'event, 'meta>) {
        // Do nothing.
    }

    fn enter(&self, _span: span::Id, _state: span::State) {}

    fn exit(&self, _span: span::Id, _state: span::State) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use {
        span::{self, State},
        subscriber,
    };

    #[test]
    fn dispatcher_is_sticky() {
        // Test ensuring that entire trace trees are collected by the same
        // dispatcher, even across dispatcher context switches.
        let subscriber1 = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")).with_state(State::Idle))
            .enter(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("bar")))
            .exit(span::mock().named(Some("bar")).with_state(State::Done))
            .exit(span::mock().named(Some("foo")).with_state(State::Done))
            .run();
        let foo = Dispatch::to(subscriber1).as_default(|| {
            let foo = span!("foo");
            foo.clone().enter(|| {});
            foo
        });
        Dispatch::to(subscriber::mock().run())
            .as_default(move || foo.enter(|| span!("bar").enter(|| {})))
    }

    #[test]
    fn dispatcher_isnt_too_sticky() {
        // Test ensuring that new trace trees are collected by the current
        // dispatcher.
        let subscriber1 = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")).with_state(State::Idle))
            .enter(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("bar")))
            .exit(span::mock().named(Some("bar")).with_state(State::Done))
            .exit(span::mock().named(Some("foo")).with_state(State::Done))
            .run();
        let subscriber2 = subscriber::mock()
            .enter(span::mock().named(Some("baz")))
            .enter(span::mock().named(Some("quux")))
            .exit(span::mock().named(Some("quux")).with_state(State::Done))
            .exit(span::mock().named(Some("baz")).with_state(State::Done))
            .run();

        let foo = Dispatch::to(subscriber1).as_default(|| {
            let foo = span!("foo");
            foo.clone().enter(|| {});
            foo
        });
        let baz = Dispatch::to(subscriber2).as_default(|| span!("baz"));
        Dispatch::to(subscriber::mock().run()).as_default(move || {
            foo.enter(|| span!("bar").enter(|| {}));
            baz.enter(|| span!("quux").enter(|| {}))
        })
    }

}
