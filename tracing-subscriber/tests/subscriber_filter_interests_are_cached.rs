#![cfg(feature = "registry")]
mod support;
use self::support::*;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tracing::{Collect, Level};
use tracing_subscriber::{filter, prelude::*};

#[test]
fn subscriber_filter_interests_are_cached() {
    let seen = Arc::new(Mutex::new(HashMap::new()));
    let seen2 = seen.clone();
    let filter = filter::filter_fn(move |meta| {
        *seen
            .lock()
            .unwrap()
            .entry(meta.callsite())
            .or_insert(0usize) += 1;
        meta.level() == &Level::INFO
    });

    let (expect, handle) = subscriber::mock()
        .event(event::mock().at_level(Level::INFO))
        .event(event::mock().at_level(Level::INFO))
        .done()
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

    events();
    {
        let lock = seen2.lock().unwrap();
        for (callsite, &count) in lock.iter() {
            assert_eq!(
                count, 1,
                "callsite {:?} should have been seen 1 time (after first set of events)",
                callsite
            );
        }
    }

    events();
    {
        let lock = seen2.lock().unwrap();
        for (callsite, &count) in lock.iter() {
            assert_eq!(
                count, 1,
                "callsite {:?} should have been seen 1 time (after second set of events)",
                callsite
            );
        }
    }

    handle.assert_finished();
}
