use tracing_core::{dispatcher::*, metadata::Metadata, span, subscriber::Subscriber, Event};

#[cfg(feature = "std")]
#[test]
fn set_default_dispatch() {
    struct TestSubscriberA;
    impl Subscriber for TestSubscriberA {
        fn enabled(&self, _: &Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
            span::Id::from_u64(1)
        }
        fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
        fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
        fn event(&self, _: &Event<'_>) {}
        fn enter(&self, _: &span::Id) {}
        fn exit(&self, _: &span::Id) {}
    }
    #[cfg(feature = "std")]
    struct TestSubscriberB;

    #[cfg(feature = "std")]
    impl Subscriber for TestSubscriberB {
        fn enabled(&self, _: &Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
            span::Id::from_u64(1)
        }
        fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
        fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
        fn event(&self, _: &Event<'_>) {}
        fn enter(&self, _: &span::Id) {}
        fn exit(&self, _: &span::Id) {}
    }

    set_global_default(Dispatch::new(TestSubscriberA)).expect("global dispatch set failed");
    get_default(|current| {
        assert!(
            current.is::<TestSubscriberA>(),
            "global dispatch get failed"
        )
    });

    let guard = set_default(&Dispatch::new(TestSubscriberB));
    get_default(|current| assert!(current.is::<TestSubscriberB>(), "set_default get failed"));

    // Drop the guard, setting the dispatch back to the global dispatch
    drop(guard);

    get_default(|current| {
        assert!(
            current.is::<TestSubscriberA>(),
            "global dispatch get failed"
        )
    });
}

#[cfg(feature = "std")]
#[test]
fn nested_set_default() {
    struct TestSubscriberA;
    impl Subscriber for TestSubscriberA {
        fn enabled(&self, _: &Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
            span::Id::from_u64(1)
        }
        fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
        fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
        fn event(&self, _: &Event<'_>) {}
        fn enter(&self, _: &span::Id) {}
        fn exit(&self, _: &span::Id) {}
    }
    #[cfg(feature = "std")]
    struct TestSubscriberB;

    #[cfg(feature = "std")]
    impl Subscriber for TestSubscriberB {
        fn enabled(&self, _: &Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
            span::Id::from_u64(1)
        }
        fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
        fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
        fn event(&self, _: &Event<'_>) {}
        fn enter(&self, _: &span::Id) {}
        fn exit(&self, _: &span::Id) {}
    }

    let guard = set_default(&Dispatch::new(TestSubscriberA));
    get_default(|current| {
        assert!(
            current.is::<TestSubscriberA>(),
            "set_default for outer subscriber failed"
        )
    });

    let inner_guard = set_default(&Dispatch::new(TestSubscriberB));
    get_default(|current| {
        assert!(
            current.is::<TestSubscriberB>(),
            "set_default inner subscriber failed"
        )
    });

    drop(inner_guard);
    get_default(|current| {
        assert!(
            current.is::<TestSubscriberA>(),
            "set_default outer subscriber failed"
        )
    });
}
