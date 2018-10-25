use {span, Event, IntoValue, Meta, SpanId};

pub trait Subscriber {
    // === Span registry methods ==============================================

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
    fn new_span(&self, span: span::Data) -> span::Id;

    // // XXX: should this be a subscriber method or should it have its own type???
    // fn span_data(&self, id: &span::Id) -> Option<&span::Data>;

    /// Adds a new field to an existing span observed by this `Subscriber`.
    ///
    /// This is expected to return an error under the following conditions:
    /// - The span ID does not correspond to a span which currently exists.
    /// - The span does not have a field with the given name.
    /// - The span has a field with the given name, but the value has already
    ///   been set.
    fn add_value(
        &self,
        span: &span::Id,
        name: &'static str,
        value: &dyn IntoValue,
    ) -> Result<(), AddValueError>;

    /// Adds an indication that `span` follows from the span with the id
    /// `follows`.
    ///
    /// This relationship differs somewhat from the parent-child relationship: a
    /// span may have any number of prior spans, rather than a single one; and
    /// spans are not considered to be executing _inside_ of the spans they
    /// follow from. This means that a span may close even if subsequent spans
    /// that follow from it are still open, and time spent inside of a
    /// subsequent span should not be included in the time its precedents were
    /// executing. This is used to model causal relationships such as when a
    /// single future spawns several related background tasks, et cetera.
    ///
    /// If the subscriber has spans corresponding to the given IDs, it should
    /// record this relationship in whatever way it deems necessary. Otherwise,
    /// if one or both of the given span IDs do not correspond to spans that the
    /// subscriber knows about, or if a cyclical relationship would be created
    /// (i.e., some span _a_ which proceeds some other span _b_ may not also
    /// follow from _b_), it should return a `PriorError`.
    fn add_prior_span(&self, span: &span::Id, follows: span::Id) -> Result<(), PriorError>;

    // === Filtering methods ==================================================

    /// Determines if a span or event with the specified metadata would be recorded.
    ///
    /// This is used by the dispatcher to avoid allocating for span construction
    /// if the span would be discarded anyway.
    fn enabled(&self, metadata: &Meta) -> bool;

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
    fn should_invalidate_filter(&self, _metadata: &Meta) -> bool {
        false
    }

    // === Notification methods ===============================================

    /// Note that this function is generic over a pair of lifetimes because the
    /// `Event` type is. See the documentation for [`Event`] for details.
    ///
    /// [`Event`]: ../struct.Event.html
    fn observe_event<'event, 'meta: 'event>(&self, event: &'event Event<'event, 'meta>);
    fn enter(&self, span: SpanId, state: span::State);
    fn exit(&self, span: SpanId, state: span::State);
}

// TODO: before releasing core 0.1 this needs to be made private, to avoid
// future breaking changes.
#[derive(Clone, Debug)]
pub enum AddValueError {
    /// The span with the given ID does not exist.
    NoSpan,
    /// The span exists, but does not have the specified field.
    NoField,
    /// The named field already has a value.
    FieldAlreadyExists,
}

// TODO: before releasing core 0.1 this needs to be made private, to avoid
// future breaking changes.
#[derive(Clone, Debug)]
pub enum PriorError {
    /// The span with the given ID does not exist.
    /// TODO: can this error type be generalized between `PriorError` and
    /// `AddValueError`?
    NoSpan(SpanId),
    /// The span that this span follows from does not exist (it has no ID).
    NoPreceedingId,
}

#[cfg(any(test, feature = "test-support"))]
pub use self::test_support::*;

#[cfg(any(test, feature = "test-support"))]
mod test_support {
    use super::*;
    use span::{self, MockSpan};
    use {Event, IntoValue, Meta, SpanData, SpanId};

    use std::{
        collections::{HashMap, VecDeque},
        sync::{
            atomic::{AtomicUsize, Ordering},
            Mutex,
        },
    };

    struct ExpectEvent {
        // TODO: implement
    }

    enum Expect {
        Event(ExpectEvent),
        Enter(MockSpan),
        Exit(MockSpan),
    }

    struct Running<F: Fn(&Meta) -> bool> {
        spans: Mutex<HashMap<SpanId, SpanData>>,
        expected: Mutex<VecDeque<Expect>>,
        ids: AtomicUsize,
        filter: F,
    }

