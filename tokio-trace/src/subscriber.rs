pub use tokio_trace_core::subscriber::*;

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use {span, subscriber, Dispatch};

    #[test]
    fn filters_are_not_reevaluated_for_the_same_span() {
        // Asserts that the `span!` macro caches the result of calling
        // `Subscriber::enabled` for each span.
        let alice_count = Arc::new(AtomicUsize::new(0));
        let bob_count = Arc::new(AtomicUsize::new(0));
        let alice_count2 = alice_count.clone();
        let bob_count2 = bob_count.clone();

        let subscriber = subscriber::mock()
            .with_filter(move |meta| match meta.name {
                Some("alice") => {
                    alice_count2.fetch_add(1, Ordering::Relaxed);
                    false
                }
                Some("bob") => {
                    bob_count2.fetch_add(1, Ordering::Relaxed);
                    true
                }
                _ => false,
            }).run();

        Dispatch::to(subscriber).as_default(move || {
            // Enter "alice" and then "bob". The dispatcher expects to see "bob" but
            // not "alice."
            let mut alice = span!("alice");
            let mut bob = alice.enter(|| {
                let mut bob = span!("bob");
                bob.enter(|| ());
                bob
            });

            // The filter should have seen each span a single time.
            assert_eq!(alice_count.load(Ordering::Relaxed), 1);
            assert_eq!(bob_count.load(Ordering::Relaxed), 1);

            alice.enter(|| bob.enter(|| {}));

            // The subscriber should see "bob" again, but the filter should not have
            // been called.
            assert_eq!(alice_count.load(Ordering::Relaxed), 1);
            assert_eq!(bob_count.load(Ordering::Relaxed), 1);

            bob.enter(|| {});
            assert_eq!(alice_count.load(Ordering::Relaxed), 1);
            assert_eq!(bob_count.load(Ordering::Relaxed), 1);
        });
    }

    // This is no longer testing the expected behaviour.
    /*
    fn filters_evaluated_across_threads() {
        ::callsite::reset_registry();
        fn do_test() -> Span {
            let mut foo = span!("foo");
            let mut bar = foo.enter(|| {
                let mut bar = span!("bar");
                bar.enter(|| ());
                bar
            });

            foo.enter(|| bar.enter(|| {}));

            bar
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

        let mut bar = thread1.join().unwrap();
        bar.enter(|| {});

        let mut bar = thread2.join().unwrap();
        bar.enter(|| {});
    }
    */

    #[test]
    fn filters_are_reevaluated_for_different_call_sites() {
        // Asserts that the `span!` macro caches the result of calling
        // `Subscriber::enabled` for each span.
        let charlie_count = Arc::new(AtomicUsize::new(0));
        let dave_count = Arc::new(AtomicUsize::new(0));
        let charlie_count2 = charlie_count.clone();
        let dave_count2 = dave_count.clone();

        let subscriber = subscriber::mock()
            .with_filter(move |meta| {
                println!("Filter: {:?}", meta.name);
                match meta.name {
                    Some("charlie") => {
                        charlie_count2.fetch_add(1, Ordering::Relaxed);
                        false
                    }
                    Some("dave") => {
                        dave_count2.fetch_add(1, Ordering::Relaxed);
                        true
                    }
                    _ => false,
                }
            }).run();

        Dispatch::to(subscriber).as_default(move || {
            // Enter "charlie" and then "dave". The dispatcher expects to see "dave" but
            // not "charlie."
            let mut charlie = span!("charlie");
            let mut dave = charlie.enter(|| {
                let mut dave = span!("dave");
                dave.enter(|| {});
                dave
            });

            // The filter should have seen each span a single time.
            assert_eq!(charlie_count.load(Ordering::Relaxed), 1);
            assert_eq!(dave_count.load(Ordering::Relaxed), 1);

            charlie.enter(|| dave.enter(|| {}));

            // The subscriber should see "dave" again, but the filter should not have
            // been called.
            assert_eq!(charlie_count.load(Ordering::Relaxed), 1);
            assert_eq!(dave_count.load(Ordering::Relaxed), 1);

            // A different span with the same name has a different call site, so it
            // should cause the filter to be reapplied.
            let mut charlie2 = span!("charlie");
            charlie.enter(|| {});
            assert_eq!(charlie_count.load(Ordering::Relaxed), 2);
            assert_eq!(dave_count.load(Ordering::Relaxed), 1);

            // But, the filter should not be re-evaluated for the new "charlie" span
            // when it is re-entered.
            charlie2.enter(|| span!("dave").enter(|| {}));
            assert_eq!(charlie_count.load(Ordering::Relaxed), 2);
            assert_eq!(dave_count.load(Ordering::Relaxed), 2);
        });
    }

    #[test]
    fn filter_caching_is_lexically_scoped() {
        pub fn my_great_function() -> bool {
            span!("emily").enter(|| true)
        }

        pub fn my_other_function() -> bool {
            span!("frank").enter(|| true)
        }

        let count = Arc::new(AtomicUsize::new(0));
        let count2 = count.clone();

        let subscriber = subscriber::mock()
            .enter(span::mock().named(Some("emily")))
            .exit(span::mock().named(Some("emily")))
            .enter(span::mock().named(Some("emily")))
            .exit(span::mock().named(Some("emily")))
            .enter(span::mock().named(Some("frank")))
            .exit(span::mock().named(Some("frank")))
            .enter(span::mock().named(Some("emily")))
            .exit(span::mock().named(Some("emily")))
            .enter(span::mock().named(Some("frank")))
            .exit(span::mock().named(Some("frank")))
            .enter(span::mock().named(Some("emily")))
            .exit(span::mock().named(Some("emily")))
            .with_filter(move |meta| match meta.name {
                Some("emily") | Some("frank") => {
                    count2.fetch_add(1, Ordering::Relaxed);
                    true
                }
                _ => false,
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
