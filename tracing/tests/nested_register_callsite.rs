#![cfg(feature = "std")]

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tracing::{
    metadata::Metadata,
    span,
    subscriber::{self, Interest, Subscriber},
    Event,
};

static NESTED_EVENTS: AtomicUsize = AtomicUsize::new(0);

fn nested() {
    tracing::info!(target: "nested", "hello world!");
}

#[test]
fn callsite_first_hit_inside_register_callsite_is_not_dead() {
    struct TestSubscriber {
        entered: AtomicBool,
    }

    impl Subscriber for TestSubscriber {
        fn register_callsite(&self, meta: &'static Metadata<'static>) -> Interest {
            if meta.target() == "outer" && !self.entered.swap(true, Ordering::SeqCst) {
                nested();
            }
            Interest::always()
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
            if event.metadata().target() == "nested" {
                NESTED_EVENTS.fetch_add(1, Ordering::SeqCst);
            }
        }
        fn enter(&self, _: &span::Id) {}
        fn exit(&self, _: &span::Id) {}
    }

    subscriber::with_default(
        TestSubscriber {
            entered: AtomicBool::new(false),
        },
        || {
            tracing::info!(target: "outer", "hello world!");
            NESTED_EVENTS.store(0, Ordering::SeqCst);
            nested();
        },
    );

    assert_eq!(NESTED_EVENTS.load(Ordering::SeqCst), 1);
}