    pub struct MockSubscriber<F: Fn(&Meta) -> bool> {
        expected: VecDeque<Expect>,
        filter: F,
    }

    pub fn mock() -> MockSubscriber<fn(&Meta) -> bool> {
        MockSubscriber {
            expected: VecDeque::new(),
            filter: (|_: &Meta| true) as for<'r, 's> fn(&'r Meta<'s>) -> _,
        }
    }

    impl<F: Fn(&Meta) -> bool> MockSubscriber<F> {
        pub fn enter(mut self, span: MockSpan) -> Self {
            self.expected
                .push_back(Expect::Enter(span.with_state(::span::State::Running)));
            self
        }

        pub fn exit(mut self, span: MockSpan) -> Self {
            self.expected.push_back(Expect::Exit(span));
            self
        }

        pub fn with_filter<G>(self, filter: G) -> MockSubscriber<G>
        where
            G: Fn(&Meta) -> bool,
        {
            MockSubscriber {
                filter,
                expected: self.expected,
            }
        }

        pub fn run(self) -> impl Subscriber {
            Running {
                spans: Mutex::new(HashMap::new()),
                expected: Mutex::new(self.expected),
                ids: AtomicUsize::new(0),
                filter: self.filter,
            }
        }
    }

    impl<F: Fn(&Meta) -> bool> Subscriber for Running<F> {
        fn enabled(&self, meta: &Meta) -> bool {
            (self.filter)(meta)
        }

        fn add_value(
            &self,
            _span: &span::Id,
            _name: &'static str,
            _value: &dyn IntoValue,
        ) -> Result<(), AddValueError> {
            // TODO: it should be possible to expect values...
            Ok(())
        }

        fn add_prior_span(
            &self,
            _span: &span::Id,
            _follows: span::Id,
        ) -> Result<(), PriorError> {
            // TODO: it should be possible to expect spans to follow from other spans
            Ok(())
        }

        fn new_span(&self, span: SpanData) -> span::Id {
            let id = self.ids.fetch_add(1, Ordering::SeqCst);
            let id = span::Id::from_u64(id as u64);
            self.spans.lock().unwrap().insert(id.clone(), span);
            id
        }

        fn observe_event<'event, 'meta: 'event>(&self, _event: &'event Event<'event, 'meta>) {
            match self.expected.lock().unwrap().pop_front() {
                None => {}
                Some(Expect::Event(_)) => unimplemented!(),
                Some(Expect::Enter(expected_span)) => panic!(
                    "expected to enter span {:?}, but got an event",
                    expected_span.name
                ),
                Some(Expect::Exit(expected_span)) => panic!(
                    "expected to exit span {:?} but got an event",
                    expected_span.name
                ),
            }
        }

        fn enter(&self, span: span::Id, state: span::State) {
            let spans = self.spans.lock().unwrap();
            let span = spans
                .get(&span)
                .unwrap_or_else(|| panic!("no span for ID {:?}", span));
            match self.expected.lock().unwrap().pop_front() {
                None => {}
                Some(Expect::Event(_)) => panic!(
                    "expected an event, but entered span {:?} instead",
                    span.name()
                ),
                Some(Expect::Enter(expected_span)) => {
                    if let Some(name) = expected_span.name {
                        assert_eq!(name, span.name());
                    }
                    if let Some(expected_state) = expected_span.state {
                        assert_eq!(expected_state, state);
                    }
                    // TODO: expect fields
                }
                Some(Expect::Exit(expected_span)) => panic!(
                    "expected to exit span {:?}, but entered span {:?} instead",
                    expected_span.name,
                    span.name()
                ),
            }
        }

