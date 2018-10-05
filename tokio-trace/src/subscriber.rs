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
    ///
    /// tokio_trace::Dispatch::to(filtered_subscriber).with(|| {
    ///     /// // This span will be logged.
    ///     span!("foo", enabled = true) .enter(|| {
    ///         // do work;
    ///     });
    ///     // This span will *not* be logged.
    ///     span!("bar", enabled = false).enter(|| {
    ///         // This event also will not be logged.
    ///         event!(Level::Debug, { enabled = false },"this won't be logged");
    ///     });
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
        collections::VecDeque,
        thread,
        sync::{Mutex, atomic::{AtomicUsize, Ordering}},
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
        expected: Mutex<VecDeque<Expect>>,
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

        pub fn run(self) -> impl Subscriber {
            Running {
                expected: Mutex::new(self.expected),
                ids: AtomicUsize::new(0),
            }
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
            match self.expected.lock().unwrap().pop_front() {
                None => {}
                Some(Expect::Event(_)) => unimplemented!(),
                Some(Expect::Enter(expected_span)) => panic!("expected to enter span {:?}, but got an event", expected_span.name),
                Some(Expect::Exit(expected_span)) => panic!("expected to exit span {:?} but got an event", expected_span.name),
            }
        }

        fn enter(&self, span: &SpanData) {
            // println!("+ {}: {:?}", thread::current().name().unwrap_or("unknown thread"), span);
            match self.expected.lock().unwrap().pop_front() {
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
            // println!("- {}: {:?}", thread::current().name().unwrap_or("unknown_thread"), span);
            match self.expected.lock().unwrap().pop_front() {
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
}

#[cfg(test)]
mod tests {
    use ::{
        span,
        subscriber::{self, Subscriber},
        Span,
        Dispatch,
    };
    use std::{
        thread,
        sync::{
            Arc,
            Barrier,
            atomic::{AtomicUsize, Ordering},
        },
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
            .run()
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

        Dispatch::to(subscriber).with(move || {
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
        });
    }


    #[test]
    fn filters_are_reevaluated_when_changing_subscribers() {
        let foo_count = Arc::new(AtomicUsize::new(0));
        let bar_count = Arc::new(AtomicUsize::new(0));

        let foo_count1 = foo_count.clone();
        let bar_count1 = bar_count.clone();
        let subscriber1 = Dispatch::to(subscriber::mock()
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
            .run()
            .with_filter(move |meta| match meta.name {
                Some("foo") => {
                    foo_count1.fetch_add(1, Ordering::Relaxed);
                    false
                },
                Some("bar") => {
                    bar_count1.fetch_add(1, Ordering::Relaxed);
                    true
                },
                _ => false,
            }));

        let foo_count2 = foo_count.clone();
        let bar_count2 = bar_count.clone();
        let subscriber2 = Dispatch::to(subscriber::mock()
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
            .run()
            .with_filter(move |meta| match meta.name {
                Some("foo") => {
                    foo_count2.fetch_add(1, Ordering::Relaxed);
                    true
                },
                Some("bar") => {
                    bar_count2.fetch_add(1, Ordering::Relaxed);
                    false
                },
                _ => false,
            }));


        let do_test = move |n: usize| {
            let foo = span!("foo");
            let bar = foo.clone().enter(|| {
                let bar = span!("bar");
                bar.clone().enter(|| { bar })
            });

            assert_eq!(foo_count.load(Ordering::Relaxed), n);
            assert_eq!(bar_count.load(Ordering::Relaxed), n);

            foo.clone().enter(|| {
                bar.clone().enter(|| { })
            });

            assert_eq!(foo_count.load(Ordering::Relaxed), n);
            assert_eq!(bar_count.load(Ordering::Relaxed), n);
        };

        subscriber1.with(|| {
            do_test(1);
        });

        subscriber2.with(|| {
            do_test(2)
        });

        subscriber1.with(|| {
            do_test(3)
        });

        subscriber2.with(|| {
            do_test(4)
        });
    }

    #[test]
    fn filters_evaluated_across_threads() {
        fn do_test() -> Span {
            let foo = span!("foo");
            let bar = foo.clone().enter(|| {
                let bar = span!("bar");
                bar.clone().enter(|| { bar })
            });

            foo.enter(|| {
                bar.clone().enter(|| { })
            });

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
                .run()
                .with_filter(|meta| match meta.name {
                    Some("bar") => true,
                    _ => false,
                });
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
                .run()
                .with_filter(move |meta| match meta.name {
                    Some("foo") => true,
                    _ => false,
                });
            let subscriber = Dispatch::to(subscriber);
            subscriber.with(do_test);
            barrier.wait();
            subscriber.with(do_test)
        });

        // the threads have completed, but the spans should still notify their
        // parent subscribers.

        let bar = thread1.join().unwrap();
        bar.enter(|| { });

        let bar = thread2.join().unwrap();
        bar.enter(|| { });
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
            .run()
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

        Dispatch::to(subscriber).with(move || {
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
        });
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
            .run()
            .with_filter(move |_meta| {
                count2.fetch_add(1, Ordering::Relaxed);
                true
            });

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
