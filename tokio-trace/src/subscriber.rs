use super::{span, Event, SpanData, Meta};

pub trait Subscriber {
    /// Determines if a span or event with the specified metadata would be recorded.
    ///
    /// This is used by the dispatcher to avoid allocating for span construction
    /// if the span would be discarded anyway.
    fn enabled(&self, metadata: &Meta) -> bool;

    /// Record the construction of a new [`Span`], returning a a new [span ID] for
    /// the span being constructed.
    ///
    /// Span IDs are used to uniquely identify spans, so span equality will be
    /// based on the returned ID. Thus, if the subscriber wishes for all spans
    /// with the same metadata to be considered equal, it should return the same
    /// ID every time it is given a particular set of metadata. Similarly, if it
    /// wishes for two separate instances of a span with the same metadata to *not*
    /// be equal, it should return a distinct ID every time this function is called,
    /// regardless of the metadata.
    ///
    /// Subscribers which do not rely on the implementations of `PartialEq`,
    /// `Eq`, and `Hash` for `Span`s are free to return span IDs with value 0
    /// from all calls to this function, if they so choose.
    ///
    /// [span ID]: ../span/struct.Id.html
    fn new_span(&self, new_span: &span::NewSpan) -> span::Id;

    /// Returns `true` if the cached result to a call to `enabled` for a span
    /// with the given metadata is still valid.
    ///
    /// By default, this function assumes that cached filter results will remain
    /// valid, but should be overridden when this is not the case.
    ///
    /// If this returns `false`, then the prior value may be used.
    /// `Subscriber`s which require their filters to be run every time an event
    /// occurs or a span is entered/exited should always return `true`.
    ///
    /// For example, suppose a sampling subscriber is implemented by incrementing a
    /// counter every time `enabled` is called and only returning `true` when
    /// the counter is divisible by a specified sampling rate. If that
    /// subscriber returns `false` from `should_invalidate_filter`, then the
    /// filter will not be re-evaluated once it has been applied to a given set
    /// of metadata. Thus, the counter will not be incremented, and the span or
    /// event that correspands to the metadata will never be `enabled`.
    ///
    /// Similarly, if a `Subscriber` has a filtering strategy that can be
    /// changed dynamically at runtime, it would need to invalidate any cached
    /// filter results when the filtering rules change.
    ///
    /// A subscriber which manages fanout to multiple other subscribers should
    /// proxy this decision to all of its child subscribers, returning `false`
    /// only if _all_ such children return `false`. If the set of subscribers to
    /// which spans are broadcast may change dynamically, adding a new
    /// subscriber should also invalidate cached filters.
    fn should_invalidate_filter(&self, metadata: &Meta) -> bool {
        false
    }