        fn exit(&self, span: span::Id, state: span::State) {
            let spans = self.spans.lock().unwrap();
            let span = spans
                .get(&span)
                .unwrap_or_else(|| panic!("no span for ID {:?}", span));
            match self.expected.lock().unwrap().pop_front() {
                None => {}
                Some(Expect::Event(_)) => panic!(
                    "expected an event, but exited span {:?} instead",
                    span.name()
                ),
                Some(Expect::Enter(expected_span)) => panic!(
                    "expected to enter span {:?}, but exited span {:?} instead",
                    expected_span.name,
                    span.name()
                ),
                Some(Expect::Exit(expected_span)) => {
                    if let Some(name) = expected_span.name {
                        assert_eq!(name, span.name());
                    }
                    if let Some(expected_state) = expected_span.state {
                        assert_eq!(expected_state, state);
                    }
                    // TODO: expect fields
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Barrier,
        },
        thread,
    };
    use {span, subscriber, Dispatch, Span};

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
            .with_filter(move |meta| match meta.name {
                Some("foo") => {
                    foo_count2.fetch_add(1, Ordering::Relaxed);
                    false
                }
                Some("bar") => {
                    bar_count2.fetch_add(1, Ordering::Relaxed);
                    true
                }
                _ => false,
            }).run();

        Dispatch::to(subscriber).with(move || {
            // Enter "foo" and then "bar". The dispatcher expects to see "bar" but
            // not "foo."
            let foo = span!("foo");
            let bar = foo.clone().enter(|| {
                let bar = span!("bar");
                bar.clone().enter(|| bar)
            });

            // The filter should have seen each span a single time.
            assert_eq!(foo_count.load(Ordering::Relaxed), 1);
            assert_eq!(bar_count.load(Ordering::Relaxed), 1);

            foo.clone().enter(|| bar.clone().enter(|| {}));

            // The subscriber should see "bar" again, but the filter should not have
            // been called.
            assert_eq!(foo_count.load(Ordering::Relaxed), 1);
            assert_eq!(bar_count.load(Ordering::Relaxed), 1);

            bar.clone().enter(|| {});
            assert_eq!(foo_count.load(Ordering::Relaxed), 1);
            assert_eq!(bar_count.load(Ordering::Relaxed), 1);
        });
    }

    #[test]
    fn filters_are_reevaluated_when_changing_subscribers() {
        let foo_count = Arc::new(AtomicUsize::new(0));
        let bar_count = Arc::new(AtomicUsize::new(0));

        let foo_count1 = foo_count.clone();
        let bar_count1 = bar_count.clone();
        let subscriber1 = subscriber::mock()
            .enter(span::mock().named(Some("bar")))
            .exit(span::mock().named(Some("bar")))
            .enter(span::mock().named(Some("bar")))
            .exit(span::mock().named(Some("bar")))
            .enter(span::mock().named(Some("bar")))
            .exit(span::mock().named(Some("bar")))
            .enter(span::mock().named(Some("bar")))
            .exit(span::mock().named(Some("bar")))
            .enter(span::mock().named(Some("bar")))
            .exit(span::mock().named(Some("bar")))
            .enter(span::mock().named(Some("bar")))
            .exit(span::mock().named(Some("bar")))
            .with_filter(move |meta| match meta.name {
                Some("foo") => {
                    foo_count1.fetch_add(1, Ordering::Relaxed);
                    false
                }
                Some("bar") => {
                    bar_count1.fetch_add(1, Ordering::Relaxed);
                    true
                }
                _ => false,
            }).run();
        let subscriber1 = Dispatch::to(subscriber1);

        let foo_count2 = foo_count.clone();
        let bar_count2 = bar_count.clone();
        let subscriber2 = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .with_filter(move |meta| match meta.name {
                Some("foo") => {
                    foo_count2.fetch_add(1, Ordering::Relaxed);
                    true
                }
                Some("bar") => {
                    bar_count2.fetch_add(1, Ordering::Relaxed);
                    false
                }
                _ => false,
            }).run();
        let subscriber2 = Dispatch::to(subscriber2);

        let do_test = move |n: usize| {
            let foo = span!("foo");
            let bar = foo.clone().enter(|| {
                let bar = span!("bar");
                bar.clone().enter(|| bar)
            });

            assert_eq!(foo_count.load(Ordering::Relaxed), n);
            assert_eq!(bar_count.load(Ordering::Relaxed), n);

            foo.clone().enter(|| bar.clone().enter(|| {}));

            assert_eq!(foo_count.load(Ordering::Relaxed), n);
            assert_eq!(bar_count.load(Ordering::Relaxed), n);
        };

        subscriber1.with(|| {
            do_test(1);
        });

        subscriber2.with(|| do_test(2));

        subscriber1.with(|| do_test(3));

        subscriber2.with(|| do_test(4));
    }

