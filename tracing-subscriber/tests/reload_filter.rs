#![cfg(feature = "registry")]
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing_core::{collect::Interest, Collect, LevelFilter, Metadata};
use tracing_subscriber::{prelude::*, reload::*, subscribe};

fn event() {
    tracing::info!("my event");
}

pub struct NopSubscriber;
impl<S: Collect> tracing_subscriber::Subscribe<S> for NopSubscriber {
    fn register_callsite(&self, _m: &Metadata<'_>) -> Interest {
        Interest::sometimes()
    }

    fn enabled(&self, _m: &Metadata<'_>, _: subscribe::Context<'_, S>) -> bool {
        true
    }
}

/// Tests the behavior of reload Subscriber's Filter implementation`.
///
/// This test asserts on the global value from `LevelFilter::current()`. As
/// such, it must be in a test module by itself. No other tests can be included
/// in this file.
#[test]
fn reload_filter() {
    static FILTER1_CALLS: AtomicUsize = AtomicUsize::new(0);
    static FILTER2_CALLS: AtomicUsize = AtomicUsize::new(0);

    enum Filter {
        One,
        Two,
    }

    impl<S: Collect> tracing_subscriber::subscribe::Filter<S> for Filter {
        fn enabled(&self, m: &Metadata<'_>, _: &subscribe::Context<'_, S>) -> bool {
            println!("ENABLED: {:?}", m);
            match self {
                Filter::One => FILTER1_CALLS.fetch_add(1, Ordering::SeqCst),
                Filter::Two => FILTER2_CALLS.fetch_add(1, Ordering::SeqCst),
            };
            true
        }

        fn max_level_hint(&self) -> Option<LevelFilter> {
            match self {
                Filter::One => Some(LevelFilter::INFO),
                Filter::Two => Some(LevelFilter::DEBUG),
            }
        }
    }

    let (filter, handle) = Subscriber::new(Filter::One);

    let dispatcher = tracing_core::dispatch::Dispatch::new(
        tracing_subscriber::registry().with(NopSubscriber.with_filter(filter)),
    );

    tracing_core::dispatch::with_default(&dispatcher, || {
        assert_eq!(FILTER1_CALLS.load(Ordering::SeqCst), 0);
        assert_eq!(FILTER2_CALLS.load(Ordering::SeqCst), 0);

        event();

        assert_eq!(FILTER1_CALLS.load(Ordering::SeqCst), 1);
        assert_eq!(FILTER2_CALLS.load(Ordering::SeqCst), 0);

        assert_eq!(LevelFilter::current(), LevelFilter::INFO);
        handle.reload(Filter::Two).expect("should reload");
        assert_eq!(LevelFilter::current(), LevelFilter::DEBUG);

        event();

        assert_eq!(FILTER1_CALLS.load(Ordering::SeqCst), 1);
        assert_eq!(FILTER2_CALLS.load(Ordering::SeqCst), 1);
    })
}