    /// Note that this function is generic over a pair of lifetimes because the
    /// `Event` type is. See the documentation for [`Event`] for details.
    ///
    /// [`Event`]: ../struct.Event.html
    fn observe_event<'event, 'meta: 'event>(&self, event: &'event Event<'event, 'meta>);
    fn enter(&self, span: &SpanData);
    fn exit(&self, span: &SpanData);

    /// Composes `self` with a `filter` that returns true or false if a span or
    /// event with the specified metadata should be recorded.
    ///
    /// This function is intended to be used with composing subscribers from
    /// external crates with user-defined filters, so that the resulting
    /// subscriber is [`enabled`] only for a subset of the events and spans for
    /// which the original subscriber would be enabled.
    ///
    /// For example:
    /// ```
    /// #[macro_use]
    /// extern crate tokio_trace;
    /// extern crate tokio_trace_log;
    /// use tokio_trace::subscriber::Subscriber;
    /// # use tokio_trace::Level;
    /// # fn main() {
    ///
    /// let filtered_subscriber = tokio_trace_log::LogSubscriber::new()
    ///     // Subscribe *only* to spans named "foo".
    ///     .with_filter(|meta| {
    ///         meta.name == Some("foo")
    ///     });
    /// tokio_trace::Dispatcher::builder()
    ///     .add_subscriber(filtered_subscriber)
    ///     .try_init();
    ///
    /// // This span will be logged.
    /// span!("foo", enabled = true) .enter(|| {
    ///     // do work;
    /// });
    /// // This span will *not* be logged.
    /// span!("bar", enabled = false).enter(|| {
    ///     // This event also will not be logged.
    ///     event!(Level::Debug, { enabled = false },"this won't be logged");
    /// });
    /// # }
    /// ```
    ///
    /// [`enabled`]: #tymethod.enabled
    fn with_filter<F>(self, filter: F) -> WithFilter<Self, F>
    where
        F: Fn(&Meta) -> bool,
        Self: Sized,
    {
        WithFilter {
            inner: self,
            filter,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WithFilter<S, F> {
    inner: S,
    filter: F
}

impl<S, F> Subscriber for WithFilter<S, F>
where
    S: Subscriber,
    F: Fn(&Meta) -> bool,
{
    fn enabled(&self, metadata: &Meta) -> bool {
        (self.filter)(metadata) && self.inner.enabled(metadata)
    }

    fn new_span(&self, new_span: &span::NewSpan) -> span::Id {
        self.inner.new_span(&new_span)
    }

    #[inline]
    fn observe_event<'event, 'meta: 'event>(&self, event: &'event Event<'event, 'meta>) {
        self.inner.observe_event(event)
    }

    #[inline]
    fn enter(&self, span: &SpanData) {
        self.inner.enter(span)
    }

    #[inline]
    fn exit(&self, span: &SpanData) {
        self.inner.exit(span)
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use self::test_support::*;

#[cfg(any(test, feature = "test-support"))]
mod test_support {
    use super::Subscriber;
    use ::{Event, SpanData, Meta};
    use ::span::{self, MockSpan};

    use std::{
        cell::RefCell,
        collections::VecDeque,
        thread,
        sync::atomic::{AtomicUsize, Ordering},
    };

    struct ExpectEvent {
        // TODO: implement
    }

    enum Expect {
        Event(ExpectEvent),
        Enter(MockSpan),
        Exit(MockSpan),
    }

    struct Running {
        expected: RefCell<VecDeque<Expect>>,
        ids: AtomicUsize,
    }

    pub struct MockSubscriber {
        expected: VecDeque<Expect>,
    }

    pub fn mock() -> MockSubscriber {
        MockSubscriber {
            expected: VecDeque::new(),
        }
    }

    // hack so each test thread can run its own mock subscriber, even though the
    // global dispatcher is static for the lifetime of the whole test binary.
    pub struct MockDispatch {}

    thread_local! {
        static MOCK_SUBSCRIBER: RefCell<Option<Box<dyn Subscriber>>> = RefCell::new(None);
    }

    impl MockSubscriber {
        pub fn enter(mut self, span: MockSpan) -> Self {
            self.expected.push_back(Expect::Enter(span
                .with_state(::span::State::Running)));
            self
        }

        pub fn exit(mut self, span: MockSpan) -> Self {
            self.expected.push_back(Expect::Exit(span));
            self
        }

        pub fn to_subscriber(self) -> impl Subscriber {
            Running {
                expected: RefCell::new(self.expected),
                ids: AtomicUsize::new(0),
            }
        }

        pub fn run(self) {
            MockDispatch::run(self.to_subscriber());
        }
    }

    impl Subscriber for Running {
        fn enabled(&self, _meta: &Meta) -> bool {
            // TODO: allow the mock subscriber to filter events for testing filtering?
            true
        }

        fn new_span(&self, _: &span::NewSpan) -> span::Id {
            span::Id::from_u64(self.ids.fetch_add(1, Ordering::SeqCst) as u64)
        }

        fn observe_event<'event, 'meta: 'event>(&self, _event: &'event Event<'event, 'meta>) {
            match self.expected.borrow_mut().pop_front() {
                None => {}
                Some(Expect::Event(_)) => unimplemented!(),
                Some(Expect::Enter(expected_span)) => panic!("expected to enter span {:?}, but got an event", expected_span.name),
                Some(Expect::Exit(expected_span)) => panic!("expected to exit span {:?} but got an event", expected_span.name),
            }
        }

        fn enter(&self, span: &SpanData) {
            println!("+ {}: {:?}", thread::current().name().unwrap_or("unknown thread"), span);
            match self.expected.borrow_mut().pop_front() {
                None => {},
                Some(Expect::Event(_)) => panic!("expected an event, but entered span {:?} instead", span.name()),
                Some(Expect::Enter(expected_span)) => {
                    if let Some(name) = expected_span.name {
                        assert_eq!(name, span.name());
                    }
                    if let Some(state) = expected_span.state {
                        assert_eq!(state, span.state());
                    }
                    // TODO: expect fields
                }
                Some(Expect::Exit(expected_span)) => panic!(
                    "expected to exit span {:?}, but entered span {:?} instead",
                    expected_span.name,
                    span.name()),
            }
        }

        fn exit(&self, span: &SpanData) {
            println!("- {}: {:?}", thread::current().name().unwrap_or("unknown_thread"), span);
            match self.expected.borrow_mut().pop_front() {
                None => {},
                Some(Expect::Event(_)) => panic!("expected an event, but exited span {:?} instead", span.name()),
                Some(Expect::Enter(expected_span)) => panic!(
                    "expected to enter span {:?}, but exited span {:?} instead",
                    expected_span.name,
                    span.name()),
                Some(Expect::Exit(expected_span)) => {
                    if let Some(name) = expected_span.name {
                        assert_eq!(name, span.name());
                    }
                    if let Some(state) = expected_span.state {
                        assert_eq!(state, span.state());
                    }
                    // TODO: expect fields
                }
            }
        }
    }

    impl Subscriber for MockDispatch {
        fn enabled(&self, meta: &Meta) -> bool {
            MOCK_SUBSCRIBER.with(|mock| {
                if let Some(ref subscriber) = *mock.borrow() {
                    subscriber.enabled(meta)
                } else {
                    false
                }
            })
        }

        fn new_span(&self, new_span: &span::NewSpan) -> span::Id {
            MOCK_SUBSCRIBER.with(|mock| {
                mock.borrow()
                    .as_ref()
                    .map(|subscriber| subscriber.new_span(new_span))
                    .unwrap_or_else(|| span::Id::from_u64(0))
            })
        }

        fn observe_event<'event, 'meta: 'event>(&self, event: &'event Event<'event, 'meta>) {
            MOCK_SUBSCRIBER.with(|mock| {
                if let Some(ref subscriber) = *mock.borrow() {
                    subscriber.observe_event(event)
                }
            })
        }

        #[inline]
        fn enter(&self, span: &SpanData) {
            MOCK_SUBSCRIBER.with(|mock| {
                if let Some(ref subscriber) = *mock.borrow() {
                    subscriber.enter(span)
                }
            })
        }

        #[inline]
        fn exit(&self, span: &SpanData) {
            MOCK_SUBSCRIBER.with(|mock| {
                if let Some(ref subscriber) = *mock.borrow() {
                    subscriber.exit(span)
                }
            })
        }
    }

    impl MockDispatch {
        pub fn run<T: Subscriber + Sized + 'static>(subscriber: T) {
            // don't care if this succeeds --- another test may have already
            // installed the test dispatcher.
            let _ = ::Dispatcher::builder()
                .add_subscriber(MockDispatch {})
                .try_init();
            MOCK_SUBSCRIBER.with(move |mock| {
                *mock.borrow_mut() = Some(Box::new(subscriber));
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use ::{
        span,
        subscriber::{self, Subscriber},
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[test]
    fn filters_are_not_reevaluated_for_the_same_span() {
        // Asserts that the `span!` macro caches the result of calling
        // `Subscriber::enabled` for each span.
        let foo_count = Arc::new(AtomicUsize::new(0));
        let bar_count = Arc::new(AtomicUsize::new(0));
        let foo_count2 = foo_count.clone();
        let bar_count2 = bar_count.clone();

        let subscriber = subscriber::mock()
            .enter(span::mock().named(Some("bar")))
            .exit(span::mock().named(Some("bar")))
            .enter(span::mock().named(Some("bar")))
            .exit(span::mock().named(Some("bar")))
            .enter(span::mock().named(Some("bar")))
            .exit(span::mock().named(Some("bar")))
            .to_subscriber()
            .with_filter(move |meta| match meta.name {
                Some("foo") => {
                    foo_count2.fetch_add(1, Ordering::Relaxed);
                    false
                },
                Some("bar") => {
                    bar_count2.fetch_add(1, Ordering::Relaxed);
                    true
                },
                _ => false,
            });
        subscriber::MockDispatch::run(subscriber);


        // Enter "foo" and then "bar". The dispatcher expects to see "bar" but
        // not "foo."
        let foo = span!("foo");
        let bar = foo.clone().enter(|| {
            let bar = span!("bar");
            bar.clone().enter(|| { bar })
        });

        // The filter should have seen each span a single time.
        assert_eq!(foo_count.load(Ordering::Relaxed), 1);
        assert_eq!(bar_count.load(Ordering::Relaxed), 1);

        foo.clone().enter(|| {
            bar.clone().enter(|| { })
        });

        // The subscriber should see "bar" again, but the filter should not have
        // been called.
        assert_eq!(foo_count.load(Ordering::Relaxed), 1);
        assert_eq!(bar_count.load(Ordering::Relaxed), 1);

        bar.clone().enter(|| { });
        assert_eq!(foo_count.load(Ordering::Relaxed), 1);
        assert_eq!(bar_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn filters_are_reevaluated_for_different_call_sites() {
        // Asserts that the `span!` macro caches the result of calling
        // `Subscriber::enabled` for each span.
        let foo_count = Arc::new(AtomicUsize::new(0));
        let bar_count = Arc::new(AtomicUsize::new(0));
        let foo_count2 = foo_count.clone();
        let bar_count2 = bar_count.clone();

        let subscriber = subscriber::mock()
            .enter(span::mock().named(Some("bar")))
            .exit(span::mock().named(Some("bar")))
            .enter(span::mock().named(Some("bar")))
            .exit(span::mock().named(Some("bar")))
            .enter(span::mock().named(Some("bar")))
            .exit(span::mock().named(Some("bar")))
            .to_subscriber()
            .with_filter(move |meta| match meta.name {
                Some("foo") => {
                    foo_count2.fetch_add(1, Ordering::Relaxed);
                    false
                },
                Some("bar") => {
                    bar_count2.fetch_add(1, Ordering::Relaxed);
                    true
                },
                _ => false,
            });
        subscriber::MockDispatch::run(subscriber);


        // Enter "foo" and then "bar". The dispatcher expects to see "bar" but
        // not "foo."
        let foo = span!("foo");
        let bar = foo.clone().enter(|| {
            let bar = span!("bar");
            bar.clone().enter(|| { bar })
        });

        // The filter should have seen each span a single time.
        assert_eq!(foo_count.load(Ordering::Relaxed), 1);
        assert_eq!(bar_count.load(Ordering::Relaxed), 1);

        foo.clone().enter(|| {
            bar.clone().enter(|| { })
        });

        // The subscriber should see "bar" again, but the filter should not have
        // been called.
        assert_eq!(foo_count.load(Ordering::Relaxed), 1);
        assert_eq!(bar_count.load(Ordering::Relaxed), 1);

        // A different span with the same name has a different call site, so it
        // should cause the filter to be reapplied.
        let foo2 = span!("foo");
        foo.clone().enter(|| { });
        assert_eq!(foo_count.load(Ordering::Relaxed), 2);
        assert_eq!(bar_count.load(Ordering::Relaxed), 1);

        // But, the filter should not be re-evaluated for the new "foo" span
        // when it is re-entered.
        foo2.enter(|| { span!("bar").enter(|| { }) });
        assert_eq!(foo_count.load(Ordering::Relaxed), 2);
        assert_eq!(bar_count.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn filter_caching_is_lexically_scoped() {
        pub fn my_great_function() -> bool {
            span!("foo").enter(|| {
                true
            })
        }

        pub fn my_other_function() -> bool {
            span!("bar").enter(|| {
                true
            })
        }

        let count = Arc::new(AtomicUsize::new(0));
        let count2 = count.clone();

        let subscriber = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("bar")))
            .exit(span::mock().named(Some("bar")))
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("bar")))
            .exit(span::mock().named(Some("bar")))
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .to_subscriber()
            .with_filter(move |_meta| {
                count2.fetch_add(1, Ordering::Relaxed);
                true
            });
        subscriber::MockDispatch::run(subscriber);

        // Call the function once. The filter should be re-evaluated.
        assert!(my_great_function());
        assert_eq!(count.load(Ordering::Relaxed), 1);

        // Call the function again. The cached result should be used.
        assert!(my_great_function());
        assert_eq!(count.load(Ordering::Relaxed), 1);

        assert!(my_other_function());
        assert_eq!(count.load(Ordering::Relaxed), 2);

        assert!(my_great_function());
        assert_eq!(count.load(Ordering::Relaxed), 2);

        assert!(my_other_function());
        assert_eq!(count.load(Ordering::Relaxed), 2);

        assert!(my_great_function());
        assert_eq!(count.load(Ordering::Relaxed), 2);


    }
}
