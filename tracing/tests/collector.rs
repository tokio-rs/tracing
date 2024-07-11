// These tests require the thread-local scoped dispatcher, which only works when
// we have a standard library. The behaviour being tested should be the same
// with the standard lib disabled.
//
// The alternative would be for each of these tests to be defined in a separate
// file, which is :(
#![cfg(feature = "std")]
use tracing::{
    collect::{with_default, Collect, Interest},
    field::display,
    span::{Attributes, Id, Record},
    Event, Level, Metadata,
};
use tracing_mock::{collector, expect};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn event_macros_dont_infinite_loop() {
    // This test ensures that an event macro within a collector
    // won't cause an infinite loop of events.
    struct TestCollector;
    impl Collect for TestCollector {
        fn register_callsite(&self, _: &Metadata<'_>) -> Interest {
            // Always return sometimes so that `enabled` will be called
            // (which can loop).
            Interest::sometimes()
        }

        fn enabled(&self, meta: &Metadata<'_>) -> bool {
            assert!(meta.fields().iter().any(|f| f.name() == "foo"));
            tracing::event!(Level::TRACE, bar = false);
            true
        }

        fn new_span(&self, _: &Attributes<'_>) -> Id {
            Id::from_u64(0xAAAA)
        }

        fn record(&self, _: &Id, _: &Record<'_>) {}

        fn record_follows_from(&self, _: &Id, _: &Id) {}

        fn event(&self, event: &Event<'_>) {
            assert!(event.metadata().fields().iter().any(|f| f.name() == "foo"));
            tracing::event!(Level::TRACE, baz = false);
        }

        fn enter(&self, _: &Id) {}

        fn exit(&self, _: &Id) {}

        fn current_span(&self) -> tracing_core::span::Current {
            tracing_core::span::Current::unknown()
        }
    }

    with_default(TestCollector, || {
        tracing::event!(Level::TRACE, foo = false);
    })
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn boxed_collector() {
    let (collector, handle) = collector::mock()
        .new_span(
            expect::span().named("foo").with_fields(
                expect::field("bar")
                    .with_value(&display("hello from my span"))
                    .only(),
            ),
        )
        .enter(expect::span().named("foo"))
        .exit(expect::span().named("foo"))
        .drop_span(expect::span().named("foo"))
        .only()
        .run_with_handle();
    let collector: Box<dyn Collect + Send + Sync + 'static> = Box::new(collector);

    with_default(collector, || {
        let from = "my span";
        let span = tracing::span!(
            Level::TRACE,
            "foo",
            bar = format_args!("hello from {}", from)
        );
        span.in_scope(|| {});
    });

    handle.assert_finished();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn arced_collector() {
    use std::sync::Arc;

    let (collector, handle) = collector::mock()
        .new_span(
            expect::span().named("foo").with_fields(
                expect::field("bar")
                    .with_value(&display("hello from my span"))
                    .only(),
            ),
        )
        .enter(expect::span().named("foo"))
        .exit(expect::span().named("foo"))
        .drop_span(expect::span().named("foo"))
        .event(
            expect::event()
                .with_fields(expect::field("message").with_value(&display("hello from my event"))),
        )
        .only()
        .run_with_handle();
    let collector: Arc<dyn Collect + Send + Sync + 'static> = Arc::new(collector);

    // Test using a clone of the `Arc`ed collector
    with_default(collector.clone(), || {
        let from = "my span";
        let span = tracing::span!(
            Level::TRACE,
            "foo",
            bar = format_args!("hello from {}", from)
        );
        span.in_scope(|| {});
    });

    with_default(collector, || {
        tracing::info!("hello from my event");
    });

    handle.assert_finished();
}
