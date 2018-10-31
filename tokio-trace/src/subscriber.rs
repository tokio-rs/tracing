pub use tokio_trace_core::subscriber::*;

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

        Dispatch::to(subscriber).as_default(move || {
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

        subscriber1.as_default(|| {
            do_test(1);
        });

        subscriber2.as_default(|| do_test(2));

        subscriber1.as_default(|| do_test(3));

        subscriber2.as_default(|| do_test(4));
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
            subscriber.as_default(do_test);
            barrier1.wait();
            subscriber.as_default(do_test)
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
            subscriber.as_default(do_test);
            barrier.wait();
            subscriber.as_default(do_test)
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

        Dispatch::to(subscriber).as_default(move || {
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

        Dispatch::to(subscriber).as_default(|| {
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
