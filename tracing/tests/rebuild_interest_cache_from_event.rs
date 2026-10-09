#![cfg(feature = "std")]

use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::{
    callsite,
    metadata::{LevelFilter, Metadata},
    span,
    subscriber::{self, Interest, Subscriber},
    Event,
};

static EVENTS: AtomicUsize = AtomicUsize::new(0);

#[test]
fn rebuild_interest_cache_from_event_keeps_max_level() {
    struct TestSubscriber;

    impl Subscriber for TestSubscriber {
        fn register_callsite(&self, _: &'static Metadata<'static>) -> Interest {
            Interest::always()
        }
        fn max_level_hint(&self) -> Option<LevelFilter> {
            Some(LevelFilter::INFO)
        }
        fn enabled(&self, _: &Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
            span::Id::from_u64(1)
        }
        fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
        fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
        fn event(&self, event: &Event<'_>) {
            if event.metadata().target() == "trigger" {
                callsite::rebuild_interest_cache();
            } else {
                EVENTS.fetch_add(1, Ordering::SeqCst);
            }
        }
        fn enter(&self, _: &span::Id) {}
        fn exit(&self, _: &span::Id) {}
    }

    subscriber::with_default(TestSubscriber, || {
        tracing::info!(target: "trigger", "rebuild");
        tracing::info!("hello world!");
    });

    assert_ne!(LevelFilter::current(), LevelFilter::OFF);
    assert_eq!(EVENTS.load(Ordering::SeqCst), 1);
}
