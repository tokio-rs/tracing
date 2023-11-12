#![cfg(feature = "registry")]
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing_core::{
    collect::Interest,
    span::{Attributes, Id, Record},
    Collect, Event, LevelFilter, Metadata,
};
use tracing_subscriber::{prelude::*, reload::*, subscribe};

fn event() {
    tracing::info!("my event");
}

pub struct NopCollector;

impl Collect for NopCollector {
    fn register_callsite(&self, _: &'static Metadata<'static>) -> Interest {
        Interest::never()
    }

    fn enabled(&self, _: &Metadata<'_>) -> bool {
        false
    }

    fn new_span(&self, _: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}
    fn record_follows_from(&self, _: &Id, _: &Id) {}
    fn event(&self, _: &Event<'_>) {}
    fn enter(&self, _: &Id) {}
    fn exit(&self, _: &Id) {}
    fn current_span(&self) -> tracing_core::span::Current {
        tracing_core::span::Current::unknown()
    }
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

/// Running these two tests in parallel will cause flaky failures, since they are both modifying the MAX_LEVEL value.
/// "cargo test -- --test-threads=1 fixes it, but it runs all tests in serial.
/// The only way to run tests in serial in a single file is this way.
#[test]
fn run_all_reload_test() {
    reload_handle();
    reload_filter();
}

fn reload_handle() {
    static FILTER1_CALLS: AtomicUsize = AtomicUsize::new(0);
    static FILTER2_CALLS: AtomicUsize = AtomicUsize::new(0);

    enum Filter {
        One,
        Two,
    }

    impl<S: Collect> tracing_subscriber::Subscribe<S> for Filter {
        fn register_callsite(&self, m: &Metadata<'_>) -> Interest {
            println!("REGISTER: {:?}", m);
            Interest::sometimes()
        }

        fn enabled(&self, m: &Metadata<'_>, _: subscribe::Context<'_, S>) -> bool {
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

    let (subscriber, handle) = Subscriber::new(Filter::One);

    let dispatcher = tracing_core::dispatch::Dispatch::new(subscriber.with_collector(NopCollector));

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