    #[test]
    fn filters_evaluated_across_threads() {
        fn do_test() -> Span {
            let foo = span!("foo");
            let bar = foo.clone().enter(|| {
                let bar = span!("bar");
                bar.clone().enter(|| bar)
            });

            foo.enter(|| bar.clone().enter(|| {}));

            bar.clone()
        }

        let barrier = Arc::new(Barrier::new(2));

        let barrier1 = barrier.clone();
        let thread1 = thread::spawn(move || {
            let subscriber = subscriber::mock()
                .enter(span::mock().named(Some("bar")))
                .exit(span::mock().named(Some("bar")))
                .enter(span::mock().named(Some("bar")))
                .exit(span::mock().named(Some("bar")))
                .enter(span::mock().named(Some("bar")))
                .exit(span::mock().named(Some("bar")))
                .enter(span::mock().named(Some("bar")))
                .exit(span::mock().named(Some("bar")))
                .enter(span::mock().named(Some("bar")))
                .exit(span::mock().named(Some("bar")))
                .with_filter(|meta| match meta.name {
                    Some("bar") => true,
                    _ => false,
                }).run();
            // barrier1.wait();
            let subscriber = Dispatch::to(subscriber);
            subscriber.with(do_test);
            barrier1.wait();
            subscriber.with(do_test)
        });

        let thread2 = thread::spawn(move || {
            let subscriber = subscriber::mock()
                .enter(span::mock().named(Some("foo")))
                .exit(span::mock().named(Some("foo")))
                .enter(span::mock().named(Some("foo")))
                .exit(span::mock().named(Some("foo")))
                .enter(span::mock().named(Some("foo")))
                .exit(span::mock().named(Some("foo")))
                .enter(span::mock().named(Some("foo")))
                .exit(span::mock().named(Some("foo")))
                .enter(span::mock().named(Some("foo")))
                .exit(span::mock().named(Some("foo")))
                .with_filter(move |meta| match meta.name {
                    Some("foo") => true,
                    _ => false,
                }).run();
            let subscriber = Dispatch::to(subscriber);
            subscriber.with(do_test);
            barrier.wait();
            subscriber.with(do_test)
        });

        // the threads have completed, but the spans should still notify their
        // parent subscribers.

        let bar = thread1.join().unwrap();
        bar.enter(|| {});

        let bar = thread2.join().unwrap();
        bar.enter(|| {});
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
            .with_filter(move |meta| match meta.name {
                Some("foo") => {
                    foo_count2.fetch_add(1, Ordering::Relaxed);
                    false
                }
                Some("bar") => {
                    bar_count2.fetch_add(1, Ordering::Relaxed);
                    true
                }
                _ => false,
            }).run();

        Dispatch::to(subscriber).with(move || {
            // Enter "foo" and then "bar". The dispatcher expects to see "bar" but
            // not "foo."
            let foo = span!("foo");
            let bar = foo.clone().enter(|| {
                let bar = span!("bar");
                bar.clone().enter(|| bar)
            });

            // The filter should have seen each span a single time.
            assert_eq!(foo_count.load(Ordering::Relaxed), 1);
            assert_eq!(bar_count.load(Ordering::Relaxed), 1);

            foo.clone().enter(|| bar.clone().enter(|| {}));

            // The subscriber should see "bar" again, but the filter should not have
            // been called.
            assert_eq!(foo_count.load(Ordering::Relaxed), 1);
            assert_eq!(bar_count.load(Ordering::Relaxed), 1);

            // A different span with the same name has a different call site, so it
            // should cause the filter to be reapplied.
            let foo2 = span!("foo");
            foo.clone().enter(|| {});
            assert_eq!(foo_count.load(Ordering::Relaxed), 2);
            assert_eq!(bar_count.load(Ordering::Relaxed), 1);

            // But, the filter should not be re-evaluated for the new "foo" span
            // when it is re-entered.
            foo2.enter(|| span!("bar").enter(|| {}));
            assert_eq!(foo_count.load(Ordering::Relaxed), 2);
            assert_eq!(bar_count.load(Ordering::Relaxed), 2);
        });
    }

    #[test]
    fn filter_caching_is_lexically_scoped() {
        pub fn my_great_function() -> bool {
            span!("foo").enter(|| true)
        }

        pub fn my_other_function() -> bool {
            span!("bar").enter(|| true)
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
            .with_filter(move |_meta| {
                count2.fetch_add(1, Ordering::Relaxed);
                true
            }).run();

        Dispatch::to(subscriber).with(|| {
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
        });
    }
}
