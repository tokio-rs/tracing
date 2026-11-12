#![cfg(feature = "registry")]
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tracing::{Level, Subscriber};
use tracing_mock::{expect, layer};
use tracing_subscriber::{filter, prelude::*};

#[test]
fn layer_filter_interests_are_cached() {
    let seen = Arc::new(Mutex::new(HashMap::new()));
    let seen2 = seen.clone();
    let filter = filter::filter_fn(move |meta| {
        *seen.lock().unwrap().entry(*meta.level()).or_insert(0usize) += 1;
        meta.level() == &Level::INFO
    });

    let (expect, handle) = layer::mock()
        .event(expect::event().at_level(Level::INFO))
        .event(expect::event().at_level(Level::INFO))
        .only()
        .run_with_handle();

    let subscriber = tracing_subscriber::registry().with(expect.with_filter(filter));
    assert!(subscriber.max_level_hint().is_none());

    let _subscriber = subscriber.set_default();

    fn events() {
        tracing::trace!("hello trace");
        tracing::debug!("hello debug");
        tracing::info!("hello info");
        tracing::warn!("hello warn");
        tracing::error!("hello error");
    }

    // Per-layer filters cap callsite interest at `sometimes` (#2519, #3516),
    // so a callsite the filter accepts is re-evaluated on each dispatch.
    // Callsites the filter rejects still cache `Interest::never` and are
    // seen exactly once (during `register_callsite`).
    let assert_seen = |dispatches: usize| {
        let seen = seen2.lock().unwrap();
        for (&level, &count) in seen.iter() {
            let expected = if level == Level::INFO {
                1 + dispatches
            } else {
                1
            };
            assert_eq!(
                count, expected,
                "the {level:?} callsite should have been seen {expected} times \
                 after {dispatches} dispatches",
            );
        }
    };

    events();
    assert_seen(1);

    events();
    assert_seen(2);

    handle.assert_finished();
}
